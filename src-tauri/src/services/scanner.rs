use crate::db::DbState;
use crate::services::library::{LibraryService, PreparedArtwork, PreparedFile};
use crate::services::metadata::{extract_metadata, AudioMetadata};
use crate::services::webdav::{HttpRangeReader, WebdavClient};
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info, warn};
use walkdir::WalkDir;

/// 判定一个扩展名是否为受支持的音频格式（与 library.rs 中保持一致）
fn is_supported_audio_ext(ext: &str) -> bool {
    matches!(ext, "mp3" | "flac" | "wav" | "m4a" | "aac")
}

#[derive(Clone, Serialize)]
pub struct ScanProgressPayload {
    pub source_id: i64,
    pub scanned_count: usize,
    pub skipped_count: usize,
    pub current_path: String,
}

/// worker 从提取通道拿到的结果：成功为预处理完成的文件，失败为待标记的损坏文件
enum ExtractOutcome {
    // Boxed：PreparedFile 体积远大于错误分支，不装箱会让整个枚举按最大变体布局
    Ok(Box<PreparedFile>),
    Err { path: PathBuf, error: String },
}

/// 写库线程结束后的统计（供最终日志与前端进度使用）
#[derive(Default)]
struct ScanTotals {
    scanned: usize,
    errors: usize,
}

/// 单个文件的提取 + 预处理（封面哈希/写盘/缩略图 + lrc 预读）。
/// 本地扫描在 worker 线程并行执行；WebDAV 扫描在主扫描线程串行调用。
/// 成功后会清空 metadata 内的封面字节，保持通道/批次中的数据轻量。
fn prepare_file(
    mut metadata: AudioMetadata,
    path: PathBuf,
    mtime: i64,
    size: i64,
    app_data_dir: &Path,
    read_lrc: bool,
    seen_hashes: &Mutex<HashSet<String>>,
) -> PreparedFile {
    let artwork = prepare_artwork(&mut metadata, app_data_dir, seen_hashes);
    let lrc_content = if read_lrc { read_lrc_file(&path) } else { None };
    PreparedFile {
        path,
        metadata,
        mtime,
        size,
        artwork,
        lrc_content,
    }
}

/// 封面预处理：SHA256 + （本扫描内首次遇到该 hash 时）写原图缓存与生成缩略图。
/// 同 hash 只有第一个文件做磁盘工作，后续文件只带 hash 去写库侧复用。
fn prepare_artwork(
    metadata: &mut AudioMetadata,
    app_data_dir: &Path,
    seen_hashes: &Mutex<HashSet<String>>,
) -> PreparedArtwork {
    let Some(data) = metadata.picture_data.take() else {
        return PreparedArtwork::default();
    };
    if data.is_empty() {
        return PreparedArtwork::default();
    }
    let mime = metadata.picture_mime.clone();

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());

    let is_first = seen_hashes.lock().unwrap().insert(hash.clone());
    if !is_first {
        return PreparedArtwork {
            hash: Some(hash),
            mime,
            cache_path: None,
            thumbnail: None,
        };
    }

    let cache_path = LibraryService::artwork_cache_path(app_data_dir, mime.as_deref(), &hash);
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !cache_path.exists() {
        let _ = std::fs::write(&cache_path, &data);
    }
    let thumbnail = LibraryService::generate_thumbnail(&data);
    PreparedArtwork {
        hash: Some(hash),
        mime,
        cache_path: Some(cache_path),
        thumbnail,
    }
}

/// 预读同目录同名 .lrc 歌词（优先小写扩展名，其次大写），与旧扫描逻辑一致
fn read_lrc_file(path: &Path) -> Option<String> {
    let mut lrc_path = path.with_extension("lrc");
    if !lrc_path.exists() {
        lrc_path = path.with_extension("LRC");
    }
    if lrc_path.exists() && lrc_path.is_file() {
        return std::fs::read_to_string(&lrc_path).ok();
    }
    None
}

