use rusqlite::{Connection, params};
use crate::models::*;

pub struct PlaylistRepo;

impl PlaylistRepo {
    pub fn create_playlist(conn: &Connection, name: &str, description: Option<&str>) -> rusqlite::Result<i64> {
            conn.execute(
                "INSERT INTO playlists (name, description) VALUES (?1, ?2)",
                params![name, description],
            )?;
            Ok(conn.last_insert_rowid())
        }

    pub fn get_playlists(conn: &Connection) -> rusqlite::Result<Vec<PlaylistDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    p.id, 
                    p.name,
                    p.description,
                    COUNT(pi.id) as track_count
                FROM playlists p
                LEFT JOIN playlist_items pi ON p.id = pi.playlist_id
                GROUP BY p.id
                ORDER BY p.created_at ASC
            ")?;
            
            let rows = stmt.query_map([], |row| {
                Ok(PlaylistDTO {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    track_count: row.get(3)?,
                })
            })?;
            
            let mut result = Vec::new();
            for r in rows {
                result.push(r?);
            }
            Ok(result)
        }

    pub fn add_to_playlist(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<()> {
            let max_pos: Option<f64> = conn.query_row(
                "SELECT MAX(position) FROM playlist_items WHERE playlist_id = ?1",
                rusqlite::params![playlist_id],
                |row| row.get(0)
            ).unwrap_or(None);
            
            let next_pos = max_pos.unwrap_or(0.0) + 1.0;
            
            conn.execute(
                "INSERT INTO playlist_items (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
                rusqlite::params![playlist_id, track_id, next_pos],
            )?;
            Ok(())
        }

    pub fn get_playlist_tracks(conn: &Connection, playlist_id: i64) -> rusqlite::Result<Vec<TrackDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    t.id, 
                    t.title, 
                    (SELECT artist_id FROM track_artists WHERE track_id = t.id ORDER BY position LIMIT 1) AS artist_id,
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
                    t.album_id,
                    al.title AS album_title, 
                    m.duration_ms, 
                    m.file_ext, 
                    m.id AS media_file_id,
                    ft.track_id IS NOT NULL AS is_favorite,
                    al.cover_artwork_id,
                    m.file_size
                FROM playlist_items pi
                JOIN tracks t ON pi.track_id = t.id
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE pi.playlist_id = ?1
                ORDER BY pi.position ASC
            ")?;
            
            let rows = stmt.query_map([playlist_id], crate::repositories::map_track_row)?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
            conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
            Ok(())
        }

    pub fn remove_playlist_item(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<()> {
            conn.execute(
                "DELETE FROM playlist_items WHERE playlist_id = ?1 AND track_id = ?2",
                params![playlist_id, track_id],
            )?;
            Ok(())
        }

    pub fn add_folder_to_playlist(conn: &rusqlite::Connection, playlist_id: i64, source_id: i64, folder_path: &str) -> rusqlite::Result<()> {
            // media_files.normalized_path 存的是相对 source_root 的小写路径，
            // 但此处接收到的 folder_path 可能是绝对路径或大小写混合。
            // 这里采取保守策略：对原值做 LIKE 转义 + 前缀匹配。
            let pattern = format!("{}%", crate::repositories::escape_like(folder_path));
    
            // 1. 获取目标文件夹下所有的 track_id（排除已经在歌单里的，避免重复）
            let mut stmt = conn.prepare("
                 SELECT track_id
                 FROM media_files
                 WHERE source_id = ?1 AND track_id IS NOT NULL AND normalized_path LIKE ?2 ESCAPE '\\'
                 AND track_id NOT IN (SELECT track_id FROM playlist_items WHERE playlist_id = ?3)
                 ORDER BY normalized_path ASC
            ")?;
    
            let track_ids: Vec<i64> = stmt.query_map(rusqlite::params![source_id, pattern, playlist_id], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
    
            if track_ids.is_empty() {
                return Ok(());
            }
    
            // 2. 依次插入，确保分配 position（复用同一事务）
            let tx = conn.unchecked_transaction()?;
            let max_pos: Option<f64> = tx.query_row(
                "SELECT MAX(position) FROM playlist_items WHERE playlist_id = ?1",
                rusqlite::params![playlist_id],
                |row| row.get(0)
            ).unwrap_or(None);
    
            let mut next_pos = max_pos.unwrap_or(0.0) + 1.0;
    
            {
                let mut insert_stmt = tx.prepare("INSERT INTO playlist_items (playlist_id, track_id, position) VALUES (?1, ?2, ?3)")?;
                for track_id in track_ids {
                    insert_stmt.execute(rusqlite::params![playlist_id, track_id, next_pos])?;
                    next_pos += 1.0;
                }
            }
            tx.commit()?;
    
            Ok(())
        }

}
