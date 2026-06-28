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
        mtime: i64,
        file_size: i64,
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

        // 同一张封面若已存在，直接复用其 ID。
        // 但如果该记录的 thumbnail_blob 是 NULL（老数据 / V4 回填失败），
        // 补生成一次缩略图并 UPDATE，确保后续扫描能逐步修复缺失的缩略图。
        if let Some(id) = conn.query_row(
            "SELECT id FROM artwork WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        ).optional()? {
            // 检查 thumbnail_blob 是否需要补生成
            let needs_thumb: Option<Option<Vec<u8>>> = conn.query_row(
                "SELECT thumbnail_blob FROM artwork WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            ).ok();
            if let Some(None) = needs_thumb {
                if let Some(blob) = Self::generate_thumbnail(picture_data) {
                    let _ = conn.execute(
                        "UPDATE artwork SET thumbnail_blob = ?1 WHERE id = ?2",
                        params![blob, id],
                    );
                }
            }
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

        // 生成 200x200 JPEG 缩略图，存入 BLOB 供 library_get_albums 内联返回。
        // 这一步是消灭 N+1 封面请求的关键：网格视图不再需要逐个请求 lumo://artwork。
        let thumbnail_blob = Self::generate_thumbnail(picture_data);

        conn.execute(
            "INSERT INTO artwork (cache_path, mime_type, content_hash, thumbnail_blob) VALUES (?1, ?2, ?3, ?4)",
            params![cache_path.to_string_lossy().to_string(), metadata.picture_mime, hash, thumbnail_blob],
        )?;
        Ok(Some(conn.last_insert_rowid()))
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