/// 提取 worker：从 paths 通道取任务，解析元数据并预处理，结果送回写库线程。
/// 写库线程退出（通道对端关闭）后自动收尾。
fn extract_worker(
    paths_rx: Arc<Mutex<mpsc::Receiver<(PathBuf, i64, i64)>>>,
    results_tx: mpsc::Sender<ExtractOutcome>,
    app_data_dir: PathBuf,
    seen_hashes: Arc<Mutex<HashSet<String>>>,
) {
    loop {
        let job = paths_rx.lock().unwrap().recv();
        let Ok((path, mtime, size)) = job else { break };

        match extract_metadata(&path) {
            Ok(metadata) => {
                let prepared = prepare_file(
                    metadata,
                    path,
                    mtime,
                    size,
                    &app_data_dir,
                    true,
                    &seen_hashes,
                );
                if results_tx
                    .send(ExtractOutcome::Ok(Box::new(prepared)))
                    .is_err()
                {
                    break;
                }
            }
            Err(e) => {
                if results_tx
                    .send(ExtractOutcome::Err { path, error: e })
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}

/// 写库线程：唯一写者。独占一条池连接（不被前端 artwork 请求饿死），
/// 攒 50 条一个事务提交；提交后主动收缩 WAL，长扫描期间不再无界增长。
fn db_writer_loop(
    app: AppHandle,
    source_id: i64,
    source_root: PathBuf,
    app_data_dir: PathBuf,
    results_rx: mpsc::Receiver<ExtractOutcome>,
    skipped: Arc<AtomicUsize>,
    scanned_shared: Arc<AtomicUsize>,
) -> ScanTotals {
    let mut totals = ScanTotals::default();
    let Some(db_state) = app.try_state::<DbState>() else {
        return totals;
    };
    let Ok(mut conn) = db_state.db.get() else {
        error!("Scan writer: 无法获取数据库连接，扫描写入中止");
        return totals;
    };

    // 扫描期写连接调优：加大页缓存（64MB）减少曲库增长后的索引页读盘，临时表走内存
    let _ = conn.pragma_update(None, "cache_size", -65536i64);
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");

    let mut batch: Vec<PreparedFile> = Vec::with_capacity(50);
    while let Ok(outcome) = results_rx.recv() {
        match outcome {
            ExtractOutcome::Ok(prepared) => {
                totals.scanned += 1;
                scanned_shared.store(totals.scanned, Ordering::Relaxed);
                if totals.scanned % 5 == 0 || totals.scanned == 1 {
                    let _ = app.emit(
                        "scan-progress",
                        ScanProgressPayload {
                            source_id,
                            scanned_count: totals.scanned,
                            skipped_count: skipped.load(Ordering::Relaxed),
                            current_path: prepared.path.to_string_lossy().to_string(),
                        },
                    );
                }
                batch.push(*prepared);
                if batch.len() >= 50 {
                    flush_batch(
                        &mut conn,
                        source_id,
                        &source_root,
                        &mut batch,
                        &app_data_dir,
                    );
                }
            }
            ExtractOutcome::Err { path, error } => {
                totals.errors += 1;
                warn!("Skipped file (metadata error): {:?} - {}", path, error);
                mark_scan_error(&conn, source_id, &source_root, &path, &error);
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(
            &mut conn,
            source_id,
            &source_root,
            &mut batch,
            &app_data_dir,
        );
    }
    totals
}

/// 批量提交：一个事务内把整批 PreparedFile 写入（事务内已无任何文件系统 I/O），
/// 提交后主动做一次 PASSIVE checkpoint 收缩 WAL。
fn flush_batch(
    conn: &mut Connection,
    source_id: i64,
    source_root: &Path,
    batch: &mut Vec<PreparedFile>,
    app_data_dir: &Path,
) {
    let Ok(tx) = conn.transaction() else { return };
    for prepared in batch.iter() {
        if let Err(e) =
            LibraryService::index_file(&tx, source_id, source_root, prepared, app_data_dir)
        {
            error!("Failed to index file {:?}: {}", prepared.path, e);
        }
    }
    let _ = tx.commit();
    // PASSIVE：有读者时跳过、无读者时增量收缩，绝不阻塞
    let _ = conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |_row| Ok(()));
    batch.clear();
}

/// 执行本地目录扫描（并行流水线版）：
///   生产者（本线程）WalkDir + 增量跳过判定 → 路径通道
///   → K 个提取 worker（lofty 解析 + 封面哈希/写盘/缩略图 + lrc 预读）
///   → 结果通道 → 唯一写库线程（50 条/事务 + WAL checkpoint）
/// 损坏或无法解析的文件会被跳过并计数（不再以 Unknown 形式污染曲库）。
pub fn scan_local_directory(app: AppHandle, source_id: i64, path: &Path, app_data_dir: &Path) {
    info!("Starting parallel scan for directory: {:?}", path);
    let mut skipped_count = 0usize;
    let mut scan_failed = false;

    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            finish_scan(
                &app,
                source_id,
                ScanOutcome::failed("root_not_a_directory", "扫描根路径不是目录"),
            );
            return;
        }
        Err(e) => {
            finish_scan(
                &app,
                source_id,
                ScanOutcome::failed("root_unreachable", format!("无法访问扫描根路径: {}", e)),
            );
            return;
        }
    }

    // Load existing files for incremental scan
    let mut file_cache: HashMap<String, (i64, i64, String)> = HashMap::new();
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(conn) = db_state.db.get() {
            if let Ok(mut stmt) = conn.prepare("SELECT normalized_path, modified_at, file_size, availability FROM media_files WHERE source_id = ?1") {
                if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| {
                    let mtime_str: Option<String> = row.get(1)?;
                    let mtime = mtime_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    let size: Option<i64> = row.get(2)?;
                    Ok((row.get::<_, String>(0)?, mtime, size.unwrap_or(0), row.get::<_, String>(3)?))
                }) {
                    for r in rows.filter_map(Result::ok) {
                        file_cache.insert(r.0, (r.1, r.2, r.3));
                    }
                }
            }
        }
    }
    info!(
        "Loaded {} existing files for incremental scan check.",
        file_cache.len()
    );

    let mut scanned_paths = HashSet::new();
    let skipped_shared = Arc::new(AtomicUsize::new(0));
    let scanned_shared = Arc::new(AtomicUsize::new(0));
    let seen_hashes = Arc::new(Mutex::new(HashSet::new()));

    // 提取 worker 数：吃满并行度但封顶 4（机械盘上过多并发读反而互相寻道）
    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(4);

    let (paths_tx, paths_rx) = mpsc::channel::<(PathBuf, i64, i64)>();
    let (results_tx, results_rx) = mpsc::channel::<ExtractOutcome>();

    // 写库线程（唯一写者，独占连接）
    let writer_app = app.clone();
    let writer_root = path.to_path_buf();
    let writer_data_dir = app_data_dir.to_path_buf();
    let writer_skipped = Arc::clone(&skipped_shared);
    let writer_scanned = Arc::clone(&scanned_shared);
    let writer = thread::spawn(move || {
        db_writer_loop(
            writer_app,
            source_id,
            writer_root,
            writer_data_dir,
            results_rx,
            writer_skipped,
            writer_scanned,
        )
    });

    // 提取 worker 池
    let paths_rx_shared = Arc::new(Mutex::new(paths_rx));
    for _ in 0..worker_count {
        let rx = Arc::clone(&paths_rx_shared);
        let tx = results_tx.clone();
        let dir = app_data_dir.to_path_buf();
        let seen = Arc::clone(&seen_hashes);
        thread::spawn(move || extract_worker(rx, tx, dir, seen));
    }
    drop(results_tx);

    // 生产者：遍历目录、增量判定，把需要解析的文件送入提取通道
    for entry_result in WalkDir::new(path).into_iter() {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                scan_failed = true;
                warn!("Failed to read local scan entry: {}", e);
                continue;
            }
        };
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !is_supported_audio_ext(&ext.to_lowercase()) {
            continue;
        }

        // 1. 获取文件系统元数据 (mtime, size)
        let fs_metadata = std::fs::metadata(entry_path).ok();
        let fs_size = fs_metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
        let fs_mtime = fs_metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // 构建 normalized_path
        let relative_path = entry_path
            .strip_prefix(path)
            .unwrap_or(entry_path)
            .to_string_lossy()
            .to_string();
        let normalized_path = relative_path.to_lowercase();

        // 记录已扫描的文件路径，用于后续的删除检测
        scanned_paths.insert(normalized_path.clone());

        // 增量判定：如果数据库中已经有且大小和修改时间均一致，则跳过解析
        if let Some(&(db_mtime, db_size, ref availability)) = file_cache.get(&normalized_path) {
            if db_mtime == fs_mtime && db_size == fs_size && availability == "available" {
                skipped_count += 1;
                skipped_shared.store(skipped_count, Ordering::Relaxed);

                // 每 50 个 skipped 也发一次进度，避免长久卡顿感
                if skipped_count.is_multiple_of(50) {
                    let _ = app.emit(
                        "scan-progress",
                        ScanProgressPayload {
                            source_id,
                            scanned_count: scanned_shared.load(Ordering::Relaxed),
                            skipped_count,
                            current_path: entry_path.to_string_lossy().to_string(),
                        },
                    );
                }
                continue;
            }
        }

        // 2. 送入提取流水线（解析失败在 worker 内部处理并回传）
        if paths_tx
            .send((entry_path.to_path_buf(), fs_mtime, fs_size))
            .is_err()
        {
            // 写库线程已退出（数据库不可用），终止遍历
            scan_failed = true;
            break;
        }
    }
    drop(paths_tx);

    // 等写库线程清空结果通道并提交完最后一批
    let totals = match writer.join() {
        Ok(t) => t,
        Err(e) => {
            error!("Scan writer panicked: {:?}", e);
            ScanTotals::default()
        }
    };

    // Clean up missing files (deleted from disk)
    let mut missing_count = 0;
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(mut conn) = db_state.db.get() {
            // A partial/inaccessible walk must never turn unseen files into "missing".
            if scan_failed {
                warn!("Skipping missing-file cleanup because the local scan was incomplete");
            }

            // Find files in db that are not in scanned_paths
            let mut to_delete = Vec::new();
            if !scan_failed {
                for (db_path, _) in file_cache.iter() {
                    if !scanned_paths.contains(db_path) {
                        to_delete.push(db_path.clone());
                    }
                }
            }

            missing_count = to_delete.len();
            if missing_count > 0 {
                info!(
                    "Found {} missing files, marking them as missing...",
                    missing_count
                );
                if let Ok(tx) = conn.transaction() {
                    for missing_path in to_delete {
                        let _ = tx.execute(
                            "UPDATE media_files SET availability = 'missing' WHERE source_id = ?1 AND normalized_path = ?2",
                            rusqlite::params![source_id, missing_path]
                        );
                    }
                    let _ = tx.commit();
                }
            }

            // Also update last_seen_at for all scanned paths to keep them 'available'
            let _ = conn.execute(
                "UPDATE media_files SET last_seen_at = datetime('now') WHERE source_id = ?1 AND availability = 'available'",
                rusqlite::params![source_id]
            );
        }
    }

    info!(
        "Scan completed for directory: {:?} (scanned={}, errors={}, skipped={}, missing={})",
        path, totals.scanned, totals.errors, skipped_count, missing_count
    );
    finish_scan(
        &app,
        source_id,
        if scan_failed {
            ScanOutcome::failed("scan_incomplete", "扫描未完成，已跳过缺失文件清理")
        } else {
            ScanOutcome::Success
        },
    );
}

