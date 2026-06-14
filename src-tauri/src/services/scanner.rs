use walkdir::WalkDir;
use std::path::Path;
use tracing::{info, error, warn};
use crate::services::metadata::extract_metadata;
use crate::services::library::LibraryService;
use tauri::{AppHandle, Manager, Emitter};
use crate::db::DbState;
use serde::Serialize;
use std::path::PathBuf;

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
pub fn scan_local_directory(app: AppHandle, source_id: i64, path: &Path, app_data_dir: &Path) {
    info!("Starting async scan for directory: {:?}", path);
    let mut scanned_count = 0usize;
    let mut skipped_count = 0usize;

    // Batch to hold extracted metadata before inserting into DB
    let mut batch: Vec<(PathBuf, crate::services::metadata::AudioMetadata)> = Vec::with_capacity(50);

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) else { continue };
        if !is_supported_audio_ext(&ext.to_lowercase()) {
            continue;
        }

        // 1. 提取元数据（耗时操作，不持锁）。失败则跳过，避免把损坏文件入库为 Unknown。
        match extract_metadata(entry_path) {
            Ok(m) => {
                batch.push((entry_path.to_path_buf(), m));
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

    // Update the last_scan_at timestamp
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(conn) = db_state.db.get() {
            let _ = conn.execute(
                "UPDATE sources SET last_scan_at = datetime('now') WHERE id = ?1",
                rusqlite::params![source_id]
            );
        }
    }

    info!("Scan completed for directory: {:?} (scanned={}, skipped={})", path, scanned_count, skipped_count);
    let _ = app.emit("scan-complete", source_id);
}

/// 把损坏的文件也记一行到 media_files（availability='error'），
/// 让用户在文件浏览器里能看到"有这个文件但无法解析"，而不会污染曲库。
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
    batch: &mut Vec<(PathBuf, crate::services::metadata::AudioMetadata)>,
    app_data_dir: &Path,
) {
    let Some(db_state) = app.try_state::<DbState>() else { return };
    let Ok(mut conn) = db_state.db.get() else { return };

    let Ok(tx) = conn.transaction() else { return };
    for (path, metadata) in batch.iter() {
        if let Err(e) = LibraryService::index_file(&tx, source_id, source_root, path, metadata, app_data_dir) {
            error!("Failed to index file {:?}: {}", path, e);
        }
    }
    let _ = tx.commit();
    batch.clear();
}
