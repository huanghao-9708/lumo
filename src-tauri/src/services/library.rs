use crate::models::{TrackDTO, AlbumDTO, ArtistDTO, PlaylistDTO, ArtistStatsDTO};
use crate::services::metadata::AudioMetadata;
use rusqlite::{Connection, params, OptionalExtension};
use sha2::{Sha256, Digest};
use std::fs;

/// 归一化艺人/标题字符串：去掉首尾空白、折叠中间多个空白为单个空格。
/// 仅用于展示与去重的"原值"清理；做唯一键时再额外 `.to_lowercase()`。
fn normalize_artist_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// 转义 SQLite LIKE 模式串中的特殊字符（`%` / `_` / `\`），避免路径里这些字符被当通配符。
/// 返回 (escaped_pattern, esc)，调用方需配合 `LIKE ? ESCAPE '\'` 使用。
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// 提供本地曲库核心交互的服务类，处理所有文件入库解析以及前端歌曲数据的拉取
pub struct LibraryService;

impl LibraryService {
    /// 核心方法：将被扫描器发现的物理音频文件，经过元数据提取后，解析并存入到对应的数据表中
    /// - conn: SQLite 连接（位于事务中）
    /// - source_id: 此文件归属的扫描来源（如特定的本地文件夹）
    /// - source_root: 该来源的根目录，用于计算文件的相对路径（持久化在 `relative_path` 字段，
    ///                使得将来迁移根目录或支持 WebDAV 时只需调整 `root_uri` 即可）
    /// - path: 文件的绝对路径
    /// - metadata: 提取好的音频基础元数据（如艺术家、专辑名、比特率等）
    /// - app_data_dir: 用于缓存提取到的专辑封面图片的本地路径
    ///
    /// 关键设计：**同一个物理文件反复扫描不会创建新的 track**。本方法按
    /// `(album_id, normalized_title)` 查找已有 track 并复用；不存在时才插入。
    /// 这样歌单 / 收藏 / 播放历史等外键引用在重新扫描后仍然稳定。
    pub fn index_file(
        conn: &Connection,
        source_id: i64,
        source_root: &std::path::Path,
        path: &std::path::Path,
        metadata: &AudioMetadata,
        app_data_dir: &std::path::Path,
    ) -> rusqlite::Result<()> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

