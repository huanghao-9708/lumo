use walkdir::WalkDir;
use std::path::Path;
use tracing::{info, error};
use crate::services::metadata::extract_metadata;
use crate::services::library::LibraryService;
use tauri::{AppHandle, Manager, Emitter};
use crate::db::DbState;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Serialize)]
pub struct ScanProgressPayload {
    pub source_id: i64,
    pub scanned_count: usize,
    pub current_path: String,
}

pub fn scan_local_directory(app: AppHandle, source_id: i64, path: &Path, app_data_dir: &Path) {
    info!("Starting async scan for directory: {:?}", path);
    let mut scanned_count = 0;
    
    // Batch to hold extracted metadata before inserting into DB
    let mut batch = Vec::with_capacity(50);

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.is_file() {
            if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "mp3" | "flac" | "wav" | "m4a" | "aac") {
                    // 1. Extract metadata (expensive, do not hold lock)
                    let metadata = extract_metadata(entry_path).unwrap_or_default();
                    
                    batch.push((entry_path.to_path_buf(), metadata));
                    scanned_count += 1;
                    
                    // 2. Emit progress event periodically
                    if scanned_count % 5 == 0 || scanned_count == 1 {
                        let _ = app.emit("scan-progress", ScanProgressPayload {
                            source_id,
                            scanned_count,
                            current_path: entry_path.to_string_lossy().to_string(),
                        });
                    }

                    // 3. Flush batch if it reaches 50
                    if batch.len() >= 50 {
                        flush_batch(&app, source_id, &mut batch, app_data_dir);
                    }
                }
            }
        }
    }

    // Flush any remaining items in the batch
    if !batch.is_empty() {
        flush_batch(&app, source_id, &mut batch, app_data_dir);
    }

    // Update the last_scan_at timestamp
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(conn) = db_state.db.lock() {
            let _ = conn.execute(
                "UPDATE sources SET last_scan_at = datetime('now') WHERE id = ?1",
                rusqlite::params![source_id]
            );
        }
    }

    info!("Scan completed for directory: {:?}", path);
    let _ = app.emit("scan-complete", source_id);
}

fn flush_batch(app: &AppHandle, source_id: i64, batch: &mut Vec<(PathBuf, crate::services::metadata::AudioMetadata)>, app_data_dir: &Path) {
    if let Some(db_state) = app.try_state::<DbState>() {
        if let Ok(mut conn) = db_state.db.lock() {
            if let Ok(tx) = conn.transaction() {
                for (path, metadata) in batch.iter() {
                    if let Err(e) = LibraryService::index_file(&tx, source_id, path, metadata, app_data_dir) {
                        error!("Failed to index file {:?}: {}", path, e);
                    }
                }
                let _ = tx.commit();
            }
        }
    }
    // Small sleep to ensure lock is yielded to frontend read queries
    std::thread::sleep(std::time::Duration::from_millis(10));
    batch.clear();
}
