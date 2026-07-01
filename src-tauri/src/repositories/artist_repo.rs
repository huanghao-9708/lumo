use rusqlite::{Connection, params};
use crate::models::*;

pub struct ArtistRepo;

impl ArtistRepo {
    pub fn get_artists_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<ArtistListResult> {
            // 直接读 artists.track_count 冗余字段（迁移 V2 维护），去掉子查询。
            let mut sql = "
                SELECT
                    ar.id,
                    ar.name,
                    ar.track_count,
                    ar.avatar_artwork_id
                FROM artists ar
                WHERE 1=1
            ".to_string();

            let mut count_sql = "
                SELECT COUNT(*) FROM artists ar WHERE 1=1
            ".to_string();

            let keyword_pattern = if let Some(keyword) = search_keyword {
                let kw = keyword.trim();
                if !kw.is_empty() {
                    let clause = " AND ar.name LIKE ?";
                    sql.push_str(clause);
                    count_sql.push_str(clause);
                    Some(format!("%{}%", kw))
                } else { None }
            } else { None };

            // 用 normalized_name（已有索引）代替 COLLATE NOCASE，避免函数排序
            // 加 ar.id 作 tiebreaker，避免相同 normalized_name 时分页结果重叠
            sql.push_str(" ORDER BY ar.normalized_name ASC, ar.id ASC LIMIT ? OFFSET ?");

            let total: i64 = if let Some(pattern) = &keyword_pattern {
                let mut stmt = conn.prepare(&count_sql)?;
                stmt.query_row(params![pattern], |row| row.get(0))?
            } else {
                let mut stmt = conn.prepare(&count_sql)?;
                stmt.query_row([], |row| row.get(0))?
            };

            let mut result = Vec::new();
            if let Some(pattern) = keyword_pattern {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(params![pattern, limit, offset], |row| {
                    Ok(ArtistDTO {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        track_count: row.get(2)?,
                        avatar_artwork_id: row.get(3)?,
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
                        avatar_artwork_id: row.get(3)?,
                    })
                })?;
                for r in rows { result.push(r?); }
            }
            Ok(ArtistListResult { artists: result, total })
        }

