use crate::models::*;
use rusqlite::{params, Connection};

pub struct PlaylistRepo;

impl PlaylistRepo {
    pub fn create_playlist(
        conn: &Connection,
        name: &str,
        description: Option<&str>,
    ) -> rusqlite::Result<i64> {
        conn.execute(
            "INSERT INTO playlists (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 歌单列表。
    ///
    /// 封面策略（与主流播放器一致）：取歌单内**第一首歌曲**所属专辑的封面。
    /// 由于部分曲目可能没有专辑（或专辑没有封面），这里优先取「第一首有封面的曲目」，
    /// 全部都没有时才回落到第一首曲目的 album.cover_artwork_id（可能为 NULL）。
    /// 缩略图单独用主键查询 artwork 表，避免在一条 SQL 里做多层相关子查询。
    pub fn get_playlists(conn: &Connection) -> rusqlite::Result<Vec<PlaylistDTO>> {
        let mut stmt = conn.prepare(
            "
                SELECT
                    p.id,
                    p.name,
                    p.description,
                    COUNT(pi.id) AS track_count,
                    COALESCE(
                        (SELECT al.cover_artwork_id
                           FROM playlist_items pi2
                           JOIN tracks t2 ON t2.id = pi2.track_id
                           JOIN albums al ON al.id = t2.album_id
                          WHERE pi2.playlist_id = p.id
                            AND al.cover_artwork_id IS NOT NULL
                          ORDER BY pi2.position ASC
                          LIMIT 1),
                        (SELECT al2.cover_artwork_id
                           FROM playlist_items pi3
                           JOIN tracks t3 ON t3.id = pi3.track_id
                           LEFT JOIN albums al2 ON al2.id = t3.album_id
                          WHERE pi3.playlist_id = p.id
                          ORDER BY pi3.position ASC
                          LIMIT 1)
                    ) AS cover_artwork_id
                FROM playlists p
                LEFT JOIN playlist_items pi ON p.id = pi.playlist_id
                GROUP BY p.id
                ORDER BY p.created_at ASC
            ",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?;

        // 先收集所有行，再把 stmt 归还给 conn，之后才能按 artwork id 查缩略图
        let base_rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let mut thumb_stmt = conn.prepare(
            "SELECT thumbnail_blob FROM artwork WHERE id = ?1",
        )?;

        let mut result = Vec::with_capacity(base_rows.len());
        for (id, name, description, track_count, cover_artwork_id) in base_rows {
            let cover_thumbnail_base64 = match cover_artwork_id {
                Some(aid) => thumb_stmt
                    .query_row(params![aid], |row| row.get::<_, Option<Vec<u8>>>(0))
                    .unwrap_or(None),
                None => None,
            };
            result.push(PlaylistDTO {
                id,
                name,
                description,
                track_count,
                cover_artwork_id,
                cover_thumbnail_base64: crate::repositories::thumbnail_to_data_url(
                    cover_thumbnail_base64,
                ),
            });
        }
        Ok(result)
    }

    pub fn add_to_playlist(
        conn: &Connection,
        playlist_id: i64,
        track_id: i64,
    ) -> rusqlite::Result<()> {
        let max_pos: Option<f64> = conn
            .query_row(
                "SELECT MAX(position) FROM playlist_items WHERE playlist_id = ?1",
                rusqlite::params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let next_pos = max_pos.unwrap_or(0.0) + 1.0;

        conn.execute(
            "INSERT INTO playlist_items (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, track_id, next_pos],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(
        conn: &Connection,
        playlist_id: i64,
    ) -> rusqlite::Result<Vec<TrackDTO>> {
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
                    m.file_size,
                    (SELECT s.kind FROM sources s JOIN media_files mf ON mf.source_id = s.id WHERE mf.id = m.id) AS source_kind
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
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
        Ok(())
    }

    pub fn remove_playlist_item(
        conn: &Connection,
        playlist_id: i64,
        track_id: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "DELETE FROM playlist_items WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }

    /// 批量添加歌曲到歌单（单事务）。
    /// playlist_items 无 (playlist_id, track_id) 唯一约束（设计上允许重复添加），
    /// 因此用 NOT IN 显式跳过已在歌单中的曲目；position 连续分配。
    /// 返回 (成功添加数, 跳过的重复数)。
    pub fn add_tracks_to_playlist(
        conn: &Connection,
        playlist_id: i64,
        track_ids: &[i64],
    ) -> rusqlite::Result<(usize, usize)> {
        if track_ids.is_empty() {
            return Ok((0, 0));
        }

        let tx = conn.unchecked_transaction()?;

        // 待插入列表：过滤掉已在歌单中的曲目（去重）
        let placeholders = track_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let dup_sql = format!(
            "SELECT track_id FROM playlist_items WHERE playlist_id = ? AND track_id IN ({placeholders})"
        );
        let existing: Vec<i64> = {
            let mut stmt = tx.prepare(&dup_sql)?;
            let rows = stmt.query_map(
                rusqlite::params_from_iter(std::iter::once(&playlist_id).chain(track_ids.iter())),
                |row| row.get(0),
            )?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let to_add: Vec<i64> = {
            let dup = std::collections::HashSet::<i64>::from_iter(existing);
            track_ids
                .iter()
                .copied()
                .filter(|id| !dup.contains(id))
                .collect()
        };
        let skipped = track_ids.len() - to_add.len();

        if !to_add.is_empty() {
            let max_pos: Option<f64> = tx
                .query_row(
                    "SELECT MAX(position) FROM playlist_items WHERE playlist_id = ?1",
                    rusqlite::params![playlist_id],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            let mut next_pos = max_pos.unwrap_or(0.0) + 1.0;

            let mut stmt = tx.prepare(
                "INSERT INTO playlist_items (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            )?;
            for track_id in &to_add {
                stmt.execute(rusqlite::params![playlist_id, track_id, next_pos])?;
                next_pos += 1.0;
            }
        }
        tx.commit()?;

        Ok((to_add.len(), skipped))
    }

    pub fn add_folder_to_playlist(
        conn: &rusqlite::Connection,
        playlist_id: i64,
        source_id: i64,
        folder_path: &str,
    ) -> rusqlite::Result<()> {
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

        let track_ids: Vec<i64> = stmt
            .query_map(rusqlite::params![source_id, pattern, playlist_id], |row| {
                row.get(0)
            })?
            .filter_map(Result::ok)
            .collect();

        if track_ids.is_empty() {
            return Ok(());
        }

        // 2. 依次插入，确保分配 position（复用同一事务）
        let tx = conn.unchecked_transaction()?;
        let max_pos: Option<f64> = tx
            .query_row(
                "SELECT MAX(position) FROM playlist_items WHERE playlist_id = ?1",
                rusqlite::params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let mut next_pos = max_pos.unwrap_or(0.0) + 1.0;

        {
            let mut insert_stmt = tx.prepare(
                "INSERT INTO playlist_items (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            )?;
            for track_id in track_ids {
                insert_stmt.execute(rusqlite::params![playlist_id, track_id, next_pos])?;
                next_pos += 1.0;
            }
        }
        tx.commit()?;

        Ok(())
    }
}