/// 执行 WebDAV 远程目录扫描（串行；预处理与本地 worker 相同，但进度事件按 20 个节流）。
/// 使用 HTTP HEAD/GET 探测文件列表，并尝试部分读取元数据。
pub fn scan_webdav_directory(
    app: AppHandle,
    source_id: i64,
    root_uri: String,
    username: Option<String>,
    password: Option<String>,
    app_data_dir: &Path,
) {
    info!("Starting async scan for WebDAV: {}", root_uri);
    let mut scanned_count = 0usize;
    let mut skipped_count = 0usize;
    let mut scan_failed = false;

    let Some(db_state) = app.try_state::<DbState>() else {
        // 这两条早退此前只 return、不发事件：前端会永远停在「扫描中」（CR-006）
        finish_scan(
            &app,
            source_id,
            ScanOutcome::failed(
                "db_unavailable",
                "扫描无法开始：本地数据库不可用，请重启应用后重试",
            ),
        );
        return;
    };
    // 独占一条连接贯穿整个扫描（文件缓存加载 / 预处理错误标记 / 批量写库 / 收尾）
    let Ok(mut scan_conn) = db_state.db.get() else {
        finish_scan(
            &app,
            source_id,
            ScanOutcome::failed("db_unavailable", "扫描无法开始：本地数据库正忙，请稍后重试"),
        );
        return;
    };
    let _ = scan_conn.pragma_update(None, "cache_size", -65536i64);
    let _ = scan_conn.pragma_update(None, "temp_store", "MEMORY");

    let mut file_cache: HashMap<String, (i64, i64, String)> = HashMap::new();
    if let Ok(mut stmt) = scan_conn.prepare("SELECT normalized_path, modified_at, file_size, availability FROM media_files WHERE source_id = ?1") {
        if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| {
            let mtime_str: Option<String> = row.get(1)?;
            let mtime = mtime_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            let size: Option<i64> = row.get(2)?;
            Ok((row.get::<_, String>(0)?, mtime, size.unwrap_or(0), row.get::<_, String>(3)?))
        }) {
            for r in rows.filter_map(Result::ok) {
                file_cache.insert(r.0, (r.1, r.2, r.3));
            }
        }
    }
    info!(
        "Loaded {} existing files for WebDAV incremental scan check.",
        file_cache.len()
    );

    let mut scanned_paths = HashSet::new();
    let seen_hashes = Mutex::new(HashSet::new());
    let mut batch: Vec<PreparedFile> = Vec::with_capacity(50);
    let source_root = PathBuf::from(
        reqwest::Url::parse(&root_uri)
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| root_uri.clone()),
    );

    // 联网行为: §2E —— 地址来自 sources.root_uri（用户自填）
    let webdav = WebdavClient::new(root_uri.clone(), username, password);

    // Recursive propfind (start at base URL, not server root)
    let mut dirs_to_scan = vec!["".to_string()];

    while let Some(current_dir) = dirs_to_scan.pop() {
        let files = match webdav.propfind(&current_dir) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to propfind {}: {}", current_dir, e);
                scan_failed = true;
                continue;
            }
        };

        for file in files {
            if file.is_dir {
                dirs_to_scan.push(file.path.clone());
                continue;
            }

            let entry_path = PathBuf::from(&file.path);
            let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if !is_supported_audio_ext(&ext.to_lowercase()) {
                continue;
            }

            // WebDAV dates are like "Mon, 12 Jul 2021 15:45:10 GMT".
            // We can just hash or approximate mtime. For simplicity we parse or fallback to 0.
            let fs_mtime = if let Ok(t) = chrono::DateTime::parse_from_rfc2822(&file.last_modified)
            {
                t.timestamp()
            } else {
                0
            };
            let fs_size = file.size as i64;

            let relative_path = entry_path
                .strip_prefix(&source_root)
                .unwrap_or(&entry_path)
                .to_string_lossy()
                .to_string();
            let normalized_path = relative_path.to_lowercase();
            scanned_paths.insert(normalized_path.clone());

            if let Some(&(db_mtime, db_size, ref availability)) = file_cache.get(&normalized_path) {
                if db_mtime == fs_mtime && db_size == fs_size && availability == "available" {
                    skipped_count += 1;
                    if skipped_count.is_multiple_of(50) {
                        let _ = app.emit(
                            "scan-progress",
                            ScanProgressPayload {
                                source_id,
                                scanned_count,
                                skipped_count,
                                current_path: file.path.clone(),
                            },
                        );
                    }
                    continue;
                }
            }

            scanned_count += 1;
            // 每 20 个新扫描文件发一次进度（远程扫描更慢，阈值放宽减少 IPC 噪音）
            if scanned_count.is_multiple_of(20) || scanned_count == 1 {
                let _ = app.emit(
                    "scan-progress",
                    ScanProgressPayload {
                        source_id,
                        scanned_count,
                        skipped_count,
                        current_path: file.path.clone(),
                    },
                );
            }

            // Extract metadata via HttpRangeReader
            let file_url = if file.path.starts_with("http://") || file.path.starts_with("https://")
            {
                file.path.clone()
            } else {
                let base = reqwest::Url::parse(&format!("{}/", root_uri)).unwrap();
                base.join(&file.path).unwrap().to_string()
            };
            let http_reader = HttpRangeReader::new(&webdav, file_url, file.size);
            // Wrap in BufReader to reduce tiny HTTP range requests
            let buffered_reader = std::io::BufReader::with_capacity(32 * 1024, http_reader);

            match crate::services::metadata::extract_metadata_from_reader(buffered_reader) {
                Ok(metadata) => {
                    let prepared = prepare_file(
                        metadata,
                        entry_path.clone(),
                        fs_mtime,
                        fs_size,
                        app_data_dir,
                        false,
                        &seen_hashes,
                    );
                    batch.push(prepared);
                    if batch.len() >= 50 {
                        flush_batch(
                            &mut scan_conn,
                            source_id,
                            &source_root,
                            &mut batch,
                            app_data_dir,
                        );
                    }
                }
                Err(err) => {
                    error!(
                        "Failed to extract metadata from WebDAV {:?}: {}",
                        file.path, err
                    );
                    mark_scan_error(&scan_conn, source_id, &source_root, &entry_path, &err);
                }
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(
            &mut scan_conn,
            source_id,
            &source_root,
            &mut batch,
            app_data_dir,
        );
    }

    // Cleanup logic for missing files
    let missing_count: usize;
    {
        let conn = &mut scan_conn;
        let mut to_delete = Vec::new();
        if let Ok(mut stmt) =
            conn.prepare("SELECT normalized_path FROM media_files WHERE source_id = ?1")
        {
            if let Ok(rows) =
                stmt.query_map(rusqlite::params![source_id], |row| row.get::<_, String>(0))
            {
                if !scan_failed {
                    for db_path in rows.filter_map(Result::ok) {
                        if !scanned_paths.contains(&db_path) {
                            to_delete.push(db_path);
                        }
                    }
                }
            }
        }

        missing_count = to_delete.len();
        if missing_count > 0 {
            info!(
                "Found {} missing WebDAV files, marking them as missing...",
                missing_count
            );
            if let Ok(tx) = conn.transaction() {
                for missing_path in to_delete {
                    let _ = tx.execute(
                        "UPDATE media_files SET availability = 'missing' WHERE source_id = ?1 AND normalized_path = ?2",
                        rusqlite::params![source_id, missing_path]
                    );
                }
                let _ = tx.commit();
            }
        }

        let _ = conn.execute(
            "UPDATE media_files SET last_seen_at = datetime('now') WHERE source_id = ?1 AND availability = 'available'",
            rusqlite::params![source_id],
        );
    }

    info!(
        "WebDAV Scan completed: scanned={}, skipped={}, missing={}",
        scanned_count, skipped_count, missing_count
    );
    // 先归还贯穿整轮扫描的连接，再让终止出口自己去取一条：连接池只有 8 条，
    // 扫描期间不额外占第二条，收尾记账也就不会在满载时排队等待。
    drop(scan_conn);
    finish_scan(
        &app,
        source_id,
        if scan_failed {
            ScanOutcome::failed("scan_incomplete", "WebDAV 扫描未完成，已跳过缺失文件清理")
        } else {
            ScanOutcome::Success
        },
    );
}

