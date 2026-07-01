use walkdir::WalkDir;
use std::path::Path;
use tracing::{info, error, warn};
use crate::services::metadata::extract_metadata;
use crate::services::library::LibraryService;
use tauri::{AppHandle, Manager, Emitter};
use crate::db::DbState;
use serde::Serialize;
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use crate::services::webdav::{WebdavClient, HttpRangeReader};

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

/// 异步扫描本地目录：在独立线程中遍历，按 50 条/批写入数据库。
/// 损坏或无法解析的文件会被跳过并计数（不再以 Unknown 形式污染曲库）。
/// 执行本地目录扫描。
/// 解析目录中支持的音频文件（mp3, flac, wav, m4a），提取元数据（ID3等）并入库。
/// 如果文件已在数据库中，将检查其修改时间以决定是否更新。
pub fn scan_local_directory(app: AppHandle, source_id: i64, path: &Path, app_data_dir: &Path) {
    info!("Starting async scan for directory: {:?}", path);
    let mut scanned_count = 0usize;
    let mut skipped_count = 0usize;

    // Load existing files for incremental scan
    let mut file_cache: HashMap<String, (i64, i64)> = HashMap::new();
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(conn) = db_state.db.get() {
            if let Ok(mut stmt) = conn.prepare("SELECT normalized_path, modified_at, file_size FROM media_files WHERE source_id = ?1") {
                if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| {
                    let mtime_str: Option<String> = row.get(1)?;
                    let mtime = mtime_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    let size: Option<i64> = row.get(2)?;
                    Ok((row.get::<_, String>(0)?, mtime, size.unwrap_or(0)))
                }) {
                    for r in rows.filter_map(Result::ok) {
                        file_cache.insert(r.0, (r.1, r.2));
                    }
                }
            }
        }
    }
    info!("Loaded {} existing files for incremental scan check.", file_cache.len());

    let mut scanned_paths = HashSet::new();

    // Batch to hold extracted metadata before inserting into DB
    let mut batch: Vec<(PathBuf, crate::services::metadata::AudioMetadata, i64, i64)> = Vec::with_capacity(50);

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
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
        if let Some(&(db_mtime, db_size)) = file_cache.get(&normalized_path) {
            if db_mtime == fs_mtime && db_size == fs_size {
                skipped_count += 1;
                
                // 每 50 个 skipped 也发一次进度，避免长久卡顿感
                if skipped_count % 50 == 0 {
                    let _ = app.emit("scan-progress", ScanProgressPayload {
                        source_id,
                        scanned_count,
                        skipped_count,
                        current_path: entry_path.to_string_lossy().to_string(),
                    });
                }
                
                // 这里我们要更新 last_seen_at，因为文件仍在，只是没变。
                // 暂时收集到 batch 或直接更新。为了不影响性能，可以搞个小 batch 更新。
                // 简单起见，我们可以在后续 flush 或最后做，但考虑到批量，最好单独维护一个 seen 列表
                continue;
            }
        }

        // 2. 提取元数据（耗时操作，不持锁）。失败则跳过，避免把损坏文件入库为 Unknown。
        match extract_metadata(entry_path) {
            Ok(m) => {
                batch.push((entry_path.to_path_buf(), m, fs_mtime, fs_size));
                scanned_count += 1;
            }
            Err(e) => {
                skipped_count += 1;
                warn!("Skipped file (metadata error): {:?} - {}", entry_path, e);
                // 仍然把这条记录写到 media_files（availability='error'），让用户知道有这个文件但无法播放。
                // 但不创建对应的 track/album/artist 记录。
                mark_scan_error(&app, source_id, entry_path, &e);
            }
        }

        // 2. 周期性推送进度事件
        if scanned_count % 5 == 0 || scanned_count == 1 {
            let _ = app.emit("scan-progress", ScanProgressPayload {
                source_id,
                scanned_count,
                skipped_count,
                current_path: entry_path.to_string_lossy().to_string(),
            });
        }

        // 3. 攒满 50 条就提交一次
        if batch.len() >= 50 {
            flush_batch(&app, source_id, path, &mut batch, app_data_dir);
        }
    }

    // Flush any remaining items in the batch
    if !batch.is_empty() {
        flush_batch(&app, source_id, path, &mut batch, app_data_dir);
    }

    // Clean up missing files (deleted from disk)
    let mut missing_count = 0;
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(mut conn) = db_state.db.get() {
            // Find files in db that are not in scanned_paths
            let mut to_delete = Vec::new();
            for (db_path, _) in file_cache.iter() {
                if !scanned_paths.contains(db_path) {
                    to_delete.push(db_path.clone());
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
            // To do it efficiently, since we touched all, we can just bulk update the ones not missing
            let _ = conn.execute(
                "UPDATE media_files SET availability = 'available', last_seen_at = datetime('now') WHERE source_id = ?1 AND availability != 'missing'",
                rusqlite::params![source_id]
            );
        }
    }

    info!("Scan completed for directory: {:?} (scanned={}, skipped={}, missing={})", path, scanned_count, skipped_count, missing_count);
    let _ = app.emit("scan-complete", source_id);
}

