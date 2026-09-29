use crate::services::metadata::{normalize_artist_name, AudioMetadata};
use rusqlite::{params, Connection, OptionalExtension};

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

/// 提供本地曲库核心交互的服务类，处理所有文件入库解析以及前端歌曲数据的拉取
pub struct LibraryService;

impl LibraryService {
    /// 核心方法：将被扫描器发现的物理音频文件，经过元数据提取后，解析并存入到对应的数据表中
    /// - conn: SQLite 连接（位于事务中）
    /// - source_id: 此文件归属的扫描来源（如特定的本地文件夹）
    /// - source_root: 该来源的根目录，用于计算文件的相对路径（持久化在 `relative_path` 字段，
    ///   使得将来迁移根目录或支持 WebDAV 时只需调整 `root_uri` 即可）
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

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        // 相对于 source_root 的归一化相对路径（小写），用于 media_files 唯一约束
        let relative_path = path
            .strip_prefix(source_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let normalized_path = relative_path.to_lowercase();

        // 1. 处理艺人关联 (支持更多分隔符)
        // 优先使用 album_artist 标签作为整张专辑的归属艺人，
        // 否则回退到 track 级别的 artist。这样合辑会归到 "Various Artists" 而不是被切碎。
        let album_artist_str = metadata
            .album_artist
            .as_deref()
            .or(metadata.artist.as_deref())
            .unwrap_or("Unknown Artist");
        let album_artist_ids = Self::split_and_upsert_artists(conn, album_artist_str)?;
        let album_artist_id = *album_artist_ids
            .first()
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
        let exact_match: Option<i64> = conn
            .query_row(
                "SELECT id FROM tracks WHERE normalized_title = ?1 AND album_id IS ?2 LIMIT 1",
                params![normalized_track_title, album_id],
                |row| row.get(0),
            )
            .optional()?;

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
                params![
                    normalized_track_title,
                    main_artist_normalized,
                    metadata.duration_ms.unwrap_or(0)
                ],
                |row| row.get(0),
            )
            .optional()?
        } else {
            None
        };

        let (track_id, track_is_new): (i64, bool) = if let Some(id) = exact_match.or(fuzzy_match) {
            (id, false)
        } else {
            conn.execute(
                "INSERT INTO tracks (title, normalized_title, sort_title, album_id, year) VALUES (?1, ?2, ?2, ?3, ?4)",
                params![track_title, normalized_track_title, album_id, metadata.year],
            )?;
            (conn.last_insert_rowid(), true)
        };

        // 年份补扫：已有 track 只在无值时回填，不覆盖用户可能手工修正过的值。
        // 旧曲库文件未被修改时扫描器会跳过重解析，需要增量补扫路径（见 scanner）。
        if !track_is_new {
            if metadata.year.is_some() {
                conn.execute(
                    "UPDATE tracks SET year = ?1 WHERE id = ?2 AND year IS NULL",
                    params![metadata.year, track_id],
                )?;
            }
            // 专辑发行年份同样只补空
            if metadata.year.is_some() {
                conn.execute(
                    "UPDATE albums SET release_year = ?1 WHERE id = ?2 AND release_year IS NULL",
                    params![metadata.year, album_id],
                )?;
            }
        } else if metadata.year.is_some() {
            conn.execute(
                "UPDATE albums SET release_year = ?1 WHERE id = ?2 AND release_year IS NULL",
                params![metadata.year, album_id],
            )?;
        }

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
                source_id, track_id, relative_path, normalized_path, file_name, file_ext, file_size, modified_at, duration_ms, bitrate, sample_rate, bit_depth, channels, last_seen_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))
            ON CONFLICT(source_id, normalized_path) DO UPDATE SET
                track_id=excluded.track_id,
                file_size=excluded.file_size,
                modified_at=excluded.modified_at,
                duration_ms=excluded.duration_ms,
                bitrate=excluded.bitrate,
                sample_rate=excluded.sample_rate,
                bit_depth=excluded.bit_depth,
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
                metadata.bit_depth,
                metadata.channels
            ],
        )?;

        // 6.5 流派入库：标签有流派时整体替换该曲目的流派关联（扫描数据是权威来源，
        // 目前没有手工编辑流派的入口）；无标签时保留旧关联（补扫不清空）。
        Self::sync_track_genres(conn, track_id, metadata.genre.as_deref())?;

        // 回查出刚才被更新或插入的媒体文件 ID
        let media_file_id: i64 = conn.query_row(
            "SELECT id FROM media_files WHERE source_id = ?1 AND normalized_path = ?2",
            params![source_id, normalized_path],
            |row| row.get(0),
        )?;

        // 7. 将刚刚存储成功的最优物理文件作为此歌曲的首选音源
        let current_primary_file_id: Option<i64> = conn
            .query_row(
                "SELECT primary_file_id FROM tracks WHERE id = ?1",
                params![track_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

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

                let new_score: i32 = conn
                    .query_row(
                        "SELECT kind FROM sources WHERE id = ?1",
                        params![source_id],
                        |row| {
                            let kind: String = row.get(0)?;
                            Ok(crate::services::file_priority::file_priority_score(
                                &kind, &ext,
                            ))
                        },
                    )
                    .unwrap_or(0);

                new_score > curr_score
            }
        } else {
            true
        };

        if should_update_primary {
            conn.execute(
                "UPDATE tracks SET primary_file_id = ?1 WHERE id = ?2",
                params![media_file_id, track_id],
            )?;
        }

        // 8. 歌词入库：优先扫描 worker 预读的同目录同名 LRC 文件，没有则回退内嵌歌词
        //   （文件系统读取已在 worker 完成，写事务内不再有磁盘 I/O）
        let lrc_content = prepared
            .lrc_content
            .clone()
            .or_else(|| metadata.lyrics.clone());

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

    /// 流派标签分隔符（`&` 不算流派分隔符——"R&B" 是一个流派）
    const GENRE_SEPARATORS: &[char] = &[';', '；', '、', '，', ',', '/', '／', '|', '｜'];
    /// 单个曲目的流派数量上限（脏标签防御，避免爆炸式关联）
    const MAX_GENRES_PER_TRACK: usize = 5;

    /// 切分流派标签：支持 `;` `/` `,` `、` `|` 等分隔符，trim + 去重（大小写不敏感）。
    /// "R&B"、"Drum & Bass" 等含 `&` 的复合流派不会被拆开。
    fn split_genres(raw: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut buf = String::new();
        for ch in raw.chars() {
            if Self::GENRE_SEPARATORS.contains(&ch) {
                Self::flush_genre(&mut out, &mut seen, &mut buf);
            } else {
                buf.push(ch);
            }
        }
        Self::flush_genre(&mut out, &mut seen, &mut buf);
        out.truncate(Self::MAX_GENRES_PER_TRACK);
        out
    }

    fn flush_genre(
        out: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        buf: &mut String,
    ) {
        let name = buf.trim();
        if !name.is_empty() {
            let key = name.to_lowercase();
            if seen.insert(key) {
                out.push(name.to_string());
            }
        }
        buf.clear();
    }

    /// 把标签流派写入 genres / track_genres：有流派时整体替换该曲目的关联；
    /// 标签为空时不动旧数据（增量补扫不应清空已有值）。
    fn sync_track_genres(
        conn: &Connection,
        track_id: i64,
        raw_genre: Option<&str>,
    ) -> rusqlite::Result<()> {
        let Some(raw) = raw_genre else {
            return Ok(());
        };
        let genres = Self::split_genres(raw);
        if genres.is_empty() {
            return Ok(());
        }

        conn.execute(
            "DELETE FROM track_genres WHERE track_id = ?1",
            params![track_id],
        )?;
        for name in genres {
            let normalized = name.to_lowercase();
            conn.execute(
                "INSERT OR IGNORE INTO genres (name, normalized_name) VALUES (?1, ?2)",
                params![name, normalized],
            )?;
            let genre_id: i64 = conn.query_row(
                "SELECT id FROM genres WHERE normalized_name = ?1 LIMIT 1",
                params![normalized],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?1, ?2)",
                params![track_id, genre_id],
            )?;
        }
        Ok(())
    }

    /// 切分多位艺人字符串（支持 feat./&/;/、/| 等分隔符），逐个 upsert 并返回 ID 列表。
    /// 若解析结果为空（全是分隔符或空白），则回退使用 "Unknown Artist"。
    /// 分隔符清单一律走 services::metadata，与存量数据迁移（db 的 V5/V11）保持同源。
    fn split_and_upsert_artists(conn: &Connection, raw: &str) -> rusqlite::Result<Vec<i64>> {
        let mut ids = Vec::new();
        for part in crate::services::metadata::split_artist_names(raw) {
            ids.push(Self::upsert_artist(conn, &part)?);
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
        let Some(hash) = &artwork.hash else {
            return Ok(None);
        };

        // 同一张封面若已存在，直接复用其 ID。
        // thumbnail_blob 为 NULL（老数据 / 清理缓存后）时用首见文件带来的缩略图补写，
        // 确保后续扫描能逐步修复缺失的缩略图。
        if let Some(id) = conn
            .query_row(
                "SELECT id FROM artwork WHERE content_hash = ?1",
                params![hash],
                |row| row.get(0),
            )
            .optional()?
        {
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
        let cache_path = artwork.cache_path.clone().unwrap_or_else(|| {
            Self::artwork_cache_path(app_data_dir, artwork.mime.as_deref(), hash)
        });

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
        app_data_dir
            .join("artworks")
            .join(format!("{}.{}", hash, ext))
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

// ===================== 测试：丰富歌曲信息入库（year / genre / bit_depth） =====================

#[cfg(test)]
mod rich_metadata_tests {
    use super::*;
    use crate::db::test_util::TempDir;
    use crate::repositories::track_repo::TrackRepo;
    use crate::services::metadata::AudioMetadata;

    /// 建一份带本地来源的测试库，返回 (临时目录守卫, 连接, source_id, 来源根路径)
    fn db_with_source(label: &str) -> (TempDir, Connection, i64, std::path::PathBuf) {
        let dir = TempDir::new(label);
        crate::db::init_db(dir.db_path()).expect("构造测试库失败");
        let conn = Connection::open(dir.db_path()).expect("打开测试库失败");
        let root = dir.path().join("music");
        std::fs::create_dir_all(&root).expect("创建来源根目录失败");
        conn.execute(
            "INSERT INTO sources (name, kind, root_uri) VALUES ('本地音乐', 'local', ?1)",
            rusqlite::params![root.to_string_lossy().to_string()],
        )
        .expect("插入来源失败");
        let id = conn.last_insert_rowid();
        (dir, conn, id, root)
    }

    /// 构造一个可直接入库的 PreparedFile（不触碰文件系统：index_file 对路径仅做字符串处理）
    fn prepared(root: &std::path::Path, rel: &str, meta: AudioMetadata) -> PreparedFile {
        PreparedFile {
            path: root.join(rel),
            metadata: meta,
            mtime: 1700000000,
            size: 1024,
            artwork: PreparedArtwork::default(),
            lrc_content: None,
        }
    }

    fn base_meta(title: &str) -> AudioMetadata {
        AudioMetadata {
            title: Some(title.to_string()),
            artist: Some("测试艺人".to_string()),
            album: Some("测试专辑".to_string()),
            duration_ms: Some(180_000),
            bit_rate: Some(320_000),
            sample_rate: Some(44_100),
            channels: Some(2),
            ..Default::default()
        }
    }

    #[test]
    fn split_genres_parses_dedupes_and_caps() {
        assert_eq!(
            LibraryService::split_genres("Pop; Rock"),
            vec!["Pop".to_string(), "Rock".to_string()]
        );
        // R&B 是一个流派，& 不拆
        assert_eq!(
            LibraryService::split_genres("R&B / Soul"),
            vec!["R&B".to_string(), "Soul".to_string()]
        );
        // 大小写不敏感去重 + 首见拼写保留
        assert_eq!(
            LibraryService::split_genres("Pop; POP; pop"),
            vec!["Pop".to_string()]
        );
        // 脏标签防御：最多保留 5 个流派
        assert_eq!(LibraryService::split_genres("a;b;c;d;e;f;g").len(), 5);
        // 纯分隔符 = 无流派
        assert!(LibraryService::split_genres("; / ;").is_empty());
    }

    #[test]
    fn index_file_persists_rich_metadata_and_list_query_returns_it() {
        let (_dir, conn, source_id, root) = db_with_source("rich_index");

        let mut meta = base_meta("丰富的歌");
        meta.year = Some(2020);
        meta.genre = Some("Pop; Rock".to_string());
        meta.bit_depth = Some(24);

        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "01 - rich.flac", meta),
            &root,
        )
        .expect("入库失败");

        // tracks.year / albums.release_year / media_files.bit_depth 落库
        let year: i64 = conn
            .query_row(
                "SELECT year FROM tracks WHERE title = '丰富的歌'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(year, 2020);
        let album_year: i64 = conn
            .query_row(
                "SELECT release_year FROM albums WHERE normalized_title = '测试专辑'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(album_year, 2020);
        let bit_depth: i64 = conn
            .query_row("SELECT bit_depth FROM media_files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(bit_depth, 24);

        // 流派多对多：一首歌关联两个流派
        let genre_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM track_genres", [], |r| r.get(0))
            .unwrap();
        assert_eq!(genre_count, 2);

        // 列表查询（SELECT 模板 + map_track_row 列序耦合）能带回全部扩展字段
        let tracks = TrackRepo::get_tracks_paginated(&conn, 50, 0, None).unwrap();
        assert_eq!(tracks.len(), 1);
        let t = &tracks[0];
        assert_eq!(t.year, Some(2020));
        assert_eq!(t.genres.as_deref(), Some("Pop; Rock"));
        assert_eq!(t.bitrate, Some(320_000));
        assert_eq!(t.sample_rate, Some(44_100));
        assert_eq!(t.bit_depth, Some(24));
    }

    #[test]
    fn backfill_fills_missing_year_and_never_overwrites_existing() {
        let (_dir, conn, source_id, root) = db_with_source("backfill");

        // 第一次入库：标签无年份 → tracks.year 为 NULL
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "a.flac", base_meta("补扫之歌")),
            &root,
        )
        .unwrap();

        // 第二次入库（模拟补扫重解析到年份）：只补空，不覆盖
        let mut meta = base_meta("补扫之歌");
        meta.year = Some(2001);
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "a.flac", meta),
            &root,
        )
        .unwrap();
        let year: i64 = conn
            .query_row(
                "SELECT year FROM tracks WHERE title = '补扫之歌'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(year, 2001);

        // 第三次入库：文件改了标签说 1999 —— 已有有效值不被扫描覆盖
        let mut meta = base_meta("补扫之歌");
        meta.year = Some(1999);
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "a.flac", meta),
            &root,
        )
        .unwrap();
        let year: i64 = conn
            .query_row(
                "SELECT year FROM tracks WHERE title = '补扫之歌'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(year, 2001, "已有年份不得被重新扫描覆盖");
    }

    /// 流派聚合读取的一致性辅助：与列表查询使用同一 GROUP_CONCAT 口径
    fn genres_of(conn: &Connection, title: &str) -> String {
        conn.query_row(
            "SELECT (SELECT GROUP_CONCAT(g.name, '; ') FROM track_genres tg JOIN genres g ON g.id = tg.genre_id WHERE tg.track_id = t.id) FROM tracks t WHERE t.title = ?1",
            rusqlite::params![title],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn rescan_replaces_genres_but_keeps_them_when_tag_missing() {
        let (_dir, conn, source_id, root) = db_with_source("genres");

        let mut meta = base_meta("流派之歌");
        meta.genre = Some("Pop; Rock".to_string());
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "g.flac", meta),
            &root,
        )
        .unwrap();
        assert_eq!(genres_of(&conn, "流派之歌"), "Pop; Rock");

        // 重扫到不同流派：整体替换（扫描数据是权威来源）
        let mut meta = base_meta("流派之歌");
        meta.genre = Some("Jazz".to_string());
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "g.flac", meta),
            &root,
        )
        .unwrap();
        assert_eq!(genres_of(&conn, "流派之歌"), "Jazz");

        // 再扫时标签丢了流派：保留旧关联，不清空（增量补扫安全性）
        LibraryService::index_file(
            &conn,
            source_id,
            &root,
            &prepared(&root, "g.flac", base_meta("流派之歌")),
            &root,
        )
        .unwrap();
        assert_eq!(genres_of(&conn, "流派之歌"), "Jazz");
    }

    /// V13 迁移：app_meta 表 + 流派索引就位，且 tag_parse_version 默认视为「待补扫」
    #[test]
    fn v13_provides_backfill_infrastructure() {
        let (_dir, conn, source_id, _root) = db_with_source("v13");
        let other_source_id = source_id + 1;
        assert!(crate::services::scanner::needs_tag_backfill(
            &conn, source_id
        ));
        assert!(crate::services::scanner::needs_tag_backfill(
            &conn,
            other_source_id
        ));
        crate::services::scanner::mark_tag_backfill_done(&conn, source_id).unwrap();
        assert!(!crate::services::scanner::needs_tag_backfill(
            &conn, source_id
        ));
        assert!(crate::services::scanner::needs_tag_backfill(
            &conn,
            other_source_id
        ));
        crate::services::scanner::mark_tag_backfill_done(&conn, other_source_id).unwrap();
        assert!(!crate::services::scanner::needs_tag_backfill(
            &conn,
            other_source_id
        ));

        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN ('idx_track_genres_track_id', 'idx_track_genres_genre_id')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 2);
    }
}
