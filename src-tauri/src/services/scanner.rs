use walkdir::WalkDir;
use std::path::Path;
use tracing::{info, error};
use crate::services::metadata::extract_metadata;
use crate::services::library::LibraryService;
use rusqlite::Connection;

pub fn scan_local_directory(conn: &Connection, source_id: i64, path: &Path, app_data_dir: &Path) {
    info!("Starting scan for directory: {:?}", path);
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.is_file() {
            if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "mp3" | "flac" | "wav" | "m4a" | "aac") {
                    let metadata = extract_metadata(entry_path).unwrap_or_default();
                    if let Err(e) = LibraryService::index_file(conn, source_id, entry_path, &metadata, app_data_dir) {
                        error!("Failed to index file {:?}: {}", entry_path, e);
                    }
                }
            }
        }
    }
    info!("Scan completed for directory: {:?}", path);
}