/// 执行 WebDAV 远程目录扫描。
/// 使用 HTTP HEAD/GET 探测文件列表，并尝试部分读取元数据。
pub fn scan_webdav_directory(app: AppHandle, source_id: i64, root_uri: String, username: Option<String>, password: Option<String>, app_data_dir: &Path) {
    info!("Starting async scan for WebDAV: {}", root_uri);
    let mut scanned_count = 0usize;
    let mut skipped_count = 0usize;

    let mut file_cache: HashMap<String, (i64, i64)> = HashMap::new();
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(conn) = db_state.db.get() {
            if let Ok(mut stmt) = conn.prepare("SELECT normalized_path, modified_at, file_size FROM media_files WHERE source_id = ?1") {
                if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| {
                    let mtime_str: Option<String> = row.get(1)?;
                    let mtime = mtime_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    let size: Option<i64> = row.get(2)?;
                    Ok((row.get::<_, String>(0)?, mtime, size.unwrap_or(0)))
                }) {
                    for r in rows.filter_map(Result::ok) {
                        file_cache.insert(r.0, (r.1, r.2));
                    }
                }
            }
        }
    }
    info!("Loaded {} existing files for WebDAV incremental scan check.", file_cache.len());

    let mut scanned_paths = HashSet::new();
    let mut batch = Vec::with_capacity(50);
    let source_root = PathBuf::from(reqwest::Url::parse(&root_uri).map(|u| u.path().to_string()).unwrap_or_else(|_| root_uri.clone()));
    
    let webdav = WebdavClient::new(root_uri.clone(), username, password);

    // Recursive propfind (start at base URL, not server root)
    let mut dirs_to_scan = vec!["".to_string()];
    
    while let Some(current_dir) = dirs_to_scan.pop() {
        let files = match webdav.propfind(&current_dir) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to propfind {}: {}", current_dir, e);
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

            if let Some(&(db_mtime, db_size)) = file_cache.get(&normalized_path) {
                if db_mtime == fs_mtime && db_size == fs_size {
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
            let _ = app.emit("scan-progress", ScanProgressPayload {
                source_id,
                scanned_count,
                skipped_count,
                current_path: file.path.clone(),
            });

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
                    batch.push((entry_path, metadata, fs_mtime, fs_size));
                    if batch.len() >= 50 {
                        flush_batch(&app, source_id, &source_root, &mut batch, app_data_dir);
                    }
                }
                Err(err) => {
                    error!("Failed to extract metadata from WebDAV {:?}: {}", file.path, err);
                    mark_scan_error(&app, source_id, &entry_path, &err);
                }
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(&app, source_id, &source_root, &mut batch, app_data_dir);
    }

    // Cleanup logic for missing files
    let mut missing_count = 0;
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(mut conn) = db_state.db.get() {
            let mut to_delete = Vec::new();
            if let Ok(mut stmt) = conn.prepare("SELECT normalized_path FROM media_files WHERE source_id = ?1") {
                if let Ok(rows) = stmt.query_map(rusqlite::params![source_id], |row| row.get::<_, String>(0)) {
                    for db_path in rows.filter_map(Result::ok) {
                        if !scanned_paths.contains(&db_path) {
                            to_delete.push(db_path);
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

            let _ = conn.execute("UPDATE sources SET last_scan_at = datetime('now') WHERE id = ?1", rusqlite::params![source_id]);
            let _ = conn.execute("UPDATE media_files SET availability = 'available', last_seen_at = datetime('now') WHERE source_id = ?1 AND availability != 'missing'", rusqlite::params![source_id]);
        }
    }

    info!("WebDAV Scan completed: scanned={}, skipped={}, missing={}", scanned_count, skipped_count, missing_count);
    let _ = app.emit("scan-complete", source_id);
}

/// 把损坏的文件也记一行到 media_files（availability='error'），
/// 让用户在文件浏览器里能看到"有这个文件但无法解析"，而不会污染曲库。
/// 解析单个音频文件的元数据，如果包含封面则将其缓存到本地临时目录。
/// 返回解析后的属性集合。
fn mark_scan_error(app: &AppHandle, source_id: i64, path: &Path, err: &str) {
    let Some(db_state) = app.try_state::<DbState>() else { return };
    let Ok(conn) = db_state.db.get() else { return };

    let relative = path.to_string_lossy().to_string();
    let normalized = relative.to_lowercase();
    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    // 截断过长的错误信息，避免单条记录异常巨大
    let err_msg: String = err.chars().take(500).collect();

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

/// 批量提交：把缓存好的元数据一次性写入数据库。
/// WAL 模式下写入期间前端读取不会被长时间阻塞，不再需要 sleep hack。
fn flush_batch(
    app: &AppHandle,
    source_id: i64,
    source_root: &Path,
    batch: &mut Vec<(PathBuf, crate::services::metadata::AudioMetadata, i64, i64)>,
    app_data_dir: &Path,
) {
    let Some(db_state) = app.try_state::<DbState>() else { return };
    let Ok(mut conn) = db_state.db.get() else { return };

    let Ok(tx) = conn.transaction() else { return };
    for (path, metadata, mtime, size) in batch.iter() {
        if let Err(e) = LibraryService::index_file(&tx, source_id, source_root, path, metadata, app_data_dir, *mtime, *size) {
            error!("Failed to index file {:?}: {}", path, e);
        }
    }
    let _ = tx.commit();
    batch.clear();
}
