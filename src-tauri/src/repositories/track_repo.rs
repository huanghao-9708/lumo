use rusqlite::{Connection, params};
use crate::models::*;

pub struct TrackRepo;

impl TrackRepo {
    pub fn get_tracks_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<TrackDTO>> {
            // 使用 GROUP_CONCAT 聚合子查询来合并一首歌对应的多位艺人名称
            let mut sql = "
                SELECT 
                    t.id, 
                    t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
                    al.title AS album_title, 
                    m.duration_ms, 
                    m.file_ext, 
                    m.id AS media_file_id,
                    ft.track_id IS NOT NULL AS is_favorite,
                    al.cover_artwork_id
                FROM tracks t
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE 1=1
            ".to_string();
    
            let keyword_pattern = if let Some(keyword) = search_keyword {
                let kw = keyword.trim();
                if !kw.is_empty() {
                    sql.push_str(" AND (t.title LIKE ? OR album_title LIKE ? OR artist_name LIKE ?)");
                    Some(format!("%{}%", kw))
                } else {
                    None
                }
            } else {
                None
            };
    
            // 默认按照添加时间倒序返回最新扫描的歌曲
            // 加 t.id 作 tiebreaker，避免相同 added_at 时分页结果重叠
            sql.push_str(" ORDER BY t.added_at DESC, t.id ASC LIMIT ? OFFSET ?");
    
            let mut result = Vec::new();
    
            if let Some(pattern) = keyword_pattern {
                let mut stmt = conn.prepare(&sql)?;
                let tracks = stmt.query_map(params![pattern, pattern, pattern, limit, offset], crate::repositories::map_track_row)?;
                for t in tracks { result.push(t?); }
            } else {
                let mut stmt = conn.prepare(&sql)?;
                let tracks = stmt.query_map(params![limit, offset], crate::repositories::map_track_row)?;
                for t in tracks { result.push(t?); }
            }
    
            Ok(result)
        }

    pub fn toggle_favorite(conn: &Connection, track_id: i64, is_favorite: bool) -> rusqlite::Result<()> {
            if is_favorite {
                conn.execute(
                    "INSERT OR IGNORE INTO favorite_tracks (track_id) VALUES (?1)",
                    params![track_id],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM favorite_tracks WHERE track_id = ?1",
                    params![track_id],
                )?;
            }
            Ok(())
        }

    pub fn record_play(conn: &Connection, track_id: i64, duration_ms: i64) -> rusqlite::Result<()> {
            // 更新总体计数字段
            conn.execute(
                "UPDATE tracks SET play_count = play_count + 1, last_played_at = datetime('now') WHERE id = ?1",
                rusqlite::params![track_id],
            )?;
            
            let media_file_id: Option<i64> = conn.query_row("SELECT primary_file_id FROM tracks WHERE id = ?1", params![track_id], |row| row.get(0)).unwrap_or(None);
            
            // 生成流水账单记录，用于复杂的统计
            conn.execute(
                "INSERT INTO play_history (track_id, media_file_id, source_kind, play_duration_ms) VALUES (?1, ?2, 'local', ?3)",
                params![track_id, media_file_id, duration_ms],
            )?;
            Ok(())
        }

    pub fn get_recently_played(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<TrackDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    t.id, t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name, 
                    al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id,
                    t.last_played_at
                FROM tracks t
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE t.last_played_at IS NOT NULL
                ORDER BY t.last_played_at DESC LIMIT ?1
            ")?;
            let rows = stmt.query_map([limit], |row| {
                Ok(TrackDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    album_title: row.get(3)?,
                    duration_ms: row.get(4)?,
                    format: row.get(5)?,
                    media_file_id: row.get(6)?,
                    is_favorite: row.get(7)?,
                    cover_artwork_id: row.get(8)?,
                    last_played_at: row.get(9)?,
                })
            })?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn get_favorite_tracks(conn: &Connection) -> rusqlite::Result<Vec<TrackDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    t.id, t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name, 
                    al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, 1 AS is_favorite, al.cover_artwork_id
                FROM favorite_tracks ft
                JOIN tracks t ON ft.track_id = t.id
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                ORDER BY ft.favorited_at DESC
            ")?;
            let rows = stmt.query_map([], crate::repositories::map_track_row)?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn get_track_by_path(conn: &Connection, source_id: i64, path: &str) -> rusqlite::Result<TrackDTO> {
            let sql = "
                SELECT 
                    t.id, t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
                    al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id
                FROM media_files m
                JOIN tracks t ON m.track_id = t.id
                LEFT JOIN albums al ON t.album_id = al.id
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE m.source_id = ?1 AND m.normalized_path = ?2
            ";
            conn.query_row(sql, rusqlite::params![source_id, path], crate::repositories::map_track_row)
        }

    pub fn save_play_queue(conn: &Connection, track_ids: &[i64]) -> rusqlite::Result<()> {
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM play_queue", [])?;
            {
                let mut stmt = tx.prepare("INSERT INTO play_queue (track_id, position) VALUES (?1, ?2)")?;
                for (idx, id) in track_ids.iter().enumerate() {
                    stmt.execute(params![id, idx as f64])?;
                }
            }
            tx.commit()?;
            Ok(())
        }

    pub fn get_play_queue(conn: &Connection) -> rusqlite::Result<Vec<TrackDTO>> {
            let sql = "
                SELECT 
                    t.id, 
                    t.title, 
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
                    al.title AS album_title, 
                    m.duration_ms, 
                    m.file_ext, 
                    m.id AS media_file_id,
                    (ft.track_id IS NOT NULL) AS is_favorite,
                    al.cover_artwork_id
                FROM play_queue pq
                JOIN tracks t ON pq.track_id = t.id
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                ORDER BY pq.position ASC
            ";
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([], crate::repositories::map_track_row)?;
            let mut result = Vec::new();
            for r in rows {
                result.push(r?);
            }
            Ok(result)
        }

    pub fn get_folder_contents(
            conn: &Connection,
            source_id: i64,
            folder_path: &std::path::Path,
            limit: Option<u32>,
            offset: u32,
        ) -> rusqlite::Result<crate::models::FolderContentsResult> {
            let mut entries = Vec::new();
    
            if let Ok(read_dir) = std::fs::read_dir(folder_path) {
                for entry_res in read_dir {
                    let Ok(entry) = entry_res else { continue };
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
    
                    if is_dir {
                        entries.push(crate::models::FolderEntryDTO {
                            name,
                            is_dir: true,
                            path: path.to_string_lossy().to_string(),
                            track: None,
                        });
                    } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext.to_lowercase().as_str(), "mp3" | "flac" | "wav" | "m4a" | "aac") {
                            let path_str = path.to_string_lossy().to_string();
                            let track = Self::get_track_by_path(conn, source_id, &path_str).ok();
    
                            entries.push(crate::models::FolderEntryDTO {
                                name,
                                is_dir: false,
                                path: path_str,
                                track,
                            });
                        }
                    }
                }
            }
    
            // 排序：目录优先，再按名称（保持大小写敏感的字典序，与原实现一致）
            entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    
            let total = entries.len();
            // 应用分页
            if let Some(lim) = limit {
                let start = (offset as usize).min(total);
                let end = (start + lim as usize).min(total);
                entries = entries.split_off(start);
                entries.truncate(end - start);
            }
    
            Ok(crate::models::FolderContentsResult { entries, total })
        }

}