/// 把损坏的文件也记一行到 media_files（availability='error'），
/// 让用户在文件浏览器里能看到"有这个文件但无法解析"，而不会污染曲库。
/// 在扫描持有的连接上以自动提交执行（调用点都在批量事务之外）。
fn mark_scan_error(conn: &Connection, source_id: i64, source_root: &Path, path: &Path, err: &str) {
    // 与 index_file 保持同一套相对路径/normalized_path 口径：
    // 旧版本这里用完整路径当 normalized_path，会导致增量判定失配、
    // 缺失清理把坏文件误标 missing（每次扫描翻转一次）。
    let relative = path
        .strip_prefix(source_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let normalized = relative.to_lowercase();
    let legacy_full = path.to_string_lossy().to_lowercase();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    // 截断过长的错误信息，避免单条记录异常巨大
    let err_msg: String = err.chars().take(500).collect();

    // 自愈：清掉旧版本写入的"完整路径"行，避免残留成永远 missing 的幽灵记录
    if legacy_full != normalized {
        let _ = conn.execute(
            "DELETE FROM media_files WHERE source_id = ?1 AND normalized_path = ?2",
            rusqlite::params![source_id, legacy_full],
        );
    }

    let _ = conn.execute(
        "INSERT INTO media_files (
            source_id, relative_path, normalized_path, file_name, file_ext,
            availability, scan_error, last_seen_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'error', ?6, datetime('now'))
        ON CONFLICT(source_id, normalized_path) DO UPDATE SET
            scan_error=excluded.scan_error,
            availability='error',
            last_seen_at=datetime('now')",
        rusqlite::params![source_id, relative, normalized, file_name, ext, err_msg],
    );
}

