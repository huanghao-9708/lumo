use rusqlite::{Connection, params};
use crate::models::*;

pub struct AlbumRepo;

impl AlbumRepo {
    pub fn get_albums_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<AlbumDTO>> {
            // ===== [诊断日志] 临时性能分析，问题定位后可移除 =====
            let t_start = std::time::Instant::now();
    
            let mut sql = "
                SELECT
                    al.id,
                    al.title,
                    ar.name AS artist_name,
                    al.cover_artwork_id,
                    al.track_count
                FROM albums al
                LEFT JOIN artists ar ON al.album_artist_id = ar.id
                WHERE 1=1
            ".to_string();
    
            let keyword_pattern = if let Some(keyword) = search_keyword {
                let kw = keyword.trim();
                if !kw.is_empty() {
                    sql.push_str(" AND (al.normalized_title LIKE ? OR ar.name LIKE ?)");
                    Some(format!("%{}%", kw.to_lowercase()))
                } else { None }
            } else { None };
    
            sql.push_str(" ORDER BY al.normalized_title ASC LIMIT ? OFFSET ?");
    
            let t_sql_built = t_start.elapsed();
    
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
    
            let t_end = t_start.elapsed();
            tracing::info!(
                "[PERF] get_albums_paginated: limit={}, offset={}, returned={} | \
                 sql_build={:?} sql_exec={:?} total={:?}",
                limit, offset, result.len(),
                t_sql_built,
                t_end - t_sql_built,
                t_end
            );
    
            Ok(result)
        }

    pub fn get_album_tracks(conn: &Connection, album_id: i64) -> rusqlite::Result<Vec<TrackDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    t.id, t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta2 JOIN artists a ON ta2.artist_id = a.id WHERE ta2.track_id = t.id ORDER BY ta2.position) AS artist_name,
                    al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id
                FROM tracks t
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON t.id = m.track_id
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE t.album_id = ?1
                ORDER BY t.disc_no ASC, t.track_no ASC, t.title ASC
            ")?;
            let rows = stmt.query_map([album_id], crate::repositories::map_track_row)?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

}
