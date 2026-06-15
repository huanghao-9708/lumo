use tauri::{State, Manager};
use crate::db::DbState;
use crate::services::playback::PlaybackManager;
use crate::services::scanner::scan_local_directory;
use crate::services::library::LibraryService;
use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO};
use std::path::PathBuf;
use rusqlite::params;
use std::sync::Mutex;
use crate::ipc_trace;

// For storing PlaybackManager state
pub struct PlaybackState {
    pub manager: Mutex<PlaybackManager>,
}

#[tauri::command]
pub fn source_add_local(db_state: State<'_, DbState>, path: String, name: String) -> Result<i64, String> {
    let _trace = ipc_trace!("source_add_local");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sources (name, kind, root_uri) VALUES (?1, 'local', ?2)",
        params![name, path],
    ).map_err(|e| e.to_string())?;
    
    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn source_add_webdav(db_state: State<'_, DbState>, url: String, name: String, username: Option<String>, password: Option<String>) -> Result<i64, String> {
    let _trace = ipc_trace!("source_add_webdav");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    
    // Test connection
    let webdav = crate::services::webdav::WebdavClient::new(url.clone(), username.clone(), password.clone());
    webdav.propfind("/").map_err(|e| format!("Failed to connect to WebDAV: {}", e))?;

    let cred = if let (Some(u), Some(p)) = (&username, &password) {
        Some(format!("{}:{}", u, p))
    } else if let Some(u) = &username {
        Some(u.clone())
    } else {
        None
    };

    conn.execute(
        "INSERT INTO sources (name, kind, root_uri, credential_ref) VALUES (?1, 'webdav', ?2, ?3)",
        params![name, url, cred],
    ).map_err(|e| e.to_string())?;
    
    let id = conn.last_insert_rowid();
    Ok(id)
}


#[tauri::command]
pub fn source_scan(app: tauri::AppHandle, db_state: State<'_, DbState>, source_id: i64) -> Result<(), String> {
    let _trace = ipc_trace!("source_scan");
    let (kind, path, credential) = {
        let conn = db_state.db.get().map_err(|e| e.to_string())?;
        let (k, r, c): (String, String, Option<String>) = conn.query_row(
            "SELECT kind, root_uri, credential_ref FROM sources WHERE id = ?1",
            params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|e| e.to_string())?;
        (k, r, c)
    };

    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    std::thread::spawn(move || {
        if kind == "local" {
            crate::services::scanner::scan_local_directory(app, source_id, &PathBuf::from(path), &app_dir);
        } else if kind == "webdav" {
            // For now, assume config_json or credential_ref stores password. 
            // In a real app we'd fetch username from config_json or format it. Let's assume username is extracted or empty.
            // Actually, we can store "user:pass" in credential_ref for simplicity.
            let mut username = None;
            let mut password = None;
            if let Some(cred) = credential {
                let parts: Vec<&str> = cred.splitn(2, ':').collect();
                if parts.len() == 2 {
                    username = Some(parts[0].to_string());
                    password = Some(parts[1].to_string());
                } else if parts.len() == 1 {
                    username = Some(parts[0].to_string());
                }
            }
            crate::services::scanner::scan_webdav_directory(app, source_id, path, username, password, &app_dir);
        }
    });
    
    Ok(())
}

