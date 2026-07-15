use crate::error::AppError;
use tauri::{State, Manager};
use crate::db::DbState;
use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO, ArtistListResult};
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
pub fn library_get_artists(db_state: State<'_, DbState>, limit: u32, offset: u32, search_keyword: Option<String>) -> Result<ArtistListResult, AppError> {
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
pub fn library_get_album_by_id(db_state: State<'_, DbState>, album_id: i64) -> Result<Option<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_album_by_id");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_by_id(&conn, album_id).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_by_id(db_state: State<'_, DbState>, artist_id: i64) -> Result<Option<ArtistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_by_id");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_by_id(&conn, artist_id).map_err(|e| e.into())
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
pub fn library_get_favorite_albums(db_state: State<'_, DbState>) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_albums");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_favorite_albums(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_favorite_artists(db_state: State<'_, DbState>) -> Result<Vec<ArtistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_artists");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_favorite_artists(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_toggle_favorite_album(db_state: State<'_, DbState>, album_id: i64, is_favorite: bool) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite_album");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::toggle_favorite_album(&conn, album_id, is_favorite).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_toggle_favorite_artist(db_state: State<'_, DbState>, artist_id: i64, is_favorite: bool) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite_artist");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::toggle_favorite_artist(&conn, artist_id, is_favorite).map_err(|e| e.into())
}

#[tauri::command]
pub async fn library_get_lyrics(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<String>, AppError> {
    let _trace = ipc_trace!("library_get_lyrics");
    
    // First, check DB
    let local_lyrics = {
        let conn = db_state.db.get()?;
        use rusqlite::OptionalExtension;
        let lyr: Option<String> = conn.query_row(
            "SELECT content FROM lyrics WHERE track_id = ?1 LIMIT 1",
            params![track_id],
            |row| row.get(0),
        ).optional()?;
        lyr
    };

    if local_lyrics.is_some() {
        return Ok(local_lyrics);
    }

    // Not found in DB, try to download from LRCLIB
    let (title, artist, album, duration_sec): (String, Option<String>, Option<String>, Option<u32>) = {
        let conn = db_state.db.get()?;
        let mut stmt = conn.prepare("
            SELECT 
                t.title,
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position),
                (SELECT title FROM albums WHERE id = t.album_id),
                (SELECT duration_ms FROM media_files WHERE track_id = t.id LIMIT 1)
            FROM tracks t WHERE t.id = ?1
        ")?;
        use rusqlite::OptionalExtension;
        let row = stmt.query_row(params![track_id], |row| {
            let duration_ms: Option<u32> = row.get(3)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                duration_ms.map(|ms| ms / 1000)
            ))
        }).optional()?;
        if let Some(r) = row { r } else { return Ok(None); }
    };

    // Prepare URL
    let mut url = url::Url::parse("https://lrclib.net/api/get").unwrap();
    url.query_pairs_mut().append_pair("track_name", &title);
    if let Some(a) = artist { url.query_pairs_mut().append_pair("artist_name", &a); }
    if let Some(al) = album { url.query_pairs_mut().append_pair("album_name", &al); }
    if let Some(d) = duration_sec { url.query_pairs_mut().append_pair("duration", &d.to_string()); }

    let client = reqwest::Client::builder()
        .user_agent("LumoMusicPlayer/1.0.0")
        .build()
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    #[derive(serde::Deserialize)]
    struct LrclibResponse {
        #[serde(rename = "syncedLyrics")]
        synced_lyrics: Option<String>,
        #[serde(rename = "plainLyrics")]
        plain_lyrics: Option<String>,
    }

    let result = resp.json::<LrclibResponse>()
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let fetched_lyrics = result.synced_lyrics.or(result.plain_lyrics);

    if let Some(ref l) = fetched_lyrics {
        let conn = db_state.db.get()?;
        let format = if l.contains("[00:") { "lrc" } else { "plain" };
        let synced = if format == "lrc" { 1 } else { 0 };
        let _ = conn.execute(
            "INSERT INTO lyrics (track_id, format, synced, content, source) VALUES (?1, ?2, ?3, ?4, 'lrclib')",
            params![track_id, format, synced, l]
        );
    }

    Ok(fetched_lyrics)
}

