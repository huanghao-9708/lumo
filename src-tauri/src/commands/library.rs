use crate::error::AppError;
use tauri::{State, Manager};
use crate::db::DbState;
use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO};
use std::path::PathBuf;
use rusqlite::params;
use crate::ipc_trace;

// For storing PlaybackManager state













#[tauri::command]
pub fn library_get_tracks(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_tracks_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_albums(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_albums");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_albums_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_album_count(db_state: State<'_, DbState>, search_keyword: Option<String>) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_get_album_count");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_count(&conn, search_keyword).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artists(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<ArtistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artists");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artists_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_album_tracks(db_state: State<'_, DbState>, album_id: i64) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_album_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_tracks(&conn, album_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_albums(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_albums");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_albums(&conn, artist_id, limit, offset).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_album_count(db_state: State<'_, DbState>, artist_id: i64) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_get_artist_album_count");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_album_count(&conn, artist_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_tracks(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_tracks(&conn, artist_id, limit, offset).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_stats(db_state: State<'_, DbState>, artist_id: i64) -> Result<ArtistStatsDTO, AppError> {
    let _trace = ipc_trace!("library_get_artist_stats");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_stats(&conn, artist_id).map_err(|e| e.into())
}















#[tauri::command]
pub fn library_toggle_favorite(db_state: State<'_, DbState>, track_id: i64, is_favorite: bool) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::toggle_favorite(&conn, track_id, is_favorite).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_create_playlist(db_state: State<'_, DbState>, name: String, description: Option<String>) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_create_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::create_playlist(&conn, &name, description.as_deref()).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_playlists(db_state: State<'_, DbState>) -> Result<Vec<PlaylistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_playlists");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::get_playlists(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_add_to_playlist(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_add_to_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::add_to_playlist(&conn, playlist_id, track_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_playlist_tracks(db_state: State<'_, DbState>, playlist_id: i64) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_playlist_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::get_playlist_tracks(&conn, playlist_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_record_play(db_state: State<'_, DbState>, track_id: i64, duration_ms: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_record_play");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::record_play(&conn, track_id, duration_ms).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_recently_played(db_state: State<'_, DbState>, limit: u32) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_recently_played");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_recently_played(&conn, limit).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_favorite_tracks(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_favorite_tracks(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_lyrics(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<String>, AppError> {
    let _trace = ipc_trace!("library_get_lyrics");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get()?;
    let lyr: Option<String> = conn.query_row(
        "SELECT content FROM lyrics WHERE track_id = ?1 LIMIT 1",
        params![track_id],
        |row| row.get(0),
    ).optional()?;
    Ok(lyr)
}

#[tauri::command]
pub fn library_get_track_file_info(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<crate::models::TrackFileInfoDTO>, AppError> {
    let _trace = ipc_trace!("library_get_track_file_info");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get()?;
    let mut stmt = conn.prepare("
        SELECT id, relative_path, file_size, duration_ms, bitrate, sample_rate, bit_depth, channels, file_ext
        FROM media_files
        WHERE track_id = ?1
        LIMIT 1
    ")?;
    
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
    }).optional()?;
    
    Ok(info)
}

#[tauri::command]
pub fn library_delete_playlist(db_state: State<'_, DbState>, playlist_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_delete_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::delete_playlist(&conn, playlist_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_remove_playlist_item(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_remove_playlist_item");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::remove_playlist_item(&conn, playlist_id, track_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_save_play_queue(db_state: State<'_, DbState>, track_ids: Vec<i64>) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_save_play_queue");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::save_play_queue(&conn, &track_ids).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_play_queue(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_play_queue");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_play_queue(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_cache_size(app: tauri::AppHandle) -> Result<u64, AppError> {
    let _trace = ipc_trace!("library_get_cache_size");
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
pub fn library_clear_cache(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_clear_cache");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if artworks_dir.exists() {
        let _ = std::fs::remove_dir_all(&artworks_dir);
        let _ = std::fs::create_dir_all(&artworks_dir);
    }
    
    let conn = db_state.db.get()?;
    let _ = conn.execute("DELETE FROM artwork", []);
    let _ = conn.execute("UPDATE albums SET cover_artwork_id = NULL", []);
    
    Ok(())
}

#[tauri::command]
pub fn library_get_folder_contents(
    db_state: State<'_, DbState>,
    source_id: i64,
    folder_path: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<crate::models::FolderContentsResult, AppError> {
    let _trace = ipc_trace!("library_get_folder_contents");
    let conn = db_state.db.get()?;

    // 如果未指定 folder_path，则使用该 source 对应的本地根路径
    let real_path = if let Some(p) = folder_path {
        std::path::PathBuf::from(p)
    } else {
        let root_uri: String = conn.query_row(
            "SELECT root_uri FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| row.get(0),
        )?;
        std::path::PathBuf::from(root_uri)
    };

    crate::repositories::track_repo::TrackRepo::get_folder_contents(&conn, source_id, &real_path, limit, offset.unwrap_or(0))
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_add_folder_to_playlist(db_state: State<'_, DbState>, source_id: i64, folder_path: String, playlist_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_add_folder_to_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::add_folder_to_playlist(&conn, playlist_id, source_id, &folder_path).map_err(|e| e.into())
}
