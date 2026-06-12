use tauri::{State, Manager};
use crate::db::DbState;
use crate::services::playback::PlaybackManager;
use crate::services::scanner::scan_local_directory;
use crate::services::library::LibraryService;
use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO};
use std::path::PathBuf;
use rusqlite::params;
use std::sync::Mutex;

// For storing PlaybackManager state
pub struct PlaybackState {
    pub manager: Mutex<PlaybackManager>,
}

#[tauri::command]
pub fn source_add_local(db_state: State<'_, DbState>, path: String, name: String) -> Result<i64, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sources (name, kind, root_uri) VALUES (?1, 'local', ?2)",
        params![name, path],
    ).map_err(|e| e.to_string())?;
    
    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn source_scan(app: tauri::AppHandle, db_state: State<'_, DbState>, source_id: i64) -> Result<(), String> {
    let path = {
        let conn = db_state.db.lock().map_err(|e| e.to_string())?;
        let root_uri: String = conn.query_row(
            "SELECT root_uri FROM sources WHERE id = ?1",
            params![source_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        root_uri
    };

    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    scan_local_directory(&conn, source_id, &PathBuf::from(path), &app_dir);
    
    Ok(())
}

#[tauri::command]
pub fn source_list(db_state: State<'_, DbState>) -> Result<Vec<crate::models::Source>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("
        SELECT id, name, kind, root_uri, config_json, credential_ref, enabled, last_scan_at, last_error, created_at, updated_at 
        FROM sources 
        ORDER BY created_at DESC
    ").map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        Ok(crate::models::Source {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            root_uri: row.get(3)?,
            config_json: row.get(4)?,
            credential_ref: row.get(5)?,
            enabled: row.get::<_, i64>(6)? != 0,
            last_scan_at: row.get(7)?,
            last_error: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut sources = Vec::new();
    for r in rows {
        sources.push(r.map_err(|e| e.to_string())?);
    }
    Ok(sources)
}

#[tauri::command]
pub fn source_remove(db_state: State<'_, DbState>, source_id: i64) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    
    // Begin transaction
    conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;
    
    // Delete source. If PRAGMA foreign_keys = ON is set, this cascades to media_files
    if let Err(e) = conn.execute("DELETE FROM sources WHERE id = ?1", rusqlite::params![source_id]) {
        let _ = conn.execute("ROLLBACK", []);
        return Err(e.to_string());
    }
    
    // Clean up orphaned records
    let _ = conn.execute("DELETE FROM tracks WHERE id NOT IN (SELECT track_id FROM media_files)", []);
    let _ = conn.execute("DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM tracks WHERE album_id IS NOT NULL)", []);
    let _ = conn.execute("DELETE FROM artists WHERE id NOT IN (SELECT album_artist_id FROM albums WHERE album_artist_id IS NOT NULL UNION SELECT artist_id FROM tracks WHERE artist_id IS NOT NULL)", []);
    
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn library_get_tracks(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_tracks_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_albums(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<AlbumDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_albums_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artists(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<ArtistDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_artists_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_album_tracks(db_state: State<'_, DbState>, album_id: i64) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_album_tracks(&conn, album_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_albums(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<AlbumDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_artist_albums(&conn, artist_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_tracks(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_artist_tracks(&conn, artist_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_stats(db_state: State<'_, DbState>, artist_id: i64) -> Result<ArtistStatsDTO, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_artist_stats(&conn, artist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playback_play(playback_state: State<'_, PlaybackState>, db_state: State<'_, DbState>, media_file_id: i64) -> Result<(), String> {
    let path = {
        let conn = db_state.db.lock().map_err(|e| e.to_string())?;
        let path: String = conn.query_row(
            "SELECT normalized_path FROM media_files WHERE id = ?1",
            params![media_file_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        path
    };

    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.play_file(&PathBuf::from(path))
}

#[tauri::command]
pub fn playback_pause(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.pause();
    Ok(())
}

#[tauri::command]
pub fn playback_resume(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.resume();
    Ok(())
}

#[tauri::command]
pub fn playback_stop(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.stop();
    Ok(())
}

#[tauri::command]
pub fn playback_set_volume(playback_state: State<'_, PlaybackState>, volume: f32) -> Result<(), String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.set_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn playback_get_pos(playback_state: State<'_, PlaybackState>) -> Result<u64, String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_pos())
}

#[tauri::command]
pub fn playback_seek(playback_state: State<'_, PlaybackState>, position_ms: u64) -> Result<(), String> {
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.try_seek(position_ms)
}

#[tauri::command]
pub fn library_toggle_favorite(db_state: State<'_, DbState>, track_id: i64, is_favorite: bool) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::toggle_favorite(&conn, track_id, is_favorite).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_create_playlist(db_state: State<'_, DbState>, name: String, description: Option<String>) -> Result<i64, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::create_playlist(&conn, &name, description.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_playlists(db_state: State<'_, DbState>) -> Result<Vec<PlaylistDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_playlists(&conn).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
pub fn library_add_to_playlist(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::add_to_playlist(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_playlist_tracks(db_state: State<'_, DbState>, playlist_id: i64) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_playlist_tracks(&conn, playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_record_play(db_state: State<'_, DbState>, track_id: i64) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::record_play(&conn, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_recently_played(db_state: State<'_, DbState>, limit: u32) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_recently_played(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_favorite_tracks(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_favorite_tracks(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_lyrics(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<String>, String> {
    use rusqlite::OptionalExtension;
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    let lyr: Option<String> = conn.query_row(
        "SELECT content FROM lyrics WHERE track_id = ?1 LIMIT 1",
        params![track_id],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    Ok(lyr)
}

#[tauri::command]
pub fn library_get_track_file_info(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<crate::models::TrackFileInfoDTO>, String> {
    use rusqlite::OptionalExtension;
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("
        SELECT id, normalized_path, file_size, duration_ms, bitrate, sample_rate, bit_depth, channels, file_ext
        FROM media_files
        WHERE track_id = ?1
        LIMIT 1
    ").map_err(|e| e.to_string())?;
    
    let info = stmt.query_row(params![track_id], |row| {
        Ok(crate::models::TrackFileInfoDTO {
            id: row.get(0)?,
            path: row.get(1)?,
            file_size: row.get(2)?,
            duration_ms: row.get(3)?,
            bitrate: row.get(4)?,
            sample_rate: row.get(5)?,
            bit_depth: row.get(6)?,
            channels: row.get(7)?,
            format: row.get(8)?,
        })
    }).optional().map_err(|e| e.to_string())?;
    
    Ok(info)
}

#[tauri::command]
pub fn library_delete_playlist(db_state: State<'_, DbState>, playlist_id: i64) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::delete_playlist(&conn, playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_remove_playlist_item(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::remove_playlist_item(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_save_play_queue(db_state: State<'_, DbState>, track_ids: Vec<i64>) -> Result<(), String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::save_play_queue(&conn, &track_ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_play_queue(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, String> {
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    LibraryService::get_play_queue(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_cache_size(app: tauri::AppHandle) -> Result<u64, String> {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if !artworks_dir.exists() {
        return Ok(0);
    }
    
    let mut total_size = 0;
    if let Ok(entries) = std::fs::read_dir(artworks_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total_size += meta.len();
                }
            }
        }
    }
    Ok(total_size)
}

#[tauri::command]
pub fn library_clear_cache(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if artworks_dir.exists() {
        let _ = std::fs::remove_dir_all(&artworks_dir);
        let _ = std::fs::create_dir_all(&artworks_dir);
    }
    
    let conn = db_state.db.lock().map_err(|e| e.to_string())?;
    let _ = conn.execute("DELETE FROM artwork", []);
    let _ = conn.execute("UPDATE albums SET cover_artwork_id = NULL", []);
    
    Ok(())
}