/// 扫描状态记账（G-08）。
///
/// `last_scan_at` 从此只代表「上一次**成功**扫描」：失败路径不再推进它，只写 `last_error`。
/// 此前两者一起刷新，一个每次都失败的来源在界面上仍显示"刚刚扫描"，
/// 用户完全看不出它其实一直没成功过。
fn record_scan_result(
    conn: &rusqlite::Connection,
    source_id: i64,
    failed: bool,
    failure_note: &str,
) {
    let result = if failed {
        conn.execute(
            "UPDATE sources SET last_error = ?1 WHERE id = ?2",
            rusqlite::params![failure_note, source_id],
        )
    } else {
        conn.execute(
            "UPDATE sources SET last_scan_at = datetime('now'), last_error = NULL WHERE id = ?1",
            rusqlite::params![source_id],
        )
    };
    if let Err(e) = result {
        warn!("更新来源 {} 扫描状态失败: {}", source_id, e);
    }
}

/// 一次扫描的终态（CR-006）。
///
/// 扫描线程一旦启动就必须落到这两种状态之一，并且**写库与发事件两件事一起做**：
/// 只写库，数据库恰好不可用时用户什么也看不到；只发事件，重开界面后错误就消失了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanOutcome {
    Success,
    Failed {
        /// 稳定的机器可读分类；前端据此区分展示，不随文案改写而变。
        code: &'static str,
        /// 面向用户的中文说明：写进 `sources.last_error`，同时随 `scan-complete` 下发。
        message: String,
    },
}