    pub fn get_artist_album_count(conn: &Connection, artist_id: i64) -> rusqlite::Result<i64> {
        let mut stmt = conn.prepare("
            SELECT COUNT(*) FROM (
                SELECT al.id
                FROM albums al
                WHERE EXISTS (SELECT 1 FROM album_artists aa WHERE aa.album_id = al.id AND aa.artist_id = ?1)
                   OR al.id IN (
                       SELECT DISTINCT t.album_id
                       FROM track_artists ta JOIN tracks t ON ta.track_id = t.id
                       WHERE ta.artist_id = ?1 AND t.album_id IS NOT NULL
                   )
                GROUP BY al.id
            )
        ")?;
        let count: i64 = stmt.query_row(rusqlite::params![artist_id], |row| row.get(0))?;
        Ok(count)
    }

    pub fn get_artist_albums(conn: &Connection, artist_id: i64, limit: u32, offset: u32) -> rusqlite::Result<Vec<AlbumDTO>> {
            // 直接读 albums.track_count 冗余字段，去掉子查询和 GROUP BY。
            // 注意 DISTINCT + JOIN track_artists 会让同一专辑出现多次，仍需 GROUP BY al.id 去重，
            // 但因为不再 COUNT(t.id)，分组本身极快（不需扫描 track_artists 的全部行）。
            let mut stmt = conn.prepare("
                SELECT
                    al.id, al.title,
                    (SELECT GROUP_CONCAT(aa2.name, ', ') FROM album_artists aa1 JOIN artists aa2 ON aa1.artist_id = aa2.id WHERE aa1.album_id = al.id ORDER BY aa1.position) AS artist_name,
                    al.cover_artwork_id, al.track_count, aw.thumbnail_blob
                FROM albums al
                LEFT JOIN artwork aw ON al.cover_artwork_id = aw.id
                WHERE EXISTS (SELECT 1 FROM album_artists aa WHERE aa.album_id = al.id AND aa.artist_id = ?1)
                   OR al.id IN (
                       SELECT DISTINCT t.album_id
                       FROM track_artists ta JOIN tracks t ON ta.track_id = t.id
                       WHERE ta.artist_id = ?1 AND t.album_id IS NOT NULL
                   )
                GROUP BY al.id
                ORDER BY al.release_year DESC, al.title ASC
                LIMIT ?2 OFFSET ?3
            ")?;
            let rows = stmt.query_map(rusqlite::params![artist_id, limit, offset], |row| {
                let thumb: Option<Vec<u8>> = row.get(5)?;
                let cover_thumbnail_base64 = thumb.map(|b| {
                    use base64::{engine::general_purpose, Engine as _};
                    format!("data:image/jpeg;base64,{}", general_purpose::STANDARD.encode(&b))
                });
                Ok(AlbumDTO {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist_name: row.get(2)?,
                    cover_artwork_id: row.get(3)?,
                    track_count: row.get(4)?,
                    cover_thumbnail_base64,
                })
            })?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn get_artist_tracks(conn: &Connection, artist_id: i64, limit: u32, offset: u32) -> rusqlite::Result<Vec<TrackDTO>> {
            let mut stmt = conn.prepare("
                SELECT 
                    t.id, t.title, 
                    (SELECT artist_id FROM track_artists WHERE track_id = t.id ORDER BY position LIMIT 1) AS artist_id,
                    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta2 JOIN artists a ON ta2.artist_id = a.id WHERE ta2.track_id = t.id ORDER BY ta2.position) AS artist_name,
                    t.album_id,
                    al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id, m.file_size,
                    (SELECT s.kind FROM sources s JOIN media_files mf ON mf.source_id = s.id WHERE mf.id = m.id) AS source_kind
                FROM tracks t
                JOIN track_artists ta ON ta.track_id = t.id
                LEFT JOIN albums al ON t.album_id = al.id
                JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
                LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
                WHERE ta.artist_id = ?1
                ORDER BY t.play_count DESC, t.title ASC
                LIMIT ?2 OFFSET ?3
            ")?;
            let rows = stmt.query_map(rusqlite::params![artist_id, limit, offset], crate::repositories::map_track_row)?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn get_artist_stats(conn: &Connection, artist_id: i64) -> rusqlite::Result<ArtistStatsDTO> {
            // track_count 直接读冗余字段（O(1)）。
            // album_count 由于语义包含"参与的"专辑（不只是作为 album_artist），
            // 与 artists.album_count 冗余字段定义不同，仍需一次查询。但单艺人涉及的专辑
            // 通常只有几十张，配合 idx_track_artists_artist 索引，耗时几毫秒可接受。
            let track_count: i64 = conn.query_row(
                "SELECT track_count FROM artists WHERE id = ?1",
                [artist_id],
                |row| row.get(0)
            ).unwrap_or(0);
    
            let album_count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT al.id) FROM albums al
                 LEFT JOIN tracks t ON t.album_id = al.id
                 LEFT JOIN track_artists ta ON ta.track_id = t.id
                 WHERE al.album_artist_id = ?1 OR ta.artist_id = ?1",
                [artist_id],
                |row| row.get(0)
            ).unwrap_or(0);
    
            Ok(ArtistStatsDTO {
                track_count,
                album_count
            })
        }

    pub fn get_favorite_artists(conn: &Connection) -> rusqlite::Result<Vec<ArtistDTO>> {
            let mut stmt = conn.prepare("
                SELECT
                    ar.id, ar.name, ar.track_count, ar.avatar_artwork_id
                FROM favorite_artists fa
                JOIN artists ar ON fa.artist_id = ar.id
                ORDER BY fa.favorited_at DESC
            ")?;
            let rows = stmt.query_map([], |row| {
                Ok(ArtistDTO {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    track_count: row.get(2)?,
                    avatar_artwork_id: row.get(3)?,
                })
            })?;
            let mut result = Vec::new();
            for r in rows { result.push(r?); }
            Ok(result)
        }

    pub fn toggle_favorite_artist(conn: &Connection, artist_id: i64, is_favorite: bool) -> rusqlite::Result<()> {
            if is_favorite {
                conn.execute(
                    "INSERT OR IGNORE INTO favorite_artists (artist_id) VALUES (?1)",
                    params![artist_id],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM favorite_artists WHERE artist_id = ?1",
                    params![artist_id],
                )?;
            }
            Ok(())
        }

    pub fn get_artist_by_id(conn: &Connection, artist_id: i64) -> rusqlite::Result<Option<ArtistDTO>> {
        let mut stmt = conn.prepare("
            SELECT
                ar.id, ar.name, ar.track_count, ar.avatar_artwork_id
            FROM artists ar
            WHERE ar.id = ?1
        ")?;
        let mut rows = stmt.query_map(params![artist_id], |row| {
            Ok(ArtistDTO {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
                avatar_artwork_id: row.get(3)?,
            })
        })?;
        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

}
