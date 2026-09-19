use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::models::{AlbumDTO, ArtistDTO, ArtistListResult, ArtistStatsDTO, PlaylistDTO, TrackDTO};
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
// For storing PlaybackManager state

/// lrclib 在线歌词请求超时（秒）。
/// 这个调用挂在切歌路径上，没有超时时慢速/黑洞化的第三方会把 IPC 卡死。
const LRCLIB_TIMEOUT_SECS: u64 = 15;

#[tauri::command(async)]
pub fn library_get_tracks(
    db_state: State<'_, DbState>,
    limit: u32,
    offset: u32,
    search_keyword: Option<String>,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_tracks_paginated(
        &conn,
        limit,
        offset,
        search_keyword,
    )
    .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_albums(
    db_state: State<'_, DbState>,
    limit: u32,
    offset: u32,
    search_keyword: Option<String>,
) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_albums");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_albums_paginated(
        &conn,
        limit,
        offset,
        search_keyword,
    )
    .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_album_count(
    db_state: State<'_, DbState>,
    search_keyword: Option<String>,
) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_get_album_count");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_count(&conn, search_keyword)
        .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_artists(
    db_state: State<'_, DbState>,
    limit: u32,
    offset: u32,
    search_keyword: Option<String>,
) -> Result<ArtistListResult, AppError> {
    let _trace = ipc_trace!("library_get_artists");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artists_paginated(
        &conn,
        limit,
        offset,
        search_keyword,
    )
    .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_album_tracks(
    db_state: State<'_, DbState>,
    album_id: i64,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_album_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_tracks(&conn, album_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_album_by_id(
    db_state: State<'_, DbState>,
    album_id: i64,
) -> Result<Option<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_album_by_id");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_album_by_id(&conn, album_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_by_id(
    db_state: State<'_, DbState>,
    artist_id: i64,
) -> Result<Option<ArtistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_by_id");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_by_id(&conn, artist_id)
        .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_artist_albums(
    db_state: State<'_, DbState>,
    artist_id: i64,
    limit: u32,
    offset: u32,
) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_albums");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_albums(&conn, artist_id, limit, offset)
        .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_artist_album_count(
    db_state: State<'_, DbState>,
    artist_id: i64,
) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_get_artist_album_count");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_album_count(&conn, artist_id)
        .map_err(|e| e.into())
}

#[tauri::command(async)]
pub fn library_get_artist_tracks(
    db_state: State<'_, DbState>,
    artist_id: i64,
    limit: u32,
    offset: u32,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_artist_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_tracks(&conn, artist_id, limit, offset)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_artist_stats(
    db_state: State<'_, DbState>,
    artist_id: i64,
) -> Result<ArtistStatsDTO, AppError> {
    let _trace = ipc_trace!("library_get_artist_stats");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_artist_stats(&conn, artist_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_toggle_favorite(
    db_state: State<'_, DbState>,
    track_id: i64,
    is_favorite: bool,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::toggle_favorite(&conn, track_id, is_favorite)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_create_playlist(
    db_state: State<'_, DbState>,
    name: String,
    description: Option<String>,
) -> Result<i64, AppError> {
    let _trace = ipc_trace!("library_create_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::create_playlist(
        &conn,
        &name,
        description.as_deref(),
    )
    .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_playlists(db_state: State<'_, DbState>) -> Result<Vec<PlaylistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_playlists");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::get_playlists(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_add_to_playlist(
    db_state: State<'_, DbState>,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_add_to_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::add_to_playlist(&conn, playlist_id, track_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_playlist_tracks(
    db_state: State<'_, DbState>,
    playlist_id: i64,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_playlist_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::get_playlist_tracks(&conn, playlist_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_record_play(
    db_state: State<'_, DbState>,
    track_id: i64,
    duration_ms: i64,
    media_file_id: Option<i64>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_record_play");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::record_play(
        &conn,
        track_id,
        duration_ms,
        media_file_id,
    )
    .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_recently_played(
    db_state: State<'_, DbState>,
    limit: u32,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_recently_played");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_recently_played(&conn, limit)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_favorite_tracks(
    db_state: State<'_, DbState>,
) -> Result<Vec<TrackDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_tracks");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::get_favorite_tracks(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_favorite_albums(
    db_state: State<'_, DbState>,
) -> Result<Vec<AlbumDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_albums");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::get_favorite_albums(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_get_favorite_artists(
    db_state: State<'_, DbState>,
) -> Result<Vec<ArtistDTO>, AppError> {
    let _trace = ipc_trace!("library_get_favorite_artists");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::get_favorite_artists(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub fn library_toggle_favorite_album(
    db_state: State<'_, DbState>,
    album_id: i64,
    is_favorite: bool,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite_album");
    let conn = db_state.db.get()?;
    crate::repositories::album_repo::AlbumRepo::toggle_favorite_album(&conn, album_id, is_favorite)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_toggle_favorite_artist(
    db_state: State<'_, DbState>,
    artist_id: i64,
    is_favorite: bool,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_toggle_favorite_artist");
    let conn = db_state.db.get()?;
    crate::repositories::artist_repo::ArtistRepo::toggle_favorite_artist(
        &conn,
        artist_id,
        is_favorite,
    )
    .map_err(|e| e.into())
}

/// 获取曲库数据库文件总大小（含 WAL/SHM），供移动端存储占用展示。
#[tauri::command]
pub fn storage_get_db_size(app: tauri::AppHandle) -> Result<u64, AppError> {
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let mut total = 0u64;
    for name in ["lumo.sqlite", "lumo.sqlite-wal", "lumo.sqlite-shm"] {
        if let Ok(md) = std::fs::metadata(app_dir.join(name)) {
            total += md.len();
        }
    }
    Ok(total)
}

#[tauri::command]
pub async fn library_get_lyrics(
    db_state: State<'_, DbState>,
    track_id: i64,
    allow_online: Option<bool>,
) -> Result<Option<String>, AppError> {
    let _trace = ipc_trace!("library_get_lyrics");

    // 本地歌词查询（v1.8.1 重写）：同步(lrc)优先 → lrclib 来源 → 主文件版本 → 最早行。
    // 此前 `LIMIT 1` 无排序，多文件版本的重复行中取哪行是不确定的。
    use rusqlite::OptionalExtension;
    let primary_file_id: Option<i64> = {
        let conn = db_state.db.get()?;
        conn.query_row(
            "SELECT primary_file_id FROM tracks WHERE id = ?1",
            params![track_id],
            |row| row.get(0),
        )
        .optional()?
    };
    // (id, content, format, synced)
    let local_lyrics: Option<(i64, String, String, i64)> = {
        let conn = db_state.db.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, content, format, synced FROM lyrics WHERE track_id = ?1
             ORDER BY (synced = 1) DESC, (media_file_id = ?2) DESC, id ASC LIMIT 1",
        )?;
        stmt.query_row(params![track_id, primary_file_id], |row| {
            Ok::<(i64, String, String, i64), rusqlite::Error>((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        })
        .optional()?
    };

    // 本地已是同步歌词（嵌入 lrc 质量足够）→ 直接返回
    if let Some((_, content, _, synced)) = &local_lyrics {
        if *synced == 1 {
            return Ok(Some(content.clone()));
        }
    }

    // P0-07 在线元数据隐私：默认拒绝——调用方未显式授权时不向 LRCLIB 发送任何数据，
    // 返回本地结果（可能为纯文本或 None）。授权开关由设置页「隐私」分区控制。
    if allow_online != Some(true) {
        return Ok(local_lyrics.map(|(_, c, _, _)| c));
    }

    // Not found in DB, try to download from LRCLIB
    let (title, artist, album, duration_sec): (
        String,
        Option<String>,
        Option<String>,
        Option<u32>,
    ) = {
        let conn = db_state.db.get()?;
        let mut stmt = conn.prepare("
            SELECT 
                t.title,
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position),
                (SELECT title FROM albums WHERE id = t.album_id),
                (SELECT duration_ms FROM media_files WHERE track_id = t.id ORDER BY CASE WHEN id = t.primary_file_id THEN 0 ELSE 1 END, id LIMIT 1)
            FROM tracks t WHERE t.id = ?1
        ")?;
        use rusqlite::OptionalExtension;
        let row = stmt
            .query_row(params![track_id], |row| {
                let duration_ms: Option<u32> = row.get(3)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    duration_ms.map(|ms| ms / 1000),
                ))
            })
            .optional()?;
        if let Some(r) = row {
            r
        } else {
            return Ok(None);
        }
    };

    // Prepare URL
    let mut url = url::Url::parse("https://lrclib.net/api/get").unwrap();
    url.query_pairs_mut().append_pair("track_name", &title);
    if let Some(a) = artist {
        url.query_pairs_mut().append_pair("artist_name", &a);
    }
    if let Some(al) = album {
        url.query_pairs_mut().append_pair("album_name", &al);
    }
    if let Some(d) = duration_sec {
        url.query_pairs_mut()
            .append_pair("duration", &d.to_string());
    }

    // 必须设超时：这是播放路径上的同步 IPC，慢速/黑洞化的 lrclib 会让切歌卡住
    // 联网行为: §2A —— lrclib 在线歌词，地址见 NETWORK_BEHAVIOR.md §2A
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(LRCLIB_TIMEOUT_SECS))
        .user_agent("LumoMusicPlayer/1.0.0")
        .build()
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;

    // 联网行为: §2A
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;

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

    let result = resp
        .json::<LrclibResponse>()
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;

    let fetched_lyrics = result.synced_lyrics.or(result.plain_lyrics);

    if let Some(ref l) = fetched_lyrics {
        let conn = db_state.db.get()?;
        let format = if l.contains("[00:") { "lrc" } else { "plain" };
        let synced = if format == "lrc" { 1 } else { 0 };
        // 保存（V10 后 track_id 唯一，INSERT OR REPLACE 真正生效）：
        // - 本地为纯文本且拿到同步歌词 → 覆盖升级；
        // - 无本地行 → 直接插入。media_file_id 挂主文件版本，与扫描写入口径一致。
        // 在线也只拿到纯文本时保留本地（若有的话），不做无意义覆盖。
        let is_upgrade = format == "lrc";
        let should_save = local_lyrics.is_none() || is_upgrade;
        if should_save {
            let result = match primary_file_id {
                Some(pf) => conn.execute(
                    "INSERT OR REPLACE INTO lyrics (track_id, media_file_id, format, synced, content, source) VALUES (?1, ?2, ?3, ?4, ?5, 'lrclib')",
                    params![track_id, pf, format, synced, l]),
                None => conn.execute(
                    "INSERT INTO lyrics (track_id, media_file_id, format, synced, content, source) VALUES (?1, NULL, ?2, ?3, ?4, 'lrclib')",
                    params![track_id, format, synced, l]),
            };
            if let Err(e) = result {
                tracing::warn!("[lyrics] 保存 LRCLIB 歌词失败 track={}: {}", track_id, e);
            }
        }
    }

    // 在线未命中时回退本地纯文本（若有）
    Ok(fetched_lyrics.or_else(|| local_lyrics.map(|(_, c, _, _)| c)))
}

#[tauri::command]
pub fn library_get_track_file_info(
    db_state: State<'_, DbState>,
    track_id: i64,
) -> Result<Option<crate::models::TrackFileInfoDTO>, AppError> {
    let _trace = ipc_trace!("library_get_track_file_info");
    use rusqlite::OptionalExtension;
    let conn = db_state.db.get()?;

    // 主文件优先：LIMIT 1 必须返回 tracks.primary_file_id 指向的版本（P1-04），
    // 其余按本地优先 + id 排序兜底
    let mut stmt = conn.prepare("
        SELECT
            mf.id, s.id, mf.track_id, s.root_uri || '/' || mf.relative_path, mf.relative_path,
            mf.file_name, mf.file_ext, mf.file_size, mf.modified_at, mf.duration_ms, mf.bitrate, mf.sample_rate,
            mf.bit_depth, mf.channels, mf.file_ext, s.kind
        FROM media_files mf
        JOIN sources s ON s.id = mf.source_id
        JOIN tracks t ON t.id = mf.track_id
        WHERE mf.track_id = ?1
        ORDER BY CASE WHEN mf.id = t.primary_file_id THEN 0 ELSE 1 END,
                 CASE s.kind WHEN 'local' THEN 0 ELSE 1 END, mf.id
        LIMIT 1
    ")?;

    let info = stmt
        .query_row(params![track_id], |row| {
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
        })
        .optional()?;
    Ok(info)
}

#[tauri::command]
pub fn library_delete_playlist(
    db_state: State<'_, DbState>,
    playlist_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_delete_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::delete_playlist(&conn, playlist_id)
        .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_remove_playlist_item(
    db_state: State<'_, DbState>,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_remove_playlist_item");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::remove_playlist_item(
        &conn,
        playlist_id,
        track_id,
    )
    .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_save_play_queue(
    db_state: State<'_, DbState>,
    track_ids: Vec<i64>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_save_play_queue");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::save_play_queue(&conn, &track_ids)
        .map_err(|e| e.into())
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
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
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
pub fn library_clear_cache(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_clear_cache");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let artworks_dir = app_dir.join("artworks");
    if artworks_dir.exists() {
        let _ = std::fs::remove_dir_all(&artworks_dir);
        let _ = std::fs::create_dir_all(&artworks_dir);
    }

    let conn = db_state.db.get()?;
    // Keep artwork identities/references, but invalidate the materialised cache and
    // mark media for the next scan. Deleting artwork rows would make unchanged files
    // permanently lose their covers because incremental scanning skips them.
    let _ = conn.execute("UPDATE artwork SET thumbnail_blob = NULL", []);
    let _ = conn.execute(
        "UPDATE media_files SET modified_at = NULL, availability = 'offline'",
        [],
    );

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

    crate::repositories::track_repo::TrackRepo::get_folder_contents(
        &conn,
        source_id,
        &real_path,
        limit,
        offset.unwrap_or(0),
    )
    .map_err(|e| e.into())
}

#[tauri::command]
pub fn library_add_folder_to_playlist(
    db_state: State<'_, DbState>,
    source_id: i64,
    folder_path: String,
    playlist_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_add_folder_to_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::add_folder_to_playlist(
        &conn,
        playlist_id,
        source_id,
        &folder_path,
    )
    .map_err(|e| e.into())
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

    crate::repositories::track_repo::TrackRepo::get_folder_children(
        &conn,
        source_id,
        &real_path,
        &source_root,
    )
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

    crate::repositories::track_repo::TrackRepo::get_folder_tracks_recursive(
        &conn,
        source_id,
        &real_path,
        &source_root,
        limit,
        offset,
    )
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
        |row| {
            Ok(crate::models::LibraryCounts {
                tracks: row.get(0)?,
                favorite_tracks: row.get(1)?,
                favorite_albums: row.get(2)?,
                favorite_artists: row.get(3)?,
                recently_played: row.get(4)?,
            })
        },
    )?;

    Ok(counts)
}

/// 首页统计：一条 SQL 聚合曲库规模与收听行为（8 张统计卡的数据源）。
/// 「今日/近7天」口径见 models::LibraryStats 注释。
#[tauri::command]
pub fn library_get_stats(
    db_state: State<'_, DbState>,
) -> Result<crate::models::LibraryStats, AppError> {
    let _trace = ipc_trace!("library_get_stats");
    let conn = db_state.db.get()?;

    let stats: crate::models::LibraryStats = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM tracks) AS track_count,
            (SELECT COUNT(*) FROM albums) AS album_count,
            (SELECT COUNT(*) FROM artists) AS artist_count,
            (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history) AS total_listen_ms,
            (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history
             WHERE played_at >= datetime('now','localtime','start of day','utc')) AS today_listen_ms,
            (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history
             WHERE played_at >= datetime('now','localtime','start of day','-6 days','utc')) AS week_listen_ms,
            (SELECT COALESCE(SUM(play_count), 0) FROM tracks) AS total_play_count,
            (SELECT COUNT(*) FROM play_history
             WHERE played_at >= datetime('now','localtime','start of day','utc')) AS today_play_count,
            (SELECT COUNT(*) FROM playlists) AS playlist_count,
            (SELECT COUNT(*) FROM favorite_albums) AS favorite_album_count,
            (SELECT COUNT(*) FROM favorite_artists) AS favorite_artist_count,
            (SELECT COUNT(*) FROM favorite_tracks) AS favorite_track_count",
        [],
        |row| Ok(crate::models::LibraryStats {
            track_count: row.get(0)?,
            album_count: row.get(1)?,
            artist_count: row.get(2)?,
            total_listen_ms: row.get(3)?,
            today_listen_ms: row.get(4)?,
            week_listen_ms: row.get(5)?,
            total_play_count: row.get(6)?,
            today_play_count: row.get(7)?,
            playlist_count: row.get(8)?,
            favorite_album_count: row.get(9)?,
            favorite_artist_count: row.get(10)?,
            favorite_track_count: row.get(11)?,
        }),
    )?;

    Ok(stats)
}

/// 首页洞察：8 个查询（4 个歌曲榜 + 艺人榜 + 专辑榜 + 今日次数 + 上次听歌）
/// 一次 IPC 打包返回，首页开一次就好，避免逐个查询造成 IPC 拥堵。
///
/// async 化（第三轮）：Tauri v2 默认 sync 命令跑在主线程且串行执行，
/// 即使前端 Promise.all 也会排队。`#[tauri::command(async)]`（保持 `pub fn`，
/// 不用 `pub async fn`）把命令体移到 tokio blocking pool 并发执行，不阻塞其他 IPC。
#[tauri::command(async)]
pub fn library_get_insights(
    db_state: State<'_, DbState>,
) -> Result<crate::models::LibraryInsights, AppError> {
    let _trace = ipc_trace!("library_get_insights");
    let conn = db_state.db.get()?;
    use crate::repositories::{
        album_repo::AlbumRepo, artist_repo::ArtistRepo, track_repo::TrackRepo,
    };

    Ok(crate::models::LibraryInsights {
        top_played_tracks: TrackRepo::get_top_played_ranked(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        recent_played_tracks: TrackRepo::get_recent_play_ranked(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        recent_added_tracks: TrackRepo::get_recent_added_ranked(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        favorite_tracks: TrackRepo::get_favorite_ranked(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        top_played_artists: ArtistRepo::get_top_played_artists(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        top_played_albums: AlbumRepo::get_top_played_albums(&conn, 5)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        today_play_count: TrackRepo::get_today_play_count(&conn)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        last_played: TrackRepo::get_last_played(&conn)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    })
}

/// 批量添加歌曲到歌单（单事务，自动跳过已在歌单中的重复曲目）。
/// 返回 [成功添加数, 跳过的重复数]。
#[tauri::command]
pub fn library_add_tracks_to_playlist(
    db_state: State<'_, DbState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(usize, usize), AppError> {
    let _trace = ipc_trace!("library_add_tracks_to_playlist");
    let conn = db_state.db.get()?;
    crate::repositories::playlist_repo::PlaylistRepo::add_tracks_to_playlist(
        &conn,
        playlist_id,
        &track_ids,
    )
    .map_err(|e| e.into())
}

/// 批量设置/取消收藏（单事务）。
#[tauri::command]
pub fn library_set_favorite_batch(
    db_state: State<'_, DbState>,
    track_ids: Vec<i64>,
    is_favorite: bool,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_set_favorite_batch");
    let conn = db_state.db.get()?;
    crate::repositories::track_repo::TrackRepo::set_favorite_batch(&conn, &track_ids, is_favorite)
        .map_err(|e| e.into())
}

/// 启动数据包：一次 IPC 拿回 App.vue 启动所需的全部数据（遗留事项 2）。
/// 把启动 IPC 从 ~7 个降到 2 个（本命令 + source_list——凭据解析逻辑在 scanner 模块，
/// 保持独立）。async 化理由同其他 list-load 命令。
#[tauri::command(async)]
pub fn library_get_startup_bundle(
    db_state: State<'_, DbState>,
) -> Result<crate::models::StartupBundle, AppError> {
    let _trace = ipc_trace!("library_get_startup_bundle");
    let conn = db_state.db.get()?;
    use crate::repositories::{
        album_repo::AlbumRepo, artist_repo::ArtistRepo, playlist_repo::PlaylistRepo,
        track_repo::TrackRepo,
    };

    // 计数（与 library_get_counts 同一条 SQL）
    let counts: crate::models::LibraryCounts = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM tracks),
            (SELECT COUNT(*) FROM favorite_tracks),
            (SELECT COUNT(*) FROM favorite_albums),
            (SELECT COUNT(*) FROM favorite_artists),
            (SELECT COUNT(*) FROM tracks WHERE last_played_at IS NOT NULL)",
        [],
        |row| {
            Ok(crate::models::LibraryCounts {
                tracks: row.get(0)?,
                favorite_tracks: row.get(1)?,
                favorite_albums: row.get(2)?,
                favorite_artists: row.get(3)?,
                recently_played: row.get(4)?,
            })
        },
    )?;

    // 专辑网格第一页 30 条 / 艺人第一页 50 条——与前端 albumsPageSize / artistsLimit 一致，
    // 前端据此续接增量加载。缩略图/计数等参数与各自独立命令完全同形。
    let albums = AlbumRepo::get_albums_paginated(&conn, 30, 0, None)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let album_total =
        AlbumRepo::get_album_count(&conn, None).map_err(|e| AppError::Internal(e.to_string()))?;
    let ArtistListResult { artists, total } = ArtistRepo::get_artists_paginated(&conn, 50, 0, None)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(crate::models::StartupBundle {
        counts,
        playlists: PlaylistRepo::get_playlists(&conn)
            .map_err(|e| AppError::Internal(e.to_string()))?,
        albums,
        album_total,
        artists,
        artist_total: total,
        play_queue: TrackRepo::get_play_queue(&conn)
            .map_err(|e| AppError::Internal(e.to_string()))?,
    })
}

/// 专辑封面的真实拉取流程（网易云 → iTunes 兜底 + 缩略图 + 写库）。
/// 只在 tokio 后台任务中调用，不占 IPC channel（第七轮）。
/// 返回 (artwork_id, 缩略图 base64)——缩略图随事件下发，前端即时更新网格。
async fn fetch_album_cover_impl(
    app_dir: &std::path::Path,
    pool: &crate::db::DbPool,
    album_id: i64,
) -> Result<Option<crate::models::FetchedCover>, AppError> {
    // 会话级负缓存：近期尝试过就不再发请求（避免每次打开详情都反复拉取未命中的目标）
    let attempt_key = format!("album:{}", album_id);
    if cover_attempt_recently(&attempt_key) {
        return Ok(None);
    }
    mark_cover_attempt(&attempt_key);

    // 1. Get album info
    let (album_title, artist_name): (String, Option<String>) = {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("
            SELECT al.title, (SELECT name FROM artists WHERE id = (SELECT artist_id FROM track_artists WHERE track_id = t.id LIMIT 1))
            FROM albums al
            LEFT JOIN tracks t ON t.album_id = al.id
            WHERE al.id = ?1 LIMIT 1
        ")?;
        use rusqlite::OptionalExtension;
        let row = stmt
            .query_row(params![album_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        if let Some(r) = row {
            r
        } else {
            return Ok(None);
        }
    };

    // 2. 检索封面：网易云 1500x1500 → iTunes 最大分辨率（v1.8.1 换源）
    let Some(hit) = crate::services::cover::CoverService::search_album_cover(
        &album_title,
        artist_name.as_deref(),
    )
    .await
    else {
        return Ok(None);
    };

    // 3. Save to db
    use sha2::{Digest, Sha256};
    let bytes = hit.bytes;
    let mime_type = hit.mime_type;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());

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
    // 缩略图随事件下发（data URL），前端立即更新专辑网格，无需等下次全量拉取
    let thumbnail_base64 = thumbnail_blob.as_ref().map(|b| {
        use base64::{engine::general_purpose, Engine as _};
        format!(
            "data:image/jpeg;base64,{}",
            general_purpose::STANDARD.encode(b)
        )
    });

    let conn = pool.get()?;
    use rusqlite::OptionalExtension;
    // Check if hash exists
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM artwork WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        )
        .optional()?;

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

    Ok(Some(crate::models::FetchedCover {
        artwork_id,
        thumbnail_base64,
    }))
}

/// 触发专辑封面在线拉取（第七轮：后台化）。
///
/// dev 模式 IPC 走 `http://ipc.localhost`，受 WebView2 HTTP/1.1 单 host 6 并发上限约束；
/// 此命令单次 2-10s，若同步占用 channel，5+ 个无封面专辑就会把其他列表 IPC 全部
/// Stalled 5-15s。因此命令体立即返回 `Ok(None)` 不占并发坑位，真实下载在
/// tokio 后台任务执行，完成后 emit `album-cover-fetched` 事件，由前端订阅更新 UI。
#[tauri::command]
pub async fn library_fetch_missing_album_cover(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    album_id: i64,
    allow_online: Option<bool>,
) -> Result<Option<i64>, AppError> {
    let _trace = ipc_trace!("library_fetch_missing_album_cover");

    // P0-07 在线元数据隐私：默认拒绝，未显式授权不向 iTunes 发送专辑/艺人名称
    if allow_online != Some(true) {
        return Ok(None);
    }

    // State<'_, DbState> 有生命周期，不能 move 进 spawn；DbPool 内部是 Arc，clone 后 'static。
    let app_clone = app.clone();
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let pool = db_state.db.clone();
    tokio::spawn(async move {
        match fetch_album_cover_impl(&app_dir, &pool, album_id).await {
            Ok(Some(cover)) => {
                use tauri::Emitter;
                let _ = app_clone.emit(
                    "album-cover-fetched",
                    crate::models::CoverFetchedEvent {
                        target_id: album_id,
                        artwork_id: cover.artwork_id,
                        cover_thumbnail_base64: cover.thumbnail_base64,
                    },
                );
            }
            Ok(None) => {}
            Err(e) => tracing::warn!("[cover] album={} fetch failed: {}", album_id, e),
        }
    });

    Ok(None) // 立即返回，不占 WebView 并发
}

/// 艺人头像的真实拉取流程（网易云 → iTunes 兜底 + 缩略图 + 写库）。
/// 只在 tokio 后台任务中调用，不占 IPC channel（第七轮）。
async fn fetch_artist_cover_impl(
    app_dir: &std::path::Path,
    pool: &crate::db::DbPool,
    artist_id: i64,
) -> Result<Option<crate::models::FetchedCover>, AppError> {
    let attempt_key = format!("artist:{}", artist_id);
    if cover_attempt_recently(&attempt_key) {
        return Ok(None);
    }
    mark_cover_attempt(&attempt_key);

    // 1. Get artist info
    let artist_name: String = {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT name FROM artists WHERE id = ?1 LIMIT 1")?;
        use rusqlite::OptionalExtension;
        let row = stmt
            .query_row(params![artist_id], |row| row.get(0))
            .optional()?;
        if let Some(r) = row {
            r
        } else {
            return Ok(None);
        }
    };

    // 2. 检索头像：网易云歌手大图 1200x1200 → iTunes 兜底（最大分辨率）
    let Some(hit) = crate::services::cover::CoverService::search_artist_cover(&artist_name).await
    else {
        return Ok(None);
    };

    // 3. Save to db
    use sha2::{Digest, Sha256};
    let bytes = hit.bytes;
    let mime_type = hit.mime_type;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());

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

    let conn = pool.get()?;
    use rusqlite::OptionalExtension;

    // Check if hash exists
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM artwork WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        )
        .optional()?;

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

    Ok(Some(crate::models::FetchedCover {
        artwork_id,
        thumbnail_base64: None,
    }))
}

/// 触发艺人头像在线拉取（第七轮：后台化，机制同 library_fetch_missing_album_cover）。
#[tauri::command]
pub async fn library_fetch_missing_artist_cover(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    artist_id: i64,
    allow_online: Option<bool>,
) -> Result<Option<i64>, AppError> {
    let _trace = ipc_trace!("library_fetch_missing_artist_cover");

    // P0-07 在线元数据隐私：默认拒绝，未显式授权不向 iTunes 发送艺人名称
    if allow_online != Some(true) {
        return Ok(None);
    }

    let app_clone = app.clone();
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let pool = db_state.db.clone();
    tokio::spawn(async move {
        match fetch_artist_cover_impl(&app_dir, &pool, artist_id).await {
            Ok(Some(cover)) => {
                use tauri::Emitter;
                let _ = app_clone.emit(
                    "artist-cover-fetched",
                    crate::models::CoverFetchedEvent {
                        target_id: artist_id,
                        artwork_id: cover.artwork_id,
                        cover_thumbnail_base64: cover.thumbnail_base64,
                    },
                );
            }
            Ok(None) => {}
            Err(e) => tracing::warn!("[cover] artist={} fetch failed: {}", artist_id, e),
        }
    });

    Ok(None) // 立即返回，不占 WebView 并发
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
        "recently_added" => {
            crate::repositories::track_repo::TrackRepo::get_recently_added_tracks(&conn, limit)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        "recently_played" => {
            crate::repositories::track_repo::TrackRepo::get_recently_played_tracks(&conn, limit)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        "never_played" => {
            crate::repositories::track_repo::TrackRepo::get_never_played_tracks(&conn, limit)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        _ => Err(AppError::Internal(format!(
            "Unknown smart playlist kind: {}",
            kind
        ))),
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
        let score_a = crate::services::file_priority::file_priority_score(
            &a.source_kind,
            &a.file_ext.clone().unwrap_or_default(),
        );
        let score_b = crate::services::file_priority::file_priority_score(
            &b.source_kind,
            &b.file_ext.clone().unwrap_or_default(),
        );
        score_b.cmp(&score_a) // 降序
    });

    Ok(result)
}

#[tauri::command]
pub fn library_set_primary_file(
    db_state: State<'_, DbState>,
    track_id: i64,
    media_file_id: i64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("library_set_primary_file");
    let conn = db_state.db.get()?;
    let belongs_to_track: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM media_files
            WHERE id = ?1 AND track_id = ?2 AND availability = 'available'
        )",
        rusqlite::params![media_file_id, track_id],
        |row| row.get(0),
    )?;
    if !belongs_to_track {
        return Err(AppError::Internal(
            "所选音频版本不可用或不属于该歌曲".to_string(),
        ));
    }
    conn.execute(
        "UPDATE tracks SET primary_file_id = ?1 WHERE id = ?2",
        rusqlite::params![media_file_id, track_id],
    )?;
    Ok(())
}

/// 封面/头像拉取的会话级负缓存（v1.8.1）：目标 10 分钟内只尝试一次。
/// 换源前命中率低导致「每次打开详情都重新拉取」，观感上像"同步没保存"。
static COVER_ATTEMPTS: Mutex<Option<HashMap<String, std::time::Instant>>> = Mutex::new(None);
const COVER_ATTEMPT_TTL_SECS: u64 = 600;

fn cover_attempt_recently(key: &str) -> bool {
    let mut guard = COVER_ATTEMPTS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.retain(|_, t| t.elapsed().as_secs() < COVER_ATTEMPT_TTL_SECS);
    map.contains_key(key)
}

fn mark_cover_attempt(key: &str) {
    COVER_ATTEMPTS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(key.to_string(), std::time::Instant::now());
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Playability {
    Local,
    Cached,
    Remote,
    Unavailable,
}

/// 一次批量查询里为某首曲目取得的判定输入
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayabilityRow {
    track_id: i64,
    media_file_id: Option<i64>,
    kind: Option<String>,
    root_uri: Option<String>,
    availability: Option<String>,
    file_size: Option<i64>,
}

impl PlayabilityRow {
    /// 缓存有效性校验用的期望体积；DB 未记体积时为 None（退化为「非空即有效」）
    fn expected_size(&self) -> Option<u64> {
        match self.file_size.unwrap_or(0) {
            size if size > 0 => Some(size as u64),
            _ => None,
        }
    }
}

/// 取一批 track 的判定输入。`kind` 经 `media_files → sources` 两段 LEFT JOIN 得来，
/// 因此曲目没有主文件时 kind 也是 None（不是漏 JOIN）。
fn query_playability_rows(
    conn: &rusqlite::Connection,
    ids: &[i64],
) -> Result<Vec<PlayabilityRow>, AppError> {
    let sql = format!(
        "SELECT t.id, m.id, s.kind, s.root_uri, m.availability, m.file_size
         FROM tracks t
         LEFT JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
         LEFT JOIN sources s ON s.id = m.source_id
         WHERE t.id IN ({})",
        vec!["?"; ids.len()].join(",")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
        Ok(PlayabilityRow {
            track_id: row.get(0)?,
            media_file_id: row.get(1)?,
            kind: row.get(2)?,
            root_uri: row.get(3)?,
            availability: row.get(4)?,
            file_size: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// 纯判定（单测接缝）：输入已全部算好，本函数不做任何 I/O。
///
/// 本地曲目的存在性**不再逐曲 stat**，而是用扫描器写下的 `media_files.availability`
/// 加上一次「来源根目录是否还在」的探测（30k 首机械盘曲库里每条来源只需 1 次 stat，
/// 旧实现是每条 1 次、单条 10–20 ms）。整盘拔出 / 挂载点消失这类情况一次根目录探测就能判出。
/// 已知代价：在库外手工删除单个文件时，置灰要等下次扫描才更新——此时点击仍会被
/// `playback_play` 的打开失败路径拦下并提示，不会静默播放失败。
///
/// 云端曲目只看缓存（含体积校验），与旧实现一致，不受 availability 影响。
fn classify_playability(
    row: &PlayabilityRow,
    root_reachable: bool,
    cache_hit: bool,
) -> Playability {
    match (row.kind.as_deref(), row.media_file_id) {
        (Some("local"), _) => {
            if root_reachable && row.availability.as_deref() == Some("available") {
                Playability::Local
            } else {
                Playability::Unavailable
            }
        }
        (Some("webdav"), Some(_)) => {
            if cache_hit {
                Playability::Cached
            } else {
                Playability::Remote
            }
        }
        _ => Playability::Unavailable,
    }
}

/// [MA3 A3-3] 批量查询歌曲的可播性状态（本地存在 / 已缓存 / 远端流播 / 不可用）
///
/// `#[tauri::command(async)]`：sync 命令跑在主线程且串行（见 `library_get_insights` 上方注释），
/// 本命令要对整屏曲目做文件系统判定，留在主线程就是一次界面冻结。
#[tauri::command(async)]
pub fn library_get_playability(
    db_state: State<'_, DbState>,
    cache_state: State<'_, crate::services::cache::AudioCacheState>,
    track_ids: Vec<i64>,
) -> Result<std::collections::HashMap<i64, Playability>, AppError> {
    let _trace = ipc_trace!("library_get_playability");
    let mut map = std::collections::HashMap::new();
    if track_ids.is_empty() {
        return Ok(map);
    }

    let rows = {
        let conn = db_state.db.get()?;
        let mut rows = Vec::with_capacity(track_ids.len());
        for chunk in track_ids.chunks(500) {
            rows.extend(query_playability_rows(&conn, chunk)?);
        }
        rows
    };

    // 缓存查询集中在这一把短锁里问完：它是唯一需要与「正在写缓存」互斥的部分。
    // 旧实现把整轮循环（含逐曲磁盘探测）罩在锁内，可播性查询能拖住 playback_play。
    let cached_files: std::collections::HashSet<i64> = {
        let cache = cache_state
            .cache
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        rows.iter()
            .filter(|row| row.kind.as_deref() == Some("webdav"))
            .filter(|row| {
                row.media_file_id
                    .is_some_and(|mf| cache.is_cached(mf, row.expected_size()))
            })
            .filter_map(|row| row.media_file_id)
            .collect()
    };

    // 每个来源根目录只探测一次，整批共用
    let mut roots: HashMap<&str, bool> = HashMap::new();
    for row in &rows {
        let root_reachable = match row.kind.as_deref() {
            Some("local") => match row.root_uri.as_deref() {
                Some(root) => *roots
                    .entry(root)
                    .or_insert_with(|| std::path::Path::new(root).exists()),
                None => false,
            },
            _ => false,
        };
        let cache_hit = row
            .media_file_id
            .is_some_and(|mf| cached_files.contains(&mf));
        map.insert(
            row.track_id,
            classify_playability(row, root_reachable, cache_hit),
        );
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_util::TempDir;
    use rusqlite::params;

    fn row(
        kind: Option<&str>,
        availability: Option<&str>,
        media_file_id: Option<i64>,
    ) -> PlayabilityRow {
        PlayabilityRow {
            track_id: 1,
            media_file_id,
            kind: kind.map(str::to_string),
            root_uri: kind.map(|_| "X:/lib".to_string()),
            availability: availability.map(str::to_string),
            file_size: Some(20_000_000),
        }
    }

    #[test]
    fn local_track_needs_both_scanner_verdict_and_reachable_root() {
        let available = row(Some("local"), Some("available"), Some(7));
        assert_eq!(
            classify_playability(&available, true, false),
            Playability::Local
        );
        // 盘被拔掉 / 挂载点消失：一次根目录探测就够，不必逐曲 stat
        assert_eq!(
            classify_playability(&available, false, false),
            Playability::Unavailable
        );
    }

    #[test]
    fn local_track_marked_missing_or_corrupt_by_scan_is_unavailable() {
        // 扫描器写入的三种非 available 状态都不该被标成可播
        for state in ["missing", "offline", "error"] {
            let row = row(Some("local"), Some(state), Some(7));
            assert_eq!(
                classify_playability(&row, true, true),
                Playability::Unavailable,
                "availability={state} 仍被判为可播"
            );
        }
    }

    /// 旧实现只看缓存、不看 availability（云端文件被扫描标记丢失也不改判）。
    /// 这是刻意保留的口径：本命令只做性能改造，不顺手改语义。
    #[test]
    fn webdav_track_follows_cache_not_availability() {
        let cached = row(Some("webdav"), Some("available"), Some(7));
        assert_eq!(
            classify_playability(&cached, false, true),
            Playability::Cached
        );
        assert_eq!(
            classify_playability(&cached, false, false),
            Playability::Remote
        );
        // 根目录探测与云端无关：即使为 true 也不会把未缓存的云端曲目说成本地可播
        assert_eq!(
            classify_playability(&cached, true, false),
            Playability::Remote
        );
    }

    #[test]
    fn track_without_primary_media_file_is_unavailable() {
        // kind 来自 media_files → sources 的 LEFT JOIN，没有主文件时同为 None
        let row = row(None, None, None);
        assert_eq!(
            classify_playability(&row, true, true),
            Playability::Unavailable
        );
    }

    #[test]
    fn expected_size_absents_unknown_or_non_positive_file_size() {
        let mut row = row(Some("webdav"), Some("available"), Some(7));
        assert_eq!(row.expected_size(), Some(20_000_000));
        row.file_size = Some(0);
        assert_eq!(row.expected_size(), None);
        row.file_size = Some(-1);
        assert_eq!(row.expected_size(), None);
        row.file_size = None;
        assert_eq!(row.expected_size(), None);
    }

    /// 列名回归：本命令曾因写错 `media_files` 的列名（`m.path` 不存在）而在运行时炸。
    /// 这里对真实 schema 跑一次查询，锁住所引用的列确实存在、且取值口径正确。
    #[test]
    fn playability_query_reads_real_schema_columns() {
        let dir = TempDir::new("playability_sql");
        let pool = crate::db::init_db(dir.db_path()).unwrap();
        let conn = pool.get().unwrap();
        let root = dir.path().to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO sources (id, name, kind, root_uri) VALUES (1, '本地', 'local', ?1)",
            params![&root],
        )
        .unwrap();
        // media_files.track_id 有外键，先建曲目再建文件；primary_file_id 无外键，最后回填
        conn.execute(
            "INSERT INTO tracks (id, title, normalized_title) VALUES (101, 'A', 'a'), (102, 'B', 'b'), (103, 'C', 'c')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO media_files (id, source_id, track_id, relative_path, normalized_path,
                                      file_name, file_size, availability)
             VALUES (11, 1, 101, 'a.flac', 'a.flac', 'a.flac', 1234, 'available'),
                    (12, 1, 102, 'b.flac', 'b.flac', 'b.flac', 0, 'missing')",
            [],
        )
        .unwrap();
        // 101 有 primary_file_id；102 故意留 NULL，验证 COALESCE 回退子查询取到 12；103 两路都无文件
        conn.execute("UPDATE tracks SET primary_file_id = 11 WHERE id = 101", [])
            .unwrap();

        let rows = query_playability_rows(
            &conn,
            &[
                101, 102, 103,
                999, // 不存在的 id：不应报错，也不应出现在结果里
            ],
        )
        .unwrap();
        let by_track: HashMap<i64, &PlayabilityRow> =
            rows.iter().map(|r| (r.track_id, r)).collect();
        // 103 无任何媒体文件：LEFT JOIN 仍出一行（kind 为 NULL），由判定归入不可用；
        // 只有查询里不存在的 999 才完全不出现。
        assert_eq!(by_track.len(), 3, "只应过滤掉查询里不存在的 id");
        let orphan = by_track.get(&103).unwrap();
        assert_eq!(orphan.kind, None);
        assert_eq!(
            classify_playability(orphan, true, true),
            Playability::Unavailable,
            "没有主文件的曲目不能判为可播"
        );

        let present = by_track.get(&101).unwrap();
        assert_eq!(present.kind.as_deref(), Some("local"));
        assert_eq!(present.root_uri.as_deref(), Some(root.as_str()));
        assert_eq!(present.availability.as_deref(), Some("available"));
        assert_eq!(present.expected_size(), Some(1234));

        let missing = by_track.get(&102).unwrap();
        assert_eq!(
            missing.media_file_id,
            Some(12),
            "primary_file_id 为空时应回退到该曲目的媒体文件"
        );
        assert_eq!(missing.availability.as_deref(), Some("missing"));
        assert_eq!(missing.expected_size(), None, "0 体积不能当成有效期望值");
    }

    /// 判定必须来自 availability 而非「文件是否真的存在」：临时库里的 a.flac 并不在磁盘上，
    /// 根目录存在 + available 仍判可播，扫描器标 missing 即判不可用。
    #[test]
    fn local_verdict_comes_from_availability_not_per_file_stat() {
        let dir = TempDir::new("playability_root");
        let pool = crate::db::init_db(dir.db_path()).unwrap();
        let conn = pool.get().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        conn.execute(
            "INSERT INTO sources (id, name, kind, root_uri) VALUES (1, '本地', 'local', ?1)",
            params![&root],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tracks (id, title, normalized_title) VALUES (101, 'A', 'a'), (102, 'B', 'b')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO media_files (id, source_id, track_id, relative_path, normalized_path,
                                      file_name, file_size, availability)
             VALUES (11, 1, 101, '不存在的文件.flac', 'x', 'x', 10, 'available'),
                    (12, 1, 102, '也不存在.flac', 'y', 'y', 10, 'missing')",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE tracks SET primary_file_id = CASE id WHEN 101 THEN 11 WHEN 102 THEN 12 END",
            [],
        )
        .unwrap();

        let rows = query_playability_rows(&conn, &[101, 102]).unwrap();
        let verdicts: HashMap<i64, Playability> = rows
            .iter()
            .map(|r| {
                let reachable = std::path::Path::new(r.root_uri.as_deref().unwrap()).exists();
                (r.track_id, classify_playability(r, reachable, false))
            })
            .collect();
        assert_eq!(verdicts[&101], Playability::Local);
        assert_eq!(verdicts[&102], Playability::Unavailable);
    }
}