impl ScanOutcome {
    pub(crate) fn failed(code: &'static str, message: impl Into<String>) -> Self {
        Self::Failed {
            code,
            message: message.into(),
        }
    }

    fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// 写库用的说明文案；成功路径不读这个值。
    fn note(&self) -> &str {
        match self {
            Self::Success => "",
            Self::Failed { message, .. } => message,
        }
    }
}

/// `scan-complete` 的结构化载荷（CR-006）。
///
/// 原来事件只带 `source_id`，前端回读数据库当作本次结果；DB 写入失败时那条路径
/// 会把「上一次的旧错误」当成这次的状态。现在失败原因随事件一起给出。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ScanCompletePayload {
    pub source_id: i64,
    pub success: bool,
    pub error_code: Option<&'static str>,
    pub message: Option<String>,
    /// 终态是否已写进 `sources`：false 表示数据库当时不可用，前端必须直接展示
    /// `message`，而不是回读一份永远停在旧值的表。
    pub persisted: bool,
}

impl ScanCompletePayload {
    fn new(source_id: i64, outcome: &ScanOutcome, persisted: bool) -> Self {
        let (success, error_code, message) = match outcome {
            ScanOutcome::Success => (true, None, None),
            ScanOutcome::Failed { code, message } => (false, Some(*code), Some(message.clone())),
        };
        Self {
            source_id,
            success,
            error_code,
            message,
            persisted,
        }
    }
}

