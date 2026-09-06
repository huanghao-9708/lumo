use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::{info, error, warn};
use sha2::{Sha256, Digest};
use rusqlite::Connection;
use tauri::{AppHandle, Manager, Emitter};
use crate::db::DbState;
use crate::services::metadata::{extract_metadata, AudioMetadata};
use crate::services::library::{LibraryService, PreparedArtwork, PreparedFile};
use crate::services::webdav::{WebdavClient, HttpRangeReader};
use serde::Serialize;

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
    Ok(PreparedFile),
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
    PreparedFile { path, metadata, mtime, size, artwork, lrc_content }
}

/// 封面预处理：SHA256 + （本扫描内首次遇到该 hash 时）写原图缓存与生成缩略图。
/// 同 hash 只有第一个文件做磁盘工作，后续文件只带 hash 去写库侧复用。
fn prepare_artwork(
    metadata: &mut AudioMetadata,
    app_data_dir: &Path,
    seen_hashes: &Mutex<HashSet<String>>,
) -> PreparedArtwork {
    let Some(data) = metadata.picture_data.take() else { return PreparedArtwork::default() };
    if data.is_empty() {
        return PreparedArtwork::default();
    }
    let mime = metadata.picture_mime.clone();

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());

    let is_first = seen_hashes.lock().unwrap().insert(hash.clone());
    if !is_first {
        return PreparedArtwork { hash: Some(hash), mime, cache_path: None, thumbnail: None };
    }

    let cache_path = LibraryService::artwork_cache_path(app_data_dir, mime.as_deref(), &hash);
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !cache_path.exists() {
        let _ = std::fs::write(&cache_path, &data);
    }
    let thumbnail = LibraryService::generate_thumbnail(&data);
    PreparedArtwork { hash: Some(hash), mime, cache_path: Some(cache_path), thumbnail }
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
                let prepared = prepare_file(metadata, path, mtime, size, &app_data_dir, true, &seen_hashes);
                if results_tx.send(ExtractOutcome::Ok(prepared)).is_err() {
                    break;
                }
            }
            Err(e) => {
                if results_tx.send(ExtractOutcome::Err { path, error: e }).is_err() {
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
    let Some(db_state) = app.try_state::<DbState>() else { return totals };
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
                    let _ = app.emit("scan-progress", ScanProgressPayload {
                        source_id,
                        scanned_count: totals.scanned,
                        skipped_count: skipped.load(Ordering::Relaxed),
                        current_path: prepared.path.to_string_lossy().to_string(),
                    });
                }
                batch.push(prepared);
                if batch.len() >= 50 {
                    flush_batch(&mut conn, source_id, &source_root, &mut batch, &app_data_dir);
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
        flush_batch(&mut conn, source_id, &source_root, &mut batch, &app_data_dir);
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
        if let Err(e) = LibraryService::index_file(&tx, source_id, source_root, prepared, app_data_dir) {
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
            let message = "扫描根路径不是目录";
            update_scan_status(&app, source_id, false, Some(message));
            let _ = app.emit("scan-complete", source_id);
            return;
        }
        Err(e) => {
            let message = format!("无法访问扫描根路径: {}", e);
            update_scan_status(&app, source_id, false, Some(&message));
            let _ = app.emit("scan-complete", source_id);
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
    info!("Loaded {} existing files for incremental scan check.", file_cache.len());

    let mut scanned_paths = HashSet::new();
    let skipped_shared = Arc::new(AtomicUsize::new(0));
    let scanned_shared = Arc::new(AtomicUsize::new(0));
    let seen_hashes = Arc::new(Mutex::new(HashSet::new()));

    // 提取 worker 数：吃满并行度但封顶 4（机械盘上过多并发读反而互相寻道）
    let worker_count = thread::available_parallelism().map(|n| n.get()).unwrap_or(2).min(4);

    let (paths_tx, paths_rx) = mpsc::channel::<(PathBuf, i64, i64)>();
    let (results_tx, results_rx) = mpsc::channel::<ExtractOutcome>();

    // 写库线程（唯一写者，独占连接）
    let writer_app = app.clone();
    let writer_root = path.to_path_buf();
    let writer_data_dir = app_data_dir.to_path_buf();
    let writer_skipped = Arc::clone(&skipped_shared);
    let writer_scanned = Arc::clone(&scanned_shared);
    let writer = thread::spawn(move || {
        db_writer_loop(writer_app, source_id, writer_root, writer_data_dir, results_rx, writer_skipped, writer_scanned)
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
        let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else { continue };
        if !is_supported_audio_ext(&ext.to_lowercase()) {
            continue;
        }

        // 1. 获取文件系统元数据 (mtime, size)
        let fs_metadata = std::fs::metadata(&entry_path).ok();
        let fs_size = fs_metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
        let fs_mtime = fs_metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // 构建 normalized_path
        let relative_path = entry_path.strip_prefix(path).unwrap_or(&entry_path).to_string_lossy().to_string();
        let normalized_path = relative_path.to_lowercase();

        // 记录已扫描的文件路径，用于后续的删除检测
        scanned_paths.insert(normalized_path.clone());

        // 增量判定：如果数据库中已经有且大小和修改时间均一致，则跳过解析
        if let Some(&(db_mtime, db_size, ref availability)) = file_cache.get(&normalized_path) {
            if db_mtime == fs_mtime && db_size == fs_size && availability == "available" {
                skipped_count += 1;
                skipped_shared.store(skipped_count, Ordering::Relaxed);

                // 每 50 个 skipped 也发一次进度，避免长久卡顿感
                if skipped_count % 50 == 0 {
                    let _ = app.emit("scan-progress", ScanProgressPayload {
                        source_id,
                        scanned_count: scanned_shared.load(Ordering::Relaxed),
                        skipped_count,
                        current_path: entry_path.to_string_lossy().to_string(),
                    });
                }
                continue;
            }
        }

        // 2. 送入提取流水线（解析失败在 worker 内部处理并回传）
        if paths_tx.send((entry_path.to_path_buf(), fs_mtime, fs_size)).is_err() {
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
                info!("Found {} missing files, marking them as missing...", missing_count);
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

            // Update the last_scan_at timestamp
            let _ = conn.execute(
                "UPDATE sources SET last_scan_at = datetime('now') WHERE id = ?1",
                rusqlite::params![source_id]
            );

            // Also update last_seen_at for all scanned paths to keep them 'available'
            let _ = conn.execute(
                "UPDATE media_files SET last_seen_at = datetime('now') WHERE source_id = ?1 AND availability = 'available'",
                rusqlite::params![source_id]
            );
            let _ = conn.execute(
                "UPDATE sources SET last_scan_at = datetime('now'), last_error = ?1 WHERE id = ?2",
                rusqlite::params![if scan_failed { Some("扫描未完成，已跳过缺失文件清理") } else { None }, source_id],
            );
        }
    }

    info!(
        "Scan completed for directory: {:?} (scanned={}, errors={}, skipped={}, missing={})",
        path, totals.scanned, totals.errors, skipped_count, missing_count
    );
    let _ = app.emit("scan-complete", source_id);
}

/// 执行 WebDAV 远程目录扫描（串行；预处理与本地 worker 相同，但进度事件按 20 个节流）。
/// 使用 HTTP HEAD/GET 探测文件列表，并尝试部分读取元数据。
pub fn scan_webdav_directory(app: AppHandle, source_id: i64, root_uri: String, username: Option<String>, password: Option<String>, app_data_dir: &Path) {
    info!("Starting async scan for WebDAV: {}", root_uri);
    let mut scanned_count = 0usize;
    let mut skipped_count = 0usize;
    let mut scan_failed = false;

    let Some(db_state) = app.try_state::<DbState>() else { return };
    // 独占一条连接贯穿整个扫描（文件缓存加载 / 预处理错误标记 / 批量写库 / 收尾）
    let Ok(mut scan_conn) = db_state.db.get() else { return };
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
    info!("Loaded {} existing files for WebDAV incremental scan check.", file_cache.len());

    let mut scanned_paths = HashSet::new();
    let seen_hashes = Mutex::new(HashSet::new());
    let mut batch: Vec<PreparedFile> = Vec::with_capacity(50);
    let source_root = PathBuf::from(reqwest::Url::parse(&root_uri).map(|u| u.path().to_string()).unwrap_or_else(|_| root_uri.clone()));

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
            let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else { continue };
            if !is_supported_audio_ext(&ext.to_lowercase()) {
                continue;
            }

            // WebDAV dates are like "Mon, 12 Jul 2021 15:45:10 GMT".
            // We can just hash or approximate mtime. For simplicity we parse or fallback to 0.
            let fs_mtime = if let Ok(t) = chrono::DateTime::parse_from_rfc2822(&file.last_modified) {
                t.timestamp()
            } else {
                0
            };
            let fs_size = file.size as i64;

            let relative_path = entry_path.strip_prefix(&source_root).unwrap_or(&entry_path).to_string_lossy().to_string();
            let normalized_path = relative_path.to_lowercase();
            scanned_paths.insert(normalized_path.clone());

            if let Some(&(db_mtime, db_size, ref availability)) = file_cache.get(&normalized_path) {
                if db_mtime == fs_mtime && db_size == fs_size && availability == "available" {
                    skipped_count += 1;
                    if skipped_count % 50 == 0 {
                        let _ = app.emit("scan-progress", ScanProgressPayload {
                            source_id,
                            scanned_count,
                            skipped_count,
                            current_path: file.path.clone(),
                        });
                    }
                    continue;
                }
            }

            scanned_count += 1;
            // 每 20 个新扫描文件发一次进度（远程扫描更慢，阈值放宽减少 IPC 噪音）
            if scanned_count % 20 == 0 || scanned_count == 1 {
                let _ = app.emit("scan-progress", ScanProgressPayload {
                    source_id,
                    scanned_count,
                    skipped_count,
                    current_path: file.path.clone(),
                });
            }

            // Extract metadata via HttpRangeReader
            let file_url = if file.path.starts_with("http://") || file.path.starts_with("https://") {
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
                    let prepared = prepare_file(metadata, entry_path.clone(), fs_mtime, fs_size, app_data_dir, false, &seen_hashes);
                    batch.push(prepared);
                    if batch.len() >= 50 {
                        flush_batch(&mut scan_conn, source_id, &source_root, &mut batch, app_data_dir);
                    }
                }
                Err(err) => {
                    error!("Failed to extract metadata from WebDAV {:?}: {}", file.path, err);
                    mark_scan_error(&scan_conn, source_id, &source_root, &entry_path, &err);
                }
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(&mut scan_conn, source_id, &source_root, &mut batch, app_data_dir);
    }

    // Cleanup logic for missing files
    let missing_count: usize;
    {
        let conn = &mut scan_conn;
        let mut to_delete = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT normalized_path FROM media_files WHERE source_id = ?1") {
            if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| row.get::<_, String>(0)) {
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
            info!("Found {} missing WebDAV files, marking them as missing...", missing_count);
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
        let _ = conn.execute(
            "UPDATE sources SET last_scan_at = datetime('now'), last_error = ?1 WHERE id = ?2",
            rusqlite::params![if scan_failed { Some("WebDAV 扫描未完成，已跳过缺失文件清理") } else { None }, source_id],
        );
    }

    info!("WebDAV Scan completed: scanned={}, skipped={}, missing={}", scanned_count, skipped_count, missing_count);
    let _ = app.emit("scan-complete", source_id);
}

/// 把损坏的文件也记一行到 media_files（availability='error'），
/// 让用户在文件浏览器里能看到"有这个文件但无法解析"，而不会污染曲库。
/// 在扫描持有的连接上以自动提交执行（调用点都在批量事务之外）。
fn mark_scan_error(conn: &Connection, source_id: i64, source_root: &Path, path: &Path, err: &str) {
    // 与 index_file 保持同一套相对路径/normalized_path 口径：
    // 旧版本这里用完整路径当 normalized_path，会导致增量判定失配、
    // 缺失清理把坏文件误标 missing（每次扫描翻转一次）。
    let relative = path.strip_prefix(source_root).unwrap_or(path).to_string_lossy().to_string();
    let normalized = relative.to_lowercase();
    let legacy_full = path.to_string_lossy().to_lowercase();
    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
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

fn update_scan_status(app: &AppHandle, source_id: i64, success: bool, error_message: Option<&str>) {
    let Some(db_state) = app.try_state::<DbState>() else { return };
    let Ok(conn) = db_state.db.get() else { return };
    let message = if success { None } else { error_message };
    let _ = conn.execute(
        "UPDATE sources SET last_scan_at = datetime('now'), last_error = ?1 WHERE id = ?2",
        rusqlite::params![message, source_id],
    );
}
