use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO};
use crate::services::metadata::AudioMetadata;
use rusqlite::{Connection, params, OptionalExtension};
use sha2::{Sha256, Digest};
use std::fs;

pub struct LibraryService;

impl LibraryService {
    pub fn index_file(conn: &Connection, source_id: i64, path: &std::path::Path, metadata: &AudioMetadata, app_data_dir: &std::path::Path) -> rusqlite::Result<()> {
        let relative_path = path.to_string_lossy().to_string();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let artist_name = metadata.artist.as_deref().unwrap_or("Unknown Artist");
        conn.execute(
            "INSERT OR IGNORE INTO artists (name, normalized_name, sort_name) VALUES (?1, ?2, ?2)",
            params![artist_name, artist_name.to_lowercase()],
        )?;
        let artist_id: i64 = conn.query_row(
            "SELECT id FROM artists WHERE normalized_name = ?1 LIMIT 1",
            params![artist_name.to_lowercase()],
            |row| row.get(0),
        )?;

        let mut artwork_id: Option<i64> = None;
        if let Some(picture_data) = &metadata.picture_data {
            let mut hasher = Sha256::new();
            hasher.update(picture_data);
            let hash = hex::encode(hasher.finalize());
            
            let artworks_dir = app_data_dir.join("artworks");
            if !artworks_dir.exists() {
                let _ = fs::create_dir_all(&artworks_dir);
            }
            
            let ext = metadata.picture_mime.as_deref().and_then(|m| match m {
                "image/png" => Some("png"),
                "image/jpeg" | "image/jpg" => Some("jpg"),
                "image/gif" => Some("gif"),
                "image/webp" => Some("webp"),
                _ => None,
            }).unwrap_or("jpg");
            
            let file_name = format!("{}.{}", hash, ext);
            let cache_path = artworks_dir.join(&file_name);
            
            let existing_id: Option<i64> = conn.query_row(
                "SELECT id FROM artwork WHERE content_hash = ?1",
                params![hash],
                |row| row.get(0),
            ).optional()?;
            
            if let Some(id) = existing_id {
                artwork_id = Some(id);
            } else {
                if !cache_path.exists() {
                    let _ = fs::write(&cache_path, picture_data);
                }
                let cache_path_str = cache_path.to_string_lossy().to_string();
                conn.execute(
                    "INSERT INTO artwork (cache_path, mime_type, content_hash) VALUES (?1, ?2, ?3)",
                    params![cache_path_str, metadata.picture_mime, hash],
                )?;
                artwork_id = Some(conn.last_insert_rowid());
            }
        }

        let album_title = metadata.album.as_deref().unwrap_or("Unknown Album");
        
        let album_id: i64 = match conn.query_row(
            "SELECT id FROM albums WHERE normalized_title = ?1 AND album_artist_id = ?2 LIMIT 1",
            params![album_title.to_lowercase(), artist_id],
            |row| row.get(0),
        ).optional()? {
            Some(id) => {
                if let Some(aid) = artwork_id {
                    // Update cover if it already exists but lacks one, or overwrite
                    conn.execute(
                        "UPDATE albums SET cover_artwork_id = ?1 WHERE id = ?2 AND cover_artwork_id IS NULL",
                        params![aid, id],
                    )?;
                }
                id
            },
            None => {
                conn.execute(
                    "INSERT INTO albums (title, normalized_title, sort_title, album_artist_id, cover_artwork_id) VALUES (?1, ?2, ?2, ?3, ?4)",
                    params![album_title, album_title.to_lowercase(), artist_id, artwork_id],
                )?;
                conn.last_insert_rowid()
            }
        };

        let track_title = metadata.title.as_deref().unwrap_or(&file_name);
        conn.execute(
            "INSERT OR IGNORE INTO tracks (title, normalized_title, sort_title, album_id) VALUES (?1, ?2, ?2, ?3)",
            params![track_title, track_title.to_lowercase(), album_id],
        )?;
        let track_id: i64 = conn.query_row(
            "SELECT id FROM tracks WHERE normalized_title = ?1 AND album_id = ?2 LIMIT 1",
            params![track_title.to_lowercase(), album_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO media_files (
                source_id, track_id, relative_path, normalized_path, file_name, file_ext, duration_ms, bitrate, sample_rate, channels
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(source_id, normalized_path) DO UPDATE SET
                track_id=excluded.track_id, duration_ms=excluded.duration_ms",
            params![
                source_id,
                track_id,
                relative_path,
                relative_path,
                file_name,
                ext,
                metadata.duration_ms,
                metadata.bit_rate,
                metadata.sample_rate,
                metadata.channels
            ],
        )?;

        Ok(())
    }

    pub fn get_tracks_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut sql = "
            SELECT 
                t.id, 
                t.title, 
                ar.name AS artist_name, 
                al.title AS album_title, 
                m.duration_ms, 
                m.file_ext, 
                m.id AS media_file_id,
                tf.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN artists ar ON ar.id = al.album_artist_id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN track_favorites tf ON t.id = tf.track_id
            WHERE 1=1
        ".to_string();

        let keyword_pattern = if let Some(keyword) = search_keyword {
            let kw = keyword.trim();
            if !kw.is_empty() {
                sql.push_str(" AND (t.title LIKE ? OR ar.name LIKE ? OR al.title LIKE ?)");
                Some(format!("%{}%", kw))
            } else {
                None
            }
        } else {
            None
        };

        sql.push_str(" ORDER BY t.added_at DESC LIMIT ? OFFSET ?");

        let mut result = Vec::new();

        if let Some(pattern) = keyword_pattern {
            let mut stmt = conn.prepare(&sql)?;
            let tracks = stmt.query_map(params![pattern, pattern, pattern, limit, offset], |row| {
                Ok(TrackDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    album_title: row.get(3)?,
                    duration_ms: row.get(4)?,
                    format: row.get(5)?,
                    media_file_id: row.get(6)?,
                    is_favorite: row.get(7)?,
                })
            })?;
            for t in tracks {
                result.push(t?);
            }
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let tracks = stmt.query_map(params![limit, offset], |row| {
                Ok(TrackDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    album_title: row.get(3)?,
                    duration_ms: row.get(4)?,
                    format: row.get(5)?,
                    media_file_id: row.get(6)?,
                    is_favorite: row.get(7)?,
                })
            })?;
            for t in tracks {
                result.push(t?);
            }
        }