/// 扫描的唯一终止出口（CR-006）：落库 → 记日志 → 发结构化 `scan-complete`。
///
/// 所有「扫描已启动但提前退出」的路径都必须走这里，否则前端会一直停在「扫描中」。
/// 记账先于事件：前端收到 `scan-complete` 就回读来源列表，顺序反了会读到上一次的状态。
pub(crate) fn finish_scan(app: &AppHandle, source_id: i64, outcome: ScanOutcome) {
    let persisted = match app.try_state::<DbState>() {
        Some(db_state) => match db_state.db.get() {
            Ok(conn) => {
                record_scan_result(&conn, source_id, !outcome.is_success(), outcome.note());
                true
            }
            // 池耗尽（CR-006 建议 5 的场景）：状态落不了库，但事件必须照发
            Err(e) => {
                error!(
                    "来源 {} 的扫描终态未能落库（取不到数据库连接: {}）",
                    source_id, e
                );
                false
            }
        },
        None => {
            error!(
                "来源 {} 的扫描终态未能落库：应用状态里没有数据库连接",
                source_id
            );
            false
        }
    };
    match &outcome {
        ScanOutcome::Success => info!("来源 {} 扫描完成", source_id),
        ScanOutcome::Failed { code, message } => {
            error!("来源 {} 扫描终止 [{}]: {}", source_id, code, message)
        }
    }
    let _ = app.emit(
        "scan-complete",
        ScanCompletePayload::new(source_id, &outcome, persisted),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_util::TempDir;

    /// 建一份带单个来源的测试库，返回 (临时目录, 连接, source_id)。
    fn db_with_source(label: &str) -> (TempDir, Connection, i64) {
        let dir = TempDir::new(label);
        crate::db::init_db(dir.db_path()).expect("构造测试库失败");
        let conn = Connection::open(dir.db_path()).expect("打开测试库失败");
        conn.execute(
            "INSERT INTO sources (name, kind, root_uri) VALUES ('测试来源', 'webdav', 'http://127.0.0.1/dav')",
            [],
        )
        .expect("插入来源失败");
        let id = conn.last_insert_rowid();
        (dir, conn, id)
    }

    fn source_state(conn: &Connection, source_id: i64) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT last_scan_at, last_error FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("读回来源状态失败")
    }

    /// `scan-complete` 载荷必须是结构化的：前端靠它区分「本次成功」与「本次失败且原因」，
    /// 光一个 source_id 无法在数据库没写进去时给出任何反馈（CR-006）。
    #[test]
    fn scan_complete_payload_is_structured() {
        let ok = ScanCompletePayload::new(7, &ScanOutcome::Success, true);
        assert!(ok.success && ok.error_code.is_none() && ok.message.is_none());
        assert_eq!(ok.source_id, 7);

        let failed = ScanCompletePayload::new(
            7,
            &ScanOutcome::failed("credential_unresolved", "凭据已失效"),
            true,
        );
        assert!(!failed.success);
        assert_eq!(failed.error_code, Some("credential_unresolved"));
        assert_eq!(failed.message.as_deref(), Some("凭据已失效"));

        // 序列化字段名是前端契约（src/api/scanner.ts 的 ScanCompleteEvent）
        let json = serde_json::to_value(&failed).expect("载荷应可序列化");
        assert_eq!(json["source_id"], 7);
        assert_eq!(json["success"], false);
        assert_eq!(json["error_code"], "credential_unresolved");
        assert_eq!(json["message"], "凭据已失效");
        assert_eq!(json["persisted"], true);
    }

    /// 落不了库时事件仍要带着原因：`persisted=false` 是前端改用临时文案的唯一依据。
    #[test]
    fn unpersisted_failure_still_carries_the_reason() {
        let payload = ScanCompletePayload::new(
            3,
            &ScanOutcome::failed("db_unavailable", "本地数据库正忙"),
            false,
        );
        let json = serde_json::to_value(&payload).expect("载荷应可序列化");
        assert_eq!(json["persisted"], false);
        assert_eq!(json["message"], "本地数据库正忙");
    }

    /// 失败只写 last_error、不推进 last_scan_at（G-08 口径）；
    /// 扫描早退同样走这条路，所以每个早退原因都会留在这里（CR-006）。
    #[test]
    fn failure_records_error_without_advancing_last_scan_at() {
        let (_dir, conn, id) = db_with_source("scan_outcome_failure");
        conn.execute(
            "UPDATE sources SET last_scan_at = datetime('now','-1 day') WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();
        let before = source_state(&conn, id);

        record_scan_result(&conn, id, true, "凭据解析失败：该来源的密码凭据已失效");

        let (last_scan_at, last_error) = source_state(&conn, id);
        assert_eq!(
            last_scan_at, before.0,
            "失败的扫描不能推进「上次成功扫描」时间"
        );
        assert_eq!(
            last_error.as_deref(),
            Some("凭据解析失败：该来源的密码凭据已失效")
        );
    }

    /// 成功清掉历史错误并推进时间：否则一次修好的扫描在界面上仍显示旧故障。
    #[test]
    fn success_clears_error_and_advances_last_scan_at() {
        let (_dir, conn, id) = db_with_source("scan_outcome_success");
        conn.execute(
            "UPDATE sources SET last_error = '上一次的错误' WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();

        record_scan_result(&conn, id, false, "");

        let (last_scan_at, last_error) = source_state(&conn, id);
        assert!(last_error.is_none(), "成功扫描要清掉 last_error");
        assert!(
            last_scan_at.is_some() && last_scan_at != Some("".to_string()),
            "成功扫描必须推进 last_scan_at"
        );
    }

    /// 终态只有两种形状：`failed()` 造出来的必然带 code + message，
    /// 这样 `finish_scan` 才不会发出「既不成功也不带原因」的事件。
    #[test]
    fn failed_outcome_always_has_code_and_message() {
        let outcome = ScanOutcome::failed("root_unreachable", "无法访问扫描根路径");
        assert!(!outcome.is_success());
        assert_eq!(outcome.note(), "无法访问扫描根路径");
        match &outcome {
            ScanOutcome::Failed { code, .. } => assert_eq!(*code, "root_unreachable"),
            ScanOutcome::Success => panic!("failed() 不可能构造出 Success"),
        }
        assert_eq!(ScanOutcome::Success.note(), "");
        assert!(ScanOutcome::Success.is_success());
    }
}
