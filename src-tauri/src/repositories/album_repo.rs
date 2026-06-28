use rusqlite::{Connection, params};
use base64::{engine::general_purpose, Engine as _};
use crate::models::*;

pub struct AlbumRepo;

impl AlbumRepo {
    pub fn get_album_count(conn: &Connection, search_keyword: Option<String>) -> rusqlite::Result<i64> {
        let mut sql = "SELECT COUNT(*) FROM albums al WHERE 1=1".to_string();
        let keyword_pattern = if let Some(keyword) = search_keyword {
            let kw = keyword.trim();
            if !kw.is_empty() {
                sql.push_str(" AND (al.normalized_title LIKE ? OR EXISTS (SELECT 1 FROM artists ar WHERE ar.id = al.album_artist_id AND ar.name LIKE ?))");
                Some(format!("%{}%", kw.to_lowercase()))
            } else { None }
        } else { None };

        if let Some(pattern) = keyword_pattern {
            let mut stmt = conn.prepare(&sql)?;
            let count: i64 = stmt.query_row(params![pattern, pattern], |row| row.get(0))?;
            Ok(count)
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        }
    }

    pub fn get_albums_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<AlbumDTO>> {
            // ===== [诊断日志] 临时性能分析，问题定位后可移除 =====
            let t_start = std::time::Instant::now();
    
            // LEFT JOIN artwork 读取 thumbnail_blob，内联返回 base64 缩略图。
            // 这样前端网格视图不再需要逐个发 lumo://artwork 请求，消灭 N+1。
            let mut sql = "
                SELECT
                    al.id,
                    al.title,
                    ar.name AS artist_name,
                    al.cover_artwork_id,
                    al.track_count,
                    aw.thumbnail_blob
                FROM albums al
                LEFT JOIN artists ar ON al.album_artist_id = ar.id
                LEFT JOIN artwork aw ON al.cover_artwork_id = aw.id
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
    
            // 把 thumbnail_blob (BLOB) 转为 base64 data URL 的闭包
            let blob_to_data_url = |blob: Option<Vec<u8>>| -> Option<String> {
                blob.map(|b| format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(&b)))
            };
    
            if let Some(pattern) = keyword_pattern {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(params![pattern, pattern, limit, offset], |row| {
                    let thumb: Option<Vec<u8>> = row.get(5)?;
                    Ok(AlbumDTO {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        artist_name: row.get(2)?,
                        cover_artwork_id: row.get(3)?,
                        track_count: row.get(4)?,
                        cover_thumbnail_base64: blob_to_data_url(thumb),
                    })
                })?;
                for r in rows { result.push(r?); }
            } else {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(params![limit, offset], |row| {
                    let thumb: Option<Vec<u8>> = row.get(5)?;
                    Ok(AlbumDTO {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        artist_name: row.get(2)?,
                        cover_artwork_id: row.get(3)?,
                        track_count: row.get(4)?,
                        cover_thumbnail_base64: blob_to_data_url(thumb),
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
