use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO};
use crate::services::metadata::AudioMetadata;
use rusqlite::{Connection, params, OptionalExtension};
use sha2::{Sha256, Digest};
use std::fs;

/// 提供本地曲库核心交互的服务类，处理所有文件入库解析以及前端歌曲数据的拉取
pub struct LibraryService;

impl LibraryService {
    /// 核心方法：将被扫描器发现的物理音频文件，经过元数据提取后，解析并存入到对应的数据表中
    /// - conn: SQLite 连接
    /// - source_id: 此文件归属的扫描来源（如特定的本地文件夹）
    /// - path: 文件的绝对路径
    /// - metadata: 提取好的音频基础元数据（如艺术家、专辑名、比特率等）
    /// - app_data_dir: 用于缓存提取到的专辑封面图片的本地路径
    pub fn index_file(conn: &Connection, source_id: i64, path: &std::path::Path, metadata: &AudioMetadata, app_data_dir: &std::path::Path) -> rusqlite::Result<()> {
        let relative_path = path.to_string_lossy().to_string();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 1. 处理艺人关联 (支持更多分隔符)
        let artist_str = metadata.artist.as_deref().unwrap_or("Unknown Artist");
        let cleaned_str = artist_str
            .replace(" feat. ", "/")
            .replace(" ft. ", "/")
            .replace(" Feat. ", "/")
            .replace(" Ft. ", "/")
            .replace(" & ", "/")
            .replace("&", "/")
            .replace(";", "/")
            .replace("；", "/")
            .replace("、", "/")
            .replace("，", "/")
            .replace(",", "/");
        let artist_names: Vec<&str> = cleaned_str.split('/').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut artist_ids = Vec::new();

        for aname in &artist_names {
            // 插入艺人表，并提取自增 ID
            conn.execute(
                "INSERT OR IGNORE INTO artists (name, normalized_name, sort_name) VALUES (?1, ?2, ?2)",
                params![aname, aname.to_lowercase()],
            )?;
            let aid: i64 = conn.query_row(
                "SELECT id FROM artists WHERE normalized_name = ?1 LIMIT 1",
                params![aname.to_lowercase()],
                |row| row.get(0),
            )?;
            artist_ids.push(aid);
        }

        // 如果没有单独指定专辑艺人，则默认使用切分出来的第一个艺人
        let album_artist_id = artist_ids.first().cloned();

        // 2. 处理封面图片并生成 hash 缓存防重复
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
            
            let file_name_art = format!("{}.{}", hash, ext);
            let cache_path = artworks_dir.join(&file_name_art);
            
            // 检查同一张封面是否已经在数据库中
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

        // 3. 处理专辑
        let album_title = metadata.album.as_deref().unwrap_or("Unknown Album");
        let album_id: i64 = match conn.query_row(
            "SELECT id FROM albums WHERE normalized_title = ?1 LIMIT 1",
            params![album_title.to_lowercase()],
            |row| row.get(0),
        ).optional()? {
            Some(id) => {
                // 如果发现专辑没有封面而现在扫描到了新封面，补充进去
                if let Some(aid) = artwork_id {
                    conn.execute(
                        "UPDATE albums SET cover_artwork_id = ?1 WHERE id = ?2 AND cover_artwork_id IS NULL",
                        params![aid, id],
                    )?;
                }
                id
            },
            None => {
                // 插入新专辑
                conn.execute(
                    "INSERT INTO albums (title, normalized_title, sort_title, album_artist_id, cover_artwork_id) VALUES (?1, ?2, ?2, ?3, ?4)",
                    params![album_title, album_title.to_lowercase(), album_artist_id, artwork_id],
                )?;
                conn.last_insert_rowid()
            }
        };

        // 4. 插入作为核心实体的 Track（歌曲抽象信息）
        let track_title = metadata.title.as_deref().unwrap_or(&file_name);
        conn.execute(
            "INSERT INTO tracks (title, normalized_title, sort_title, album_id) VALUES (?1, ?2, ?2, ?3)",
            params![track_title, track_title.to_lowercase(), album_id],
        )?;
        let track_id = conn.last_insert_rowid();

        // 5. 插入多对多映射关系：将刚才提取出的所有艺人与该曲目连接
        for (idx, aid) in artist_ids.iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO track_artists (track_id, artist_id, role, position) VALUES (?1, ?2, 'main', ?3)",
                params![track_id, aid, idx as i64],
            )?;
        }

        // 6. 插入具体的 MediaFile 物理文件记录 (同一首歌可能存在不同品质的多个物理文件)
        // 使用 UPSERT 确保同一来源下的同名文件只更新不重复新增
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

        // 回查出刚才被更新或插入的媒体文件 ID
        let media_file_id: i64 = conn.query_row(
            "SELECT id FROM media_files WHERE source_id = ?1 AND normalized_path = ?2",
            params![source_id, relative_path],
            |row| row.get(0),
        )?;
        
        // 7. 将刚刚存储成功的最优物理文件作为此歌曲的首选音源
        conn.execute("UPDATE tracks SET primary_file_id = ?1 WHERE id = ?2", params![media_file_id, track_id])?;

        Ok(())
    }

    /// 查询"所有歌曲"视图，支持分页拉取和关键词模糊过滤
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
                ft.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
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
        sql.push_str(" ORDER BY t.added_at DESC LIMIT ? OFFSET ?");

        let mut result = Vec::new();

        if let Some(pattern) = keyword_pattern {
            let mut stmt = conn.prepare(&sql)?;
            let tracks = stmt.query_map(params![pattern, pattern, pattern, limit, offset], Self::map_track_row)?;
            for t in tracks { result.push(t?); }
        } else {
            let mut stmt = conn.prepare(&sql)?;
            let tracks = stmt.query_map(params![limit, offset], Self::map_track_row)?;
            for t in tracks { result.push(t?); }
        }

        Ok(result)
    }

    /// 获取分组后的所有专辑列表并分页
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

    /// 获取所有参演或拥有的艺人列表
    pub fn get_artists_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<ArtistDTO>> {
        let mut sql = "
            SELECT 
                ar.id, 
                ar.name,
                (SELECT COUNT(DISTINCT t.id) FROM track_artists ta JOIN tracks t ON ta.track_id = t.id WHERE ta.artist_id = ar.id) as track_count
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

    /// 切换某一首歌曲的心型收藏状态
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

    /// 创建一个新的空歌单
    pub fn create_playlist(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
        conn.execute(
            "INSERT INTO playlists (name) VALUES (?1)",
            params![name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取所有的用户自建歌单及其中包含的歌曲数量
    pub fn get_playlists(conn: &Connection) -> rusqlite::Result<Vec<PlaylistDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                p.id, 
                p.name,
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
                track_count: row.get(2)?,
            })
        })?;
        
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    /// 将指定的一首歌加入到目标歌单末尾，并赋予其一个 position 用于以后拖拽排序
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

    /// 按照添加到歌单的顺序（position）拉取某一特定歌单中的全部歌曲
    pub fn get_playlist_tracks(conn: &Connection, playlist_id: i64) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, 
                t.title, 
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
                al.title AS album_title, 
                m.duration_ms, 
                m.file_ext, 
                m.id AS media_file_id,
                ft.track_id IS NOT NULL AS is_favorite
            FROM playlist_items pi
            JOIN tracks t ON pi.track_id = t.id
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
            WHERE pi.playlist_id = ?1
            ORDER BY pi.position ASC
        ")?;
        
        let rows = stmt.query_map([playlist_id], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 每当歌曲发生播放时调用此接口，记录播放时间到历史流水中
    pub fn record_play(conn: &Connection, track_id: i64) -> rusqlite::Result<()> {
        // 更新总体计数字段
        conn.execute(
            "UPDATE tracks SET play_count = play_count + 1, last_played_at = datetime('now') WHERE id = ?1",
            rusqlite::params![track_id],
        )?;
        
        let media_file_id: Option<i64> = conn.query_row("SELECT primary_file_id FROM tracks WHERE id = ?1", params![track_id], |row| row.get(0)).unwrap_or(None);
        
        // 生成流水账单记录，用于复杂的统计
        conn.execute(
            "INSERT INTO play_history (track_id, media_file_id, source_kind) VALUES (?1, ?2, 'local')",
            params![track_id, media_file_id],
        )?;
        Ok(())
    }

    /// 获取最近播放的歌曲视图，按照最后播放时间降序排列
    pub fn get_recently_played(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, t.title, 
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name, 
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
            WHERE t.last_played_at IS NOT NULL
            ORDER BY t.last_played_at DESC LIMIT ?1
        ")?;
        let rows = stmt.query_map([limit], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 获取所有被收藏喜欢过的歌曲，按心型按钮点击的时间降序
    pub fn get_favorite_tracks(conn: &Connection) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, t.title, 
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name, 
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, 1 AS is_favorite
            FROM favorite_tracks ft
            JOIN tracks t ON ft.track_id = t.id
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
            ORDER BY ft.favorited_at DESC
        ")?;
        let rows = stmt.query_map([], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 获取指定专辑下的所有歌曲
    pub fn get_album_tracks(conn: &Connection, album_id: i64) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, t.title, 
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta2 JOIN artists a ON ta2.artist_id = a.id WHERE ta2.track_id = t.id ORDER BY ta2.position) AS artist_name,
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
            WHERE t.album_id = ?1
            ORDER BY t.disc_no ASC, t.track_no ASC, t.title ASC
        ")?;
        let rows = stmt.query_map([album_id], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 获取指定艺人参与的所有专辑
    pub fn get_artist_albums(conn: &Connection, artist_id: i64, limit: u32, offset: u32) -> rusqlite::Result<Vec<AlbumDTO>> {
        let mut stmt = conn.prepare("
            SELECT DISTINCT
                al.id, al.title, ar.name AS artist_name, al.cover_artwork_id,
                (SELECT COUNT(t2.id) FROM tracks t2 WHERE t2.album_id = al.id) as track_count
            FROM albums al
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            LEFT JOIN tracks t ON t.album_id = al.id
            LEFT JOIN track_artists ta ON ta.track_id = t.id
            WHERE al.album_artist_id = ?1 OR ta.artist_id = ?1
            GROUP BY al.id
            ORDER BY al.release_year DESC, al.title ASC
            LIMIT ?2 OFFSET ?3
        ")?;
        let rows = stmt.query_map(rusqlite::params![artist_id, limit, offset], |row| {
            Ok(AlbumDTO {
                id: row.get(0)?,
                title: row.get(1)?,
                artist_name: row.get(2)?,
                cover_artwork_id: row.get(3)?,
                track_count: row.get(4)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 获取指定艺人名下的歌曲分页
    pub fn get_artist_tracks(conn: &Connection, artist_id: i64, limit: u32, offset: u32) -> rusqlite::Result<Vec<TrackDTO>> {
        let mut stmt = conn.prepare("
            SELECT 
                t.id, t.title, 
                (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta2 JOIN artists a ON ta2.artist_id = a.id WHERE ta2.track_id = t.id ORDER BY ta2.position) AS artist_name,
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite
            FROM tracks t
            JOIN track_artists ta ON ta.track_id = t.id
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
            WHERE ta.artist_id = ?1
            ORDER BY t.play_count DESC, t.title ASC
            LIMIT ?2 OFFSET ?3
        ")?;
        let rows = stmt.query_map(rusqlite::params![artist_id, limit, offset], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows { result.push(r?); }
        Ok(result)
    }

    /// 获取指定艺人的总体统计（歌曲数，专辑数）
    pub fn get_artist_stats(conn: &Connection, artist_id: i64) -> rusqlite::Result<ArtistStatsDTO> {
        let track_count: i64 = conn.query_row(
            "SELECT COUNT(t.id) FROM tracks t JOIN track_artists ta ON ta.track_id = t.id WHERE ta.artist_id = ?1",
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

    /// 提供统一的闭包将 SQLite 查询出的一行转化为前端期望的 DTO 模型
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