        Ok(result)
    }

    pub fn get_albums_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<AlbumDTO>> {
        let mut sql = "
            SELECT 
                al.id, 
                al.title, 
                ar.name AS artist_name, 
                al.cover_artwork_id,
                COUNT(t.id) as track_count
            FROM albums al
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            LEFT JOIN tracks t ON t.album_id = al.id
            WHERE 1=1
        ".to_string();

        let keyword_pattern = if let Some(keyword) = search_keyword {
            let kw = keyword.trim();
            if !kw.is_empty() {
                sql.push_str(" AND (al.title LIKE ? OR ar.name LIKE ?)");
                Some(format!("%{}%", kw))
            } else { None }
        } else { None };

        sql.push_str(" GROUP BY al.id ORDER BY al.title COLLATE NOCASE ASC LIMIT ? OFFSET ?");
        
        let mut result = Vec::new();

        if let Some(pattern) = keyword_pattern {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![pattern, pattern, limit, offset], |row| {
                Ok(AlbumDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    cover_artwork_id: row.get(3)?,
                    track_count: row.get(4)?,
                })
            })?;
            for r in rows { result.push(r?); }
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![limit, offset], |row| {
                Ok(AlbumDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    cover_artwork_id: row.get(3)?,
                    track_count: row.get(4)?,
                })
            })?;
            for r in rows { result.push(r?); }
        }
        Ok(result)
    }

    pub fn get_artists_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<ArtistDTO>> {
        let mut sql = "
            SELECT 
                ar.id, 
                ar.name,
                (SELECT COUNT(t.id) FROM tracks t LEFT JOIN albums al ON t.album_id = al.id WHERE al.album_artist_id = ar.id) as track_count
            FROM artists ar
            WHERE 1=1
        ".to_string();

        let keyword_pattern = if let Some(keyword) = search_keyword {
            let kw = keyword.trim();
            if !kw.is_empty() {
                sql.push_str(" AND ar.name LIKE ?");
                Some(format!("%{}%", kw))
            } else { None }
        } else { None };

        sql.push_str(" ORDER BY ar.name COLLATE NOCASE ASC LIMIT ? OFFSET ?");
        
        let mut result = Vec::new();
        if let Some(pattern) = keyword_pattern {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![pattern, limit, offset], |row| {
                Ok(ArtistDTO {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    track_count: row.get(2)?,
                })
            })?;
            for r in rows { result.push(r?); }
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![limit, offset], |row| {
                Ok(ArtistDTO {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    track_count: row.get(2)?,
                })
            })?;
            for r in rows { result.push(r?); }
        }
        Ok(result)
    }

    pub fn toggle_favorite(conn: &Connection, track_id: i64, is_favorite: bool) -> rusqlite::Result<()> {
        if is_favorite {
            conn.execute(
                "INSERT OR IGNORE INTO track_favorites (track_id) VALUES (?1)",
                params![track_id],
            )?;
        } else {
            conn.execute(
                "DELETE FROM track_favorites WHERE track_id = ?1",
                params![track_id],
            )?;
        }
        Ok(())
    }

    pub fn create_playlist(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
        conn.execute(
            "INSERT INTO playlists (name) VALUES (?1)",
            params![name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_playlists(conn: &Connection) -> rusqlite::Result<Vec<PlaylistDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                p.id, 
                p.name,
                COUNT(pt.track_id) as track_count
            FROM playlists p
            LEFT JOIN playlist_tracks pt ON p.id = pt.playlist_id
            GROUP BY p.id
            ORDER BY p.created_at ASC
        ")?;
        
        let rows = stmt.query_map([], |row| {
            Ok(PlaylistDTO {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
            })
        })?;
        
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn add_to_playlist(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<()> {
        let max_pos: Option<i64> = conn.query_row(
            "SELECT MAX(position) FROM playlist_tracks WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
            |row| row.get(0)
        ).unwrap_or(None);
        
        let next_pos = max_pos.unwrap_or(0) + 1;
        
        conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, track_id, next_pos],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(conn: &Connection, playlist_id: i64) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, 
                t.title, 
                ar.name AS artist_name, 
                al.title AS album_title, 
                m.duration_ms, 
                m.file_ext, 
                m.id AS media_file_id,
                tf.track_id IS NOT NULL AS is_favorite
            FROM playlist_tracks pt
            JOIN tracks t ON pt.track_id = t.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN track_favorites tf ON t.id = tf.track_id
            WHERE pt.playlist_id = ?1
            ORDER BY pt.position ASC
        ")?;
        
        let rows = stmt.query_map([playlist_id], |row| {
            Ok(TrackDTO {
                id: row.get(0)?,
                title: row.get(1)?,
                artist_name: row.get(2)?,
                album_title: row.get(3)?,
                duration_ms: row.get(4)?,
                format: row.get(5)?,
                media_file_id: row.get(6)?,
                is_favorite: row.get(7)?,
            })
        })?;
        
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn record_play(conn: &Connection, track_id: i64) -> rusqlite::Result<()> {
        conn.execute(
            "UPDATE tracks SET play_count = play_count + 1, last_played_at = datetime('now') WHERE id = ?1",
            rusqlite::params![track_id],
        )?;
        Ok(())
    }

    pub fn get_recently_played(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT t.id, t.title, ar.name AS artist_name, al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, tf.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN track_favorites tf ON t.id = tf.track_id
            WHERE t.last_played_at IS NOT NULL
            ORDER BY t.last_played_at DESC LIMIT ?1
        ")?;
        let rows = stmt.query_map([limit], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    pub fn get_favorite_tracks(conn: &Connection) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT t.id, t.title, ar.name AS artist_name, al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, 1 AS is_favorite
            FROM track_favorites tf
            JOIN tracks t ON tf.track_id = t.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            JOIN media_files m ON t.id = m.track_id
            ORDER BY tf.created_at DESC
        ")?;
        let rows = stmt.query_map([], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    fn map_track_row(row: &rusqlite::Row) -> rusqlite::Result<TrackDTO> {
        Ok(TrackDTO {
            id: row.get(0)?,
            title: row.get(1)?,
            artist_name: row.get(2)?,
            album_title: row.get(3)?,
            duration_ms: row.get(4)?,
            format: row.get(5)?,
            media_file_id: row.get(6)?,
            is_favorite: row.get(7)?,
        })
    }
}