        // 相对于 source_root 的归一化相对路径（小写），用于 media_files 唯一约束
        let relative_path = path.strip_prefix(source_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let normalized_path = relative_path.to_lowercase();

        // 1. 处理艺人关联 (支持更多分隔符)
        // 优先使用 album_artist 标签作为整张专辑的归属艺人，
        // 否则回退到 track 级别的 artist。这样合辑会归到 "Various Artists" 而不是被切碎。
        let album_artist_str = metadata.album_artist.as_deref()
            .or(metadata.artist.as_deref())
            .unwrap_or("Unknown Artist");
        let album_artist_id = Self::upsert_artist(conn, album_artist_str)?;

        // 用于 track_artists 多对多关联的是 track 级 artist
        let track_artist_str = metadata.artist.as_deref().unwrap_or(album_artist_str);
        let artist_ids = Self::split_and_upsert_artists(conn, track_artist_str)?;

        // 2. 处理封面图片并生成 hash 缓存防重复
        let artwork_id = Self::upsert_artwork(conn, metadata, app_data_dir)?;

        // 3. 处理专辑：按 (normalized_title, album_artist_id) 联合去重，
        //    避免不同艺人的同名专辑（"Greatest Hits" 之类）被错误合并。
        //    同时维护 artists.album_count 冗余字段（新建专辑时 +1）。
        let album_title = metadata.album.as_deref().unwrap_or("Unknown Album");
        let normalized_album_title = album_title.to_lowercase();
        let album_id: i64 = match conn.query_row(
            "SELECT id FROM albums WHERE normalized_title = ?1 AND (album_artist_id IS ?2 OR album_artist_id = ?2) LIMIT 1",
            params![normalized_album_title, album_artist_id],
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
                    "INSERT INTO albums (title, normalized_title, sort_title, album_artist_id, cover_artwork_id, track_count) VALUES (?1, ?2, ?2, ?3, ?4, 0)",
                    params![album_title, normalized_album_title, album_artist_id, artwork_id],
                )?;
                let new_id = conn.last_insert_rowid();
                // 同步维护 album_artist 的 album_count 冗余字段
                conn.execute(
                    "UPDATE artists SET album_count = album_count + 1 WHERE id = ?1",
                    params![album_artist_id],
                )?;
                new_id
            }
        };

        // 4. 处理 Track（歌曲抽象信息）
        let track_title = metadata.title.as_deref().unwrap_or(&file_name);
        let normalized_track_title = track_title.to_lowercase();

        // 复用已存在的 track（同一专辑内同名），避免重复扫描把歌单引用打成孤儿
        let (track_id, track_is_new): (i64, bool) = match conn.query_row(
            "SELECT id FROM tracks WHERE normalized_title = ?1 AND album_id IS ?2 LIMIT 1",
            params![normalized_track_title, album_id],
            |row| row.get(0),
        ).optional()? {
            Some(id) => (id, false),
            None => {
                conn.execute(
                    "INSERT INTO tracks (title, normalized_title, sort_title, album_id) VALUES (?1, ?2, ?2, ?3)",
                    params![track_title, normalized_track_title, album_id],
                )?;
                (conn.last_insert_rowid(), true)
            }
        };

        // 维护 albums.track_count 冗余字段：仅在新 track 真正插入时 +1。
        // 复用已有 track（如重新扫描同一文件）不会重复计数。
        if track_is_new {
            conn.execute(
                "UPDATE albums SET track_count = track_count + 1 WHERE id = ?1",
                params![album_id],
            )?;
        }

        // 5. 插入多对多映射关系：将刚才提取出的所有艺人与该曲目连接
        //    并维护 artists.track_count 冗余字段。
        //    INSERT OR IGNORE 配合 changes() 判定：只有真正插入新关联行时才 +1，
        //    重新扫描已存在的关联不会重复计数。
        for (idx, aid) in artist_ids.iter().enumerate() {
            let inserted = conn.execute(
                "INSERT OR IGNORE INTO track_artists (track_id, artist_id, role, position) VALUES (?1, ?2, 'main', ?3)",
                params![track_id, aid, idx as i64],
            )?;
            if inserted > 0 {
                conn.execute(
                    "UPDATE artists SET track_count = track_count + 1 WHERE id = ?1",
                    params![aid],
                )?;
            }
        }

        // 6. 插入具体的 MediaFile 物理文件记录 (同一首歌可能存在不同品质的多个物理文件)
        // 使用 UPSERT 确保同一来源下的同一文件只更新不重复新增。
        // 注意：on conflict 时仅刷新可变属性（时长/比特率/可见时间），
        // 不要把 track_id 改回自身之外的其他值，保持引用稳定。
        conn.execute(
            "INSERT INTO media_files (
                source_id, track_id, relative_path, normalized_path, file_name, file_ext, duration_ms, bitrate, sample_rate, channels, last_seen_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
            ON CONFLICT(source_id, normalized_path) DO UPDATE SET
                track_id=excluded.track_id,
                duration_ms=excluded.duration_ms,
                bitrate=excluded.bitrate,
                sample_rate=excluded.sample_rate,
                channels=excluded.channels,
                last_seen_at=datetime('now'),
                availability='available',
                scan_error=NULL",
            params![
                source_id,
                track_id,
                relative_path,
                normalized_path,
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
            params![source_id, normalized_path],
            |row| row.get(0),
        )?;

        // 7. 将刚刚存储成功的最优物理文件作为此歌曲的首选音源
        conn.execute("UPDATE tracks SET primary_file_id = ?1 WHERE id = ?2", params![media_file_id, track_id])?;

        // 8. 尝试提取歌词存入 lyrics 表中（优先使用同目录下同名 LRC 文件，没有则使用内嵌歌词）
        let mut lrc_content = None;
        let mut lrc_path = path.with_extension("lrc");
        if !lrc_path.exists() {
            lrc_path = path.with_extension("LRC");
        }
        if lrc_path.exists() && lrc_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&lrc_path) {
                lrc_content = Some(content);
            }
        }
        if lrc_content.is_none() {
            lrc_content = metadata.lyrics.clone();
        }

        if let Some(content) = lrc_content {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO lyrics (track_id, media_file_id, format, synced, content, source) VALUES (?1, ?2, 'lrc', 1, ?3, 'local')",
                params![track_id, media_file_id, content],
            );
        }

        Ok(())
    }

    /// 把单个艺人字符串归一化后写入 artists 表（如不存在则插入），返回其 ID。
    /// 归一化策略：trim + 折叠连续空白 + 转小写，作为 normalized_name 去重键。
    fn upsert_artist(conn: &Connection, raw_name: &str) -> rusqlite::Result<i64> {
        let name = normalize_artist_name(raw_name);
        let normalized = name.to_lowercase();
        conn.execute(
            "INSERT OR IGNORE INTO artists (name, normalized_name, sort_name) VALUES (?1, ?2, ?2)",
            params![name, normalized],
        )?;
        conn.query_row(
            "SELECT id FROM artists WHERE normalized_name = ?1 LIMIT 1",
            params![normalized],
            |row| row.get(0),
        )
    }

    /// 切分多位艺人字符串（支持 feat./&/;/、等分隔符），逐个 upsert 并返回 ID 列表。
    /// 若解析结果为空（全是分隔符或空白），则回退使用 "Unknown Artist"。
    fn split_and_upsert_artists(conn: &Connection, raw: &str) -> rusqlite::Result<Vec<i64>> {
        let cleaned = raw
            .replace(" feat. ", "/")
            .replace(" ft. ", "/")
            .replace(" Feat. ", "/")
            .replace(" Ft. ", "/")
            .replace(" & ", "/")
            .replace('&', "/")
            .replace(';', "/")
            .replace('；', "/")
            .replace('、', "/")
            .replace('，', "/")
            .replace(',', "/");

        let mut ids = Vec::new();
        for part in cleaned.split('/').map(str::trim).filter(|s| !s.is_empty()) {
            ids.push(Self::upsert_artist(conn, part)?);
        }
        if ids.is_empty() {
            ids.push(Self::upsert_artist(conn, "Unknown Artist")?);
        }
        Ok(ids)
    }

    /// 处理封面图片：基于 SHA256 内容哈希去重，避免把同一张封面写多份。
    fn upsert_artwork(conn: &Connection, metadata: &AudioMetadata, app_data_dir: &std::path::Path) -> rusqlite::Result<Option<i64>> {
        let picture_data = match &metadata.picture_data {
            Some(d) if !d.is_empty() => d,
            _ => return Ok(None),
        };

        let mut hasher = Sha256::new();
        hasher.update(picture_data);
        let hash = hex::encode(hasher.finalize());

        // 同一张封面若已存在，直接复用其 ID
        if let Some(id) = conn.query_row(
            "SELECT id FROM artwork WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        ).optional()? {
            return Ok(Some(id));
        }

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

        let cache_path = artworks_dir.join(format!("{}.{}", hash, ext));
        if !cache_path.exists() {
            let _ = fs::write(&cache_path, picture_data);
        }

        conn.execute(
            "INSERT INTO artwork (cache_path, mime_type, content_hash) VALUES (?1, ?2, ?3)",
            params![cache_path.to_string_lossy().to_string(), metadata.picture_mime, hash],
        )?;
        Ok(Some(conn.last_insert_rowid()))
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
                ft.track_id IS NOT NULL AS is_favorite,
                al.cover_artwork_id
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
    ///
    /// 性能要点：
    /// 直接读 `albums.track_count` 冗余字段，不再用子查询或 GROUP BY 算歌曲数。
    /// 覆盖索引 `idx_albums_normalized_title_covering(normalized_title, album_artist_id,
    /// cover_artwork_id, track_count)` 让整个查询几乎不回表，O(LIMIT) 而非 O(全部专辑)。
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

    /// 获取所有参演或拥有的艺人列表
    pub fn get_artists_paginated(conn: &Connection, limit: u32, offset: u32, search_keyword: Option<String>) -> rusqlite::Result<Vec<ArtistDTO>> {
        // 直接读 artists.track_count 冗余字段（迁移 V2 维护），去掉子查询。
        let mut sql = "
            SELECT
                ar.id,
                ar.name,
                ar.track_count
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

        // 用 normalized_name（已有索引）代替 COLLATE NOCASE，避免函数排序
        sql.push_str(" ORDER BY ar.normalized_name ASC LIMIT ? OFFSET ?");
        
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
    pub fn create_playlist(conn: &Connection, name: &str, description: Option<&str>) -> rusqlite::Result<i64> {
        conn.execute(
            "INSERT INTO playlists (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取所有的用户自建歌单及其中包含的歌曲数量
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
                ft.track_id IS NOT NULL AS is_favorite,
                al.cover_artwork_id
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
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id
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
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, 1 AS is_favorite, al.cover_artwork_id
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
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id
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
        // 直接读 albums.track_count 冗余字段，去掉子查询和 GROUP BY。
        // 注意 DISTINCT + JOIN track_artists 会让同一专辑出现多次，仍需 GROUP BY al.id 去重，
        // 但因为不再 COUNT(t.id)，分组本身极快（不需扫描 track_artists 的全部行）。
        let mut stmt = conn.prepare("
            SELECT
                al.id, al.title, ar.name AS artist_name, al.cover_artwork_id, al.track_count
            FROM albums al
            LEFT JOIN artists ar ON al.album_artist_id = ar.id
            WHERE al.album_artist_id = ?1
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
                al.title AS album_title, m.duration_ms, m.file_ext, m.id AS media_file_id, ft.track_id IS NOT NULL AS is_favorite, al.cover_artwork_id
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
            cover_artwork_id: row.get(8)?,
        })
    }

    /// 删除歌单
    pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
        Ok(())
    }

    /// 从歌单中移除一首歌曲
    pub fn remove_playlist_item(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<()> {
        conn.execute(
            "DELETE FROM playlist_items WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }

    /// 保存播放队列：全删全插，整体放在单事务里以保证原子性
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

    /// 获取持久化的播放队列关联详情
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
            JOIN media_files m ON t.id = m.track_id
            LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
            ORDER BY pq.position ASC
        ";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], Self::map_track_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    /// 获取物理文件夹下的内容并结合数据库查询出对应的歌曲信息。
    ///
    /// 关于分页：文件系统读取目录是"一次性"的（`read_dir` 本身没有分页能力），
    /// 我们的做法是先把所有条目读出来并排序，再按 `limit/offset` 切片。
    /// 这样做的原因是：① 真实文件夹内条目数通常只有几十到几百，内存压力可控；
    /// ② 必须先排序才能保证分页结果稳定（否则每页顺序不一致）。
    ///
    /// 返回的 `total` 是该目录的原始条目总数，前端用 `entries.len() < total` 判断是否还有更多。
    /// 当 `limit` 为 None 时返回全部（保持向后兼容）。
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

    /// 通过绝对物理路径精确定位数据库中保存的一首歌
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
        conn.query_row(sql, rusqlite::params![source_id, path], Self::map_track_row)
    }

    /// 递归将一个目录下的所有已索引歌曲添加到歌单。
    /// 路径匹配使用转义的 LIKE，避免路径里的 `_` / `%` 被当作通配符。
    pub fn add_folder_to_playlist(conn: &rusqlite::Connection, playlist_id: i64, source_id: i64, folder_path: &str) -> rusqlite::Result<()> {
        // media_files.normalized_path 存的是相对 source_root 的小写路径，
        // 但此处接收到的 folder_path 可能是绝对路径或大小写混合。
        // 这里采取保守策略：对原值做 LIKE 转义 + 前缀匹配。
        let pattern = format!("{}%", escape_like(folder_path));

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
