use crate::services::metadata::AudioMetadata;
use rusqlite::{Connection, params, OptionalExtension};

/// 扫描 worker 预处理好的封面信息：哈希、缓存路径、缩略图全部在事务外算好，
/// 使写库事务内只剩纯 SQL（不再有 SHA256 / 图片解码 / 磁盘写入拉长事务）。
#[derive(Debug, Default)]
pub struct PreparedArtwork {
    /// 封面内容 SHA256（有封面时必有）
    pub hash: Option<String>,
    /// 封面 MIME（INSERT 时落库；缺失时路径回退用 jpg 后缀）
    pub mime: Option<String>,
    /// 原图缓存文件路径：本扫描内首次遇到该 hash 的文件才有（写入也是它做的）
    pub cache_path: Option<std::path::PathBuf>,
    /// 200x200 JPEG 缩略图（仅首见文件生成；解码失败为 None，前端回退 lumo://artwork）
    pub thumbnail: Option<Vec<u8>>,
}

/// 提取 + 预处理完成、随时可以进写库事务的一个文件。
#[derive(Debug)]
pub struct PreparedFile {
    pub path: std::path::PathBuf,
    pub metadata: AudioMetadata,
    pub mtime: i64,
    pub size: i64,
    pub artwork: PreparedArtwork,
    /// 同目录同名 .lrc 的预读内容（None = 没有 lrc 或远程源；index_file 内再回退内嵌歌词）
    pub lrc_content: Option<String>,
}

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
        prepared: &PreparedFile,
        app_data_dir: &std::path::Path,
    ) -> rusqlite::Result<()> {
        let path = &prepared.path;
        let metadata = &prepared.metadata;
        let mtime = prepared.mtime;
        let file_size = prepared.size;

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
        let album_artist_ids = Self::split_and_upsert_artists(conn, album_artist_str)?;
        let album_artist_id = *album_artist_ids.first()
            .unwrap_or(&Self::upsert_artist(conn, "Unknown Artist")?);

        // 用于 track_artists 多对多关联的是 track 级 artist
        let track_artist_str = metadata.artist.as_deref().unwrap_or(album_artist_str);
        let artist_ids = Self::split_and_upsert_artists(conn, track_artist_str)?;

        // 2. 处理封面图片：哈希/写盘/缩略图已由扫描 worker 在事务外完成，这里只剩纯 SQL
        let artwork_id = Self::upsert_artwork(conn, &prepared.artwork, app_data_dir)?;

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
                // 同步维护所有 album_artist 的 album_count 冗余字段
                for aid in &album_artist_ids {
                    conn.execute(
                        "UPDATE artists SET album_count = album_count + 1 WHERE id = ?1",
                        params![aid],
                    )?;
                }
                new_id
            }
        };

        // 将专辑与所有拆分后的艺人关联写入 album_artists 多对多表
        for (idx, aid) in album_artist_ids.iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO album_artists (album_id, artist_id, role, position) VALUES (?1, ?2, 'album_artist', ?3)",
                params![album_id, aid, idx as i64],
            )?;
        }

        // 4. 处理 Track（歌曲抽象信息）
        let track_title = metadata.title.as_deref().unwrap_or(&file_name);
        let normalized_track_title = track_title.to_lowercase();
        let main_artist_normalized = normalize_artist_name(track_artist_str).to_lowercase();

        // 查找已有 track 的逻辑（多音源归并）
        // 1. 同一专辑内同名完全匹配
        let exact_match: Option<i64> = conn.query_row(
            "SELECT id FROM tracks WHERE normalized_title = ?1 AND album_id IS ?2 LIMIT 1",
            params![normalized_track_title, album_id],
            |row| row.get(0),
        ).optional()?;

        // 2. 指纹模糊匹配（标题一致 + 主艺人一致 + 时长相差不到2秒）
        let fuzzy_match: Option<i64> = if exact_match.is_none() {
            conn.query_row(
                "SELECT t.id FROM tracks t
                 JOIN media_files mf ON mf.id = t.primary_file_id
                 WHERE t.normalized_title = ?1
                   AND EXISTS (SELECT 1 FROM track_artists ta
                               JOIN artists a ON a.id = ta.artist_id
                               WHERE ta.track_id = t.id AND ta.role = 'main'
                                 AND a.normalized_name = ?2)
                   AND ABS(COALESCE(mf.duration_ms, 0) - ?3) <= 2000
                 LIMIT 1",
                params![normalized_track_title, main_artist_normalized, metadata.duration_ms.unwrap_or(0)],
                |row| row.get(0),
            ).optional()?
        } else {
            None
        };

        let (track_id, track_is_new): (i64, bool) = if let Some(id) = exact_match.or(fuzzy_match) {
            (id, false)
        } else {
            conn.execute(
                "INSERT INTO tracks (title, normalized_title, sort_title, album_id) VALUES (?1, ?2, ?2, ?3)",
                params![track_title, normalized_track_title, album_id],
            )?;
            (conn.last_insert_rowid(), true)
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
        // 注意：on conflict 时仅刷新可变属性（时长/比特率/可见时间等），
        // 不要把 track_id 改回自身之外的其他值，保持引用稳定。
        conn.execute(
            "INSERT INTO media_files (
                source_id, track_id, relative_path, normalized_path, file_name, file_ext, file_size, modified_at, duration_ms, bitrate, sample_rate, channels, last_seen_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))
            ON CONFLICT(source_id, normalized_path) DO UPDATE SET
                track_id=excluded.track_id,
                file_size=excluded.file_size,
                modified_at=excluded.modified_at,
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
                file_size,
                mtime.to_string(),
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
        let current_primary_file_id: Option<i64> = conn.query_row(
            "SELECT primary_file_id FROM tracks WHERE id = ?1",
            params![track_id],
            |row| row.get(0),
        ).optional()?.flatten();

        let should_update_primary = if let Some(current_id) = current_primary_file_id {
            if current_id == media_file_id {
                false
            } else {
                let curr_score: i32 = conn.query_row(
                    "SELECT s.kind, mf.file_ext FROM media_files mf JOIN sources s ON s.id = mf.source_id WHERE mf.id = ?1",
                    params![current_id],
                    |row| {
                        let kind: String = row.get(0)?;
                        let ext: String = row.get(1)?;
                        Ok(crate::services::file_priority::file_priority_score(&kind, &ext))
                    },
                ).unwrap_or(-1);

                let new_score: i32 = conn.query_row(
                    "SELECT kind FROM sources WHERE id = ?1",
                    params![source_id],
                    |row| {
                        let kind: String = row.get(0)?;
                        Ok(crate::services::file_priority::file_priority_score(&kind, &ext))
                    },
                ).unwrap_or(0);

                new_score > curr_score
            }
        } else {
            true
        };

        if should_update_primary {
            conn.execute("UPDATE tracks SET primary_file_id = ?1 WHERE id = ?2", params![media_file_id, track_id])?;
        }

        // 8. 歌词入库：优先扫描 worker 预读的同目录同名 LRC 文件，没有则回退内嵌歌词
        //   （文件系统读取已在 worker 完成，写事务内不再有磁盘 I/O）
        let lrc_content = prepared.lrc_content.clone().or_else(|| metadata.lyrics.clone());

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

    /// 处理封面图片：基于 worker 预计算的 SHA256 内容哈希去重，事务内只做 SQL。
    /// 原图写盘与缩略图生成已在扫描 worker 中完成（见 scanner::prepare_artwork）。
    fn upsert_artwork(
        conn: &Connection,
        artwork: &PreparedArtwork,
        app_data_dir: &std::path::Path,
    ) -> rusqlite::Result<Option<i64>> {
        let Some(hash) = &artwork.hash else { return Ok(None) };

        // 同一张封面若已存在，直接复用其 ID。
        // thumbnail_blob 为 NULL（老数据 / 清理缓存后）时用首见文件带来的缩略图补写，
        // 确保后续扫描能逐步修复缺失的缩略图。
        if let Some(id) = conn.query_row(
            "SELECT id FROM artwork WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        ).optional()? {
            if let Some(thumb) = &artwork.thumbnail {
                conn.execute(
                    "UPDATE artwork SET thumbnail_blob = ?1 WHERE id = ?2 AND thumbnail_blob IS NULL",
                    params![thumb, id],
                )?;
            }
            return Ok(Some(id));
        }

        // 新封面：cache_path / thumbnail 均来自 worker 的预处理产物，纯 SQL 插入。
        // 同 hash 的首见文件必带 cache_path；若结果乱序导致非首见文件先到写库，
        // 用确定性路径（hash 命名）兜底——worker 写入的是同一个路径，不会错位。
        let cache_path = artwork.cache_path.clone()
            .unwrap_or_else(|| Self::artwork_cache_path(app_data_dir, artwork.mime.as_deref(), hash));

        conn.execute(
            "INSERT INTO artwork (cache_path, mime_type, content_hash, thumbnail_blob) VALUES (?1, ?2, ?3, ?4)",
            params![cache_path.to_string_lossy().to_string(), artwork.mime, hash, artwork.thumbnail],
        )?;
        Ok(Some(conn.last_insert_rowid()))
    }

    /// 封面缓存文件路径：{app_data_dir}/artworks/{hash}.{ext}（确定性命名）。
    /// 扫描 worker 与写库回退共用，保证同一 hash 在任何线程算出的路径一致。
    pub fn artwork_cache_path(
        app_data_dir: &std::path::Path,
        mime: Option<&str>,
        hash: &str,
    ) -> std::path::PathBuf {
        let ext = match mime {
            Some("image/png") => "png",
            Some("image/jpeg") | Some("image/jpg") => "jpg",
            Some("image/gif") => "gif",
            Some("image/webp") => "webp",
            _ => "jpg",
        };
        app_data_dir.join("artworks").join(format!("{}.{}", hash, ext))
    }

    /// 从原始图片字节生成 200x200 JPEG 缩略图（cover 模式：等比缩放后居中裁剪）。
    /// 失败时返回 None，不影响扫描流程（只是该封面没有内联缩略图，前端 fallback 到协议）。
    /// 设为 pub 是因为 db.rs 的 V4 迁移需要调用它来回填已有记录。
    ///
    /// 滤镜选择 Triangle(bilinear) 而非 Lanczos3：
    /// - Lanczos3 质量最高但极慢(每张 100-200ms),674 张要 60-130s
    /// - Triangle 质量足够(200x200 缩略图肉眼几乎无差别),快 3-5 倍(每张 20-50ms)
    /// - 缩略图本身就是为了网格视图小尺寸显示,不需要印刷级质量
    pub fn generate_thumbnail(image_data: &[u8]) -> Option<Vec<u8>> {
        use image::imageops::FilterType;

        // 解码原图（支持 JPEG / PNG / GIF / WebP 等，取决于 image crate 的 features）
        let img = image::load_from_memory(image_data).ok()?;

        // resize_to_fill：等比缩放到刚好覆盖 200x200，然后居中裁剪多余部分
        let thumb = img.resize_to_fill(200, 200, FilterType::Triangle);

        // 编码为 JPEG（质量 80，平衡文件大小 ~5-10KB 和视觉质量）
        let mut buf = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
        encoder.encode_image(&thumb).ok()?;
        Some(buf)
    }
}