#[tauri::command]
pub fn library_get_track_file_info(db_state: State<'_, DbState>, track_id: i64) -> Result<Option<crate::models::TrackFileInfoDTO>, AppError> {
    let _trace = ipc_trace!("library_get_track_file_info");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get()?;
    let mut stmt = conn.prepare("
        SELECT 
            mf.id, s.id, mf.track_id, s.root_uri || '/' || mf.relative_path, mf.relative_path, 
            mf.file_name, mf.file_ext, mf.file_size, mf.modified_at, mf.duration_ms, mf.bitrate, mf.sample_rate, 
            mf.bit_depth, mf.channels, mf.file_ext, s.kind
        FROM media_files mf
        JOIN sources s ON s.id = mf.source_id
        WHERE mf.track_id = ?1
        LIMIT 1
    ")?;
    
    let info = stmt.query_row(params![track_id], |row| {
        Ok(crate::models::TrackFileInfoDTO {
            id: row.get(0)?,
            source_id: row.get(1)?,
            track_id: row.get(2).unwrap_or(0),
            path: row.get(3)?,
            relative_path: row.get(4)?,
            file_name: row.get(5)?,
            file_ext: row.get(6)?,
            file_size: row.get(7)?,
            modified_at: row.get(8)?,
            duration_ms: row.get(9)?,
            bitrate: row.get(10)?,
            sample_rate: row.get(11)?,
            bit_depth: row.get(12)?,
            channels: row.get(13)?,
            format: row.get(14)?,
            source_kind: row.get(15)?,
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

#[tauri::command]
pub fn library_get_folder_children(
    db_state: State<'_, DbState>,
    source_id: i64,
    folder_path: Option<String>,
) -> Result<crate::models::FolderChildrenResult, AppError> {
    let _trace = ipc_trace!("library_get_folder_children");
    let conn = db_state.db.get()?;

    let root_uri: String = conn.query_row(
        "SELECT root_uri FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get(0),
    )?;
    let source_root = std::path::PathBuf::from(&root_uri);
    let real_path = folder_path
        .map(|p| source_root.join(&p))
        .unwrap_or_else(|| source_root.clone());

    crate::repositories::track_repo::TrackRepo::get_folder_children(&conn, source_id, &real_path, &source_root)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_folder_tracks(
    db_state: State<'_, DbState>,
    source_id: i64,
    folder_path: String,
    limit: u32,
    offset: u32,
) -> Result<crate::models::FolderTracksResult, AppError> {
    let _trace = ipc_trace!("library_get_folder_tracks");
    let conn = db_state.db.get()?;

    let root_uri: String = conn.query_row(
        "SELECT root_uri FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get(0),
    )?;
    let source_root = std::path::PathBuf::from(&root_uri);
    let real_path = source_root.join(&folder_path);

    crate::repositories::track_repo::TrackRepo::get_folder_tracks_recursive(&conn, source_id, &real_path, &source_root, limit, offset)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_counts(
    db_state: State<'_, DbState>,
) -> Result<crate::models::LibraryCounts, AppError> {
    let _trace = ipc_trace!("library_get_counts");
    let conn = db_state.db.get()?;

    let counts: crate::models::LibraryCounts = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM tracks),
            (SELECT COUNT(*) FROM favorite_tracks),
            (SELECT COUNT(*) FROM favorite_albums),
            (SELECT COUNT(*) FROM favorite_artists),
            (SELECT COUNT(*) FROM tracks WHERE last_played_at IS NOT NULL)",
        [],
        |row| Ok(crate::models::LibraryCounts {
            tracks: row.get(0)?,
            favorite_tracks: row.get(1)?,
            favorite_albums: row.get(2)?,
            favorite_artists: row.get(3)?,
            recently_played: row.get(4)?,
        }),
    )?;

    Ok(counts)
}

#[tauri::command]
pub async fn library_fetch_missing_album_cover(app: tauri::AppHandle, db_state: State<'_, DbState>, album_id: i64) -> Result<Option<i64>, AppError> {
    let _trace = ipc_trace!("library_fetch_missing_album_cover");
    
    // 1. Get album info
    let (album_title, artist_name): (String, Option<String>) = {
        let conn = db_state.db.get()?;
        let mut stmt = conn.prepare("
            SELECT al.title, (SELECT name FROM artists WHERE id = (SELECT artist_id FROM track_artists WHERE track_id = t.id LIMIT 1))
            FROM albums al
            LEFT JOIN tracks t ON t.album_id = al.id
            WHERE al.id = ?1 LIMIT 1
        ")?;
        use rusqlite::OptionalExtension;
        let row = stmt.query_row(params![album_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).optional()?;
        if let Some(r) = row { r } else { return Ok(None); }
    };

    // 2. Query iTunes
    let mut term = album_title.clone();
    if let Some(a) = &artist_name {
        term = format!("{} {}", a, term);
    }
    
    let mut url = url::Url::parse("https://itunes.apple.com/search").unwrap();
    url.query_pairs_mut().append_pair("term", &term);
    url.query_pairs_mut().append_pair("entity", "album");
    url.query_pairs_mut().append_pair("limit", "1");

    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if !resp.status().is_success() { return Ok(None); }

    #[derive(serde::Deserialize)]
    struct ItunesResponse {
        results: Vec<ItunesResult>,
    }
    #[derive(serde::Deserialize)]
    struct ItunesResult {
        #[serde(rename = "artworkUrl100")]
        artwork_url_100: Option<String>,
    }

    let result = resp.json::<ItunesResponse>().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if result.results.is_empty() { return Ok(None); }
    let artwork_url = if let Some(url) = &result.results[0].artwork_url_100 {
        url.replace("100x100bb", "600x600bb")
    } else {
        return Ok(None);
    };

    // 3. Download image
    let img_resp = client.get(&artwork_url).send().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if !img_resp.status().is_success() { return Ok(None); }
    
    let mime_type = img_resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
    let bytes = img_resp.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // 4. Save to db
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());

    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if !artworks_dir.exists() {
        let _ = std::fs::create_dir_all(&artworks_dir);
    }
    let ext = match mime_type.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    };
    let cache_path = artworks_dir.join(format!("{}.{}", hash, ext));
    if !cache_path.exists() {
        let _ = std::fs::write(&cache_path, &bytes);
    }
    
    let thumbnail_blob = crate::services::library::LibraryService::generate_thumbnail(&bytes);

    let conn = db_state.db.get()?;
    use rusqlite::OptionalExtension;
    // Check if hash exists
    let existing_id: Option<i64> = conn.query_row(
        "SELECT id FROM artwork WHERE content_hash = ?1",
        params![hash],
        |row| row.get(0),
    ).optional()?;
    
    let artwork_id = if let Some(id) = existing_id {
        id
    } else {
        conn.execute(
            "INSERT INTO artwork (cache_path, mime_type, content_hash, thumbnail_blob) VALUES (?1, ?2, ?3, ?4)",
            params![cache_path.to_string_lossy().to_string(), mime_type, hash, thumbnail_blob],
        )?;
        conn.last_insert_rowid()
    };

    conn.execute(
        "UPDATE albums SET cover_artwork_id = ?1 WHERE id = ?2",
        params![artwork_id, album_id],
    )?;

    Ok(Some(artwork_id))
}

#[tauri::command]
pub async fn library_fetch_missing_artist_cover(app: tauri::AppHandle, db_state: State<'_, DbState>, artist_id: i64) -> Result<Option<i64>, AppError> {
    let _trace = ipc_trace!("library_fetch_missing_artist_cover");
    
    // 1. Get artist info
    let artist_name: String = {
        let conn = db_state.db.get()?;
        let mut stmt = conn.prepare("SELECT name FROM artists WHERE id = ?1 LIMIT 1")?;
        use rusqlite::OptionalExtension;
        let row = stmt.query_row(params![artist_id], |row| row.get(0)).optional()?;
        if let Some(r) = row { r } else { return Ok(None); }
    };

    // 2. Query iTunes
    let mut url = url::Url::parse("https://itunes.apple.com/search").unwrap();
    url.query_pairs_mut().append_pair("term", &artist_name);
    url.query_pairs_mut().append_pair("entity", "album"); // artist entity often lacks image, so we use their top album
    url.query_pairs_mut().append_pair("limit", "1");

    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if !resp.status().is_success() { return Ok(None); }

    #[derive(serde::Deserialize)]
    struct ItunesResponse {
        results: Vec<ItunesResult>,
    }
    #[derive(serde::Deserialize)]
    struct ItunesResult {
        #[serde(rename = "artworkUrl100")]
        artwork_url_100: Option<String>,
    }

    let result = resp.json::<ItunesResponse>().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if result.results.is_empty() { return Ok(None); }
    let artwork_url = if let Some(url) = &result.results[0].artwork_url_100 {
        url.replace("100x100bb", "600x600bb")
    } else {
        return Ok(None);
    };

    // 3. Download image
    let img_resp = client.get(&artwork_url).send().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    if !img_resp.status().is_success() { return Ok(None); }
    
    let mime_type = img_resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
    let bytes = img_resp.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // 4. Save to db
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());

    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if !artworks_dir.exists() {
        let _ = std::fs::create_dir_all(&artworks_dir);
    }
    let ext = match mime_type.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    };
    let cache_path = artworks_dir.join(format!("{}.{}", hash, ext));
    if !cache_path.exists() {
        let _ = std::fs::write(&cache_path, &bytes);
    }
    
    let thumbnail_blob = crate::services::library::LibraryService::generate_thumbnail(&bytes);

    let conn = db_state.db.get()?;
    use rusqlite::OptionalExtension;
    
    // Check if hash exists
    let existing_id: Option<i64> = conn.query_row(
        "SELECT id FROM artwork WHERE content_hash = ?1",
        params![hash],
        |row| row.get(0),
    ).optional()?;
    
    let artwork_id = if let Some(id) = existing_id {
        id
    } else {
        conn.execute(
            "INSERT INTO artwork (cache_path, mime_type, content_hash, thumbnail_blob) VALUES (?1, ?2, ?3, ?4)",
            params![cache_path.to_string_lossy().to_string(), mime_type, hash, thumbnail_blob],
        )?;
        conn.last_insert_rowid()
    };

    conn.execute(
        "UPDATE artists SET avatar_artwork_id = ?1 WHERE id = ?2",
        params![artwork_id, artist_id],
    )?;

    Ok(Some(artwork_id))
}

/// 智能歌单查询 command。
/// 通过 `kind` 参数选择不同预设规则，返回对应的 TrackDTO 列表：
/// - `"most_played"`：按 play_count 降序，返回播放次数最多的 Top N 歌曲
///
/// `limit` 默认为 50，前端可传入自定义值。
#[tauri::command]
pub fn library_get_smart_playlist(
    db_state: State<'_, DbState>,
    kind: String,
    limit: Option<i64>,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_smart_playlist");
    let conn = db_state.db.get()?;
    let limit = limit.unwrap_or(50);
    match kind.as_str() {
        "most_played" => {
            crate::repositories::track_repo::TrackRepo::get_most_played_tracks(&conn, limit)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        _ => Err(AppError::Internal(format!("Unknown smart playlist kind: {}", kind))),
    }
}

/// 获取某一首歌的所有可用物理文件版本（用于多音源版本切换 UI）
#[tauri::command]
pub fn library_get_track_versions(
    db_state: State<'_, DbState>,
    track_id: i64,
) -> Result<Vec<crate::models::TrackFileInfoDTO>, AppError> {
    let _trace = ipc_trace!("library_get_track_versions");
    let conn = db_state.db.get()?;
    
    // 查询所有属于该 track 的 media_files，按 file_priority_score 降序排序
    let mut stmt = conn.prepare("
        SELECT 
            mf.id, s.id as source_id, mf.track_id, s.root_uri || '/' || mf.relative_path as path, mf.relative_path, 
            mf.file_name, mf.file_ext, mf.file_size, mf.modified_at, mf.duration_ms, mf.bitrate, mf.sample_rate, 
            mf.bit_depth, mf.channels, mf.file_ext as format, s.kind as source_kind
        FROM media_files mf
        JOIN sources s ON s.id = mf.source_id
        WHERE mf.track_id = ?1 AND mf.availability = 'available'
    ")?;
    
    use rusqlite::OptionalExtension;
    
    let rows = stmt.query_map(rusqlite::params![track_id], |row| {
        Ok(crate::models::TrackFileInfoDTO {
            id: row.get(0)?,
            source_id: row.get(1)?,
            track_id: row.get(2).unwrap_or(0),
            path: row.get(3)?,
            relative_path: row.get(4)?,
            file_name: row.get(5)?,
            file_ext: row.get(6)?,
            file_size: row.get(7)?,
            modified_at: row.get(8)?,
            duration_ms: row.get(9)?,
            bitrate: row.get(10)?,
            sample_rate: row.get(11)?,
            bit_depth: row.get(12)?,
            channels: row.get(13)?,
            format: row.get(14)?,
            source_kind: row.get(15)?,
        })
    })?;
    
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    
    // 在内存中排序（复用我们写好的 file_priority_score 逻辑）
    result.sort_by(|a, b| {
        let score_a = crate::services::file_priority::file_priority_score(&a.source_kind, &a.file_ext.clone().unwrap_or_default());
        let score_b = crate::services::file_priority::file_priority_score(&b.source_kind, &b.file_ext.clone().unwrap_or_default());
        score_b.cmp(&score_a) // 降序
    });
    
    Ok(result)
}