#[tauri::command]
pub fn source_list(db_state: State<'_, DbState>) -> Result<Vec<crate::models::Source>, String> {
    let _trace = ipc_trace!("source_list");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    
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
    let _trace = ipc_trace!("source_remove");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;

    // 在单事务中完成删除 + 清理孤儿，任一步失败整体回滚
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // 删除来源（PRAGMA foreign_keys = ON，会 CASCADE 删除 media_files）
    tx.execute("DELETE FROM sources WHERE id = ?1", rusqlite::params![source_id])
        .map_err(|e| e.to_string())?;

    // 清理孤儿 track：没有任何媒体文件再指向它的歌曲
    tx.execute(
        "DELETE FROM tracks WHERE id NOT IN (SELECT track_id FROM media_files WHERE track_id IS NOT NULL)",
        [],
    ).map_err(|e| e.to_string())?;

    // 清理孤儿 album：没有任何歌曲归属的专辑
    tx.execute(
        "DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM tracks WHERE album_id IS NOT NULL)",
        [],
    ).map_err(|e| e.to_string())?;

    // 清理孤儿 artist：
    //   - 既不是任何专辑的 album_artist
    //   - 也不在 track_artists 关系表里出现
    // 注意：tracks 表本身没有 artist_id 列（多对多关系在 track_artists 表），
    //       之前的实现误用了 tracks.artist_id，导致该语句必然失败并被静默吞掉。
    tx.execute(
        "DELETE FROM artists WHERE id NOT IN (
            SELECT album_artist_id FROM albums WHERE album_artist_id IS NOT NULL
            UNION
            SELECT artist_id FROM track_artists
        )",
        [],
    ).map_err(|e| e.to_string())?;

    // 重新校准冗余统计字段。
    // 上面 CASCADE 删除了大量 track_artists / tracks / albums 行，
    // 但 albums.track_count / artists.track_count / artists.album_count 这些冗余字段
    // 不会随 CASCADE 自动维护。这里做一次全量重算（几十毫秒，删来源是低频操作，可接受），
    // 保证后续列表查询能继续 O(1) 读冗余字段，不必每页都跑子查询。
    tx.execute_batch(
        "UPDATE albums SET track_count = (
            SELECT COUNT(*) FROM tracks t WHERE t.album_id = albums.id
        );
        UPDATE artists SET track_count = (
            SELECT COUNT(DISTINCT ta.track_id) FROM track_artists ta WHERE ta.artist_id = artists.id
        );
        UPDATE artists SET album_count = (
            SELECT COUNT(*) FROM albums al WHERE al.album_artist_id = artists.id
        );"
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn library_get_tracks(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_tracks");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_tracks_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_albums(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<AlbumDTO>, String> {
    let _trace = ipc_trace!("library_get_albums");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_albums_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artists(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<Vec<ArtistDTO>, String> {
    let _trace = ipc_trace!("library_get_artists");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_artists_paginated(&conn, limit, offset, search_keyword).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_album_tracks(db_state: State<'_, DbState>, album_id: i64) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_album_tracks");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_album_tracks(&conn, album_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_albums(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<AlbumDTO>, String> {
    let _trace = ipc_trace!("library_get_artist_albums");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_artist_albums(&conn, artist_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_tracks(db_state: State<'_, DbState>, artist_id: i64, limit: u32, offset: u32) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_artist_tracks");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_artist_tracks(&conn, artist_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_artist_stats(db_state: State<'_, DbState>, artist_id: i64) -> Result<ArtistStatsDTO, String> {
    let _trace = ipc_trace!("library_get_artist_stats");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_artist_stats(&conn, artist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playback_play(playback_state: tauri::State<'_, PlaybackState>, db_state: tauri::State<'_, DbState>, media_file_id: i64) -> Result<Option<u64>, String> {
    let _trace = ipc_trace!("playback_play");
    // 通过 source 的 root_uri + media_files.relative_path 拼出真实物理路径。
    // 这样：① 路径语义清晰（relative_path 真的是相对路径）；
    //      ② 将来迁移 source 根目录或支持 WebDAV，只需调整 root_uri 的解释逻辑。
    let (path_buf, webdav_reader) = {
        let conn = db_state.db.get().map_err(|e| e.to_string())?;
        let (relative_path, root_uri, kind, cred, size): (String, String, String, Option<String>, i64) = conn.query_row(
            "SELECT mf.relative_path, s.root_uri, s.kind, s.credential_ref, mf.file_size
             FROM media_files mf JOIN sources s ON mf.source_id = s.id
             WHERE mf.id = ?1",
            params![media_file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).map_err(|e| e.to_string())?;

        if kind == "webdav" {
            let mut username = None;
            let mut password = None;
            if let Some(c) = cred {
                let parts: Vec<&str> = c.splitn(2, ':').collect();
                if parts.len() == 2 {
                    username = Some(parts[0].to_string());
                    password = Some(parts[1].to_string());
                } else if parts.len() == 1 {
                    username = Some(parts[0].to_string());
                }
            }
            let webdav = crate::services::webdav::WebdavClient::new(root_uri.clone(), username, password);
            
            // relative_path is a relative path (e.g., Music/song.mp3 or Music\song.mp3).
            // We need to resolve it against root_uri (the base URL).
            let base_str = if root_uri.ends_with('/') { root_uri.clone() } else { format!("{}/", root_uri) };
            let base = reqwest::Url::parse(&base_str).unwrap();
            let relative_url_path = relative_path.replace('\\', "/");
            let file_url = base.join(&relative_url_path).unwrap().to_string();
            
            let http_reader = crate::services::webdav::HttpRangeReader::new(&webdav, file_url, size as u64);
            (None, Some(http_reader))
        } else {
            (Some(PathBuf::from(&root_uri).join(relative_path)), None)
        }
    };

    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    let duration = if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.play_stream(buffered_reader)?
    } else if let Some(path) = path_buf {
        manager.play_file(&path)?
    } else {
        return Err("No playable source found".to_string());
    };

    Ok(duration)
}

#[tauri::command]
pub fn playback_pause(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let _trace = ipc_trace!("playback_pause");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.pause();
    Ok(())
}

#[tauri::command]
pub fn playback_resume(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let _trace = ipc_trace!("playback_resume");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.resume();
    Ok(())
}

#[tauri::command]
pub fn playback_stop(playback_state: State<'_, PlaybackState>) -> Result<(), String> {
    let _trace = ipc_trace!("playback_stop");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.stop();
    Ok(())
}

#[tauri::command]
pub fn playback_set_volume(playback_state: State<'_, PlaybackState>, volume: f32) -> Result<(), String> {
    let _trace = ipc_trace!("playback_set_volume");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.set_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn playback_get_pos(playback_state: State<'_, PlaybackState>) -> Result<u64, String> {
    let _trace = ipc_trace!("playback_get_pos");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_pos())
}

#[tauri::command]
pub fn playback_seek(playback_state: State<'_, PlaybackState>, position_ms: u64) -> Result<(), String> {
    let _trace = ipc_trace!("playback_seek");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    manager.try_seek(position_ms)
}

#[tauri::command]
pub fn library_toggle_favorite(db_state: State<'_, DbState>, track_id: i64, is_favorite: bool) -> Result<(), String> {
    let _trace = ipc_trace!("library_toggle_favorite");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::toggle_favorite(&conn, track_id, is_favorite).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_create_playlist(db_state: State<'_, DbState>, name: String, description: Option<String>) -> Result<i64, String> {
    let _trace = ipc_trace!("library_create_playlist");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::create_playlist(&conn, &name, description.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_playlists(db_state: State<'_, DbState>) -> Result<Vec<PlaylistDTO>, String> {
    let _trace = ipc_trace!("library_get_playlists");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_playlists(&conn).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
pub fn library_add_to_playlist(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let _trace = ipc_trace!("library_add_to_playlist");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::add_to_playlist(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_playlist_tracks(db_state: State<'_, DbState>, playlist_id: i64) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_playlist_tracks");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_playlist_tracks(&conn, playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_record_play(db_state: State<'_, DbState>, track_id: i64, duration_ms: i64) -> Result<(), String> {
    let _trace = ipc_trace!("library_record_play");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::record_play(&conn, track_id, duration_ms).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_recently_played(db_state: State<'_, DbState>, limit: u32) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_recently_played");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_recently_played(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_favorite_tracks(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_favorite_tracks");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_favorite_tracks(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_lyrics(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<String>, String> {
    let _trace = ipc_trace!("library_get_lyrics");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    let lyr: Option<String> = conn.query_row(
        "SELECT content FROM lyrics WHERE track_id = ?1 LIMIT 1",
        params![track_id],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    Ok(lyr)
}

#[tauri::command]
pub fn library_get_track_file_info(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<crate::models::TrackFileInfoDTO>, String> {
    let _trace = ipc_trace!("library_get_track_file_info");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("
        SELECT id, relative_path, file_size, duration_ms, bitrate, sample_rate, bit_depth, channels, file_ext
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
    let _trace = ipc_trace!("library_delete_playlist");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::delete_playlist(&conn, playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_remove_playlist_item(db_state: State<'_, DbState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let _trace = ipc_trace!("library_remove_playlist_item");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::remove_playlist_item(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_save_play_queue(db_state: State<'_, DbState>, track_ids: Vec<i64>) -> Result<(), String> {
    let _trace = ipc_trace!("library_save_play_queue");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::save_play_queue(&conn, &track_ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_play_queue(db_state: State<'_, DbState>) -> Result<Vec<TrackDTO>, String> {
    let _trace = ipc_trace!("library_get_play_queue");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::get_play_queue(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_cache_size(app: tauri::AppHandle) -> Result<u64, String> {
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
pub fn library_clear_cache(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<(), String> {
    let _trace = ipc_trace!("library_clear_cache");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if artworks_dir.exists() {
        let _ = std::fs::remove_dir_all(&artworks_dir);
        let _ = std::fs::create_dir_all(&artworks_dir);
    }
    
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
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
) -> Result<crate::models::FolderContentsResult, String> {
    let _trace = ipc_trace!("library_get_folder_contents");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;

    // 如果未指定 folder_path，则使用该 source 对应的本地根路径
    let real_path = if let Some(p) = folder_path {
        std::path::PathBuf::from(p)
    } else {
        let root_uri: String = conn.query_row(
            "SELECT root_uri FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        std::path::PathBuf::from(root_uri)
    };

    LibraryService::get_folder_contents(&conn, source_id, &real_path, limit, offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_add_folder_to_playlist(db_state: State<'_, DbState>, source_id: i64, folder_path: String, playlist_id: i64) -> Result<(), String> {
    let _trace = ipc_trace!("library_add_folder_to_playlist");
    let conn = db_state.db.get().map_err(|e| e.to_string())?;
    LibraryService::add_folder_to_playlist(&conn, playlist_id, source_id, &folder_path).map_err(|e| e.to_string())
}

/// 询问当前播放是否已经结束（队列为空或自然播完）。
/// 前端在时长未知的情况下也能据此自动切下一首。
#[tauri::command]
pub fn playback_is_finished(playback_state: State<'_, PlaybackState>) -> Result<bool, String> {
    let _trace = ipc_trace!("playback_is_finished");
    let manager = playback_state.manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.is_finished())
}
