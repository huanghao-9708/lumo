use rusqlite::{Connection, Result};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::PathBuf;

/// 连接池类型别名，简化引用。
/// r2d2::Pool 内部是 Arc，Send + Sync + Clone，可安全地跨线程共享。
pub type DbPool = r2d2::Pool<SqliteConnectionManager>;

/// 数据库全局状态，用于 Tauri 应用状态管理。
/// 使用连接池替代单一 Mutex<Connection>，并发的 artwork 请求和 IPC 命令
/// 各自从池中获取独立连接，彻底消除锁竞争导致的 ~1500ms 延迟。
pub struct DbState {
    pub db: DbPool,
}

/// 初始化 SQLite 连接池：
/// 1. 每条新连接通过 with_init 设置 WAL 模式、外键约束等 PRAGMA
/// 2. 迁移脚本在首条连接上串行执行，保证 schema 升级完成后才开放服务
/// 3. 连接池最大 8 条连接：覆盖 6 个封面并发请求 + 多个 IPC 命令
pub fn init_db(db_path: PathBuf) -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::file(&db_path)
        .with_init(|conn| {
            // 全局 PRAGMA：每条新连接都要设置一次
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            // WAL 下提升并发读取的缓存大小（默认 2000 页，加大可减少磁盘 I/O）
            conn.pragma_update(None, "cache_size", "8000")?;
            // NORMAL：WAL 模式下 NORMAL 已足够安全，且比 FULL 快很多
            conn.pragma_update(None, "synchronous", "NORMAL")?;
            Ok(())
        });

    let pool = r2d2::Pool::builder()
        .max_size(8) // 8 条并发连接：artwork × 6 + IPC 命令 × 2
        .build(manager)?;

    // 首次连接：建表 + 版本化迁移（保证在连接池对外服务前完成）
    {
        let conn = pool.get()?;
        // 第一阶段：基础建表（仅用 IF NOT EXISTS，字段定义以这一版为准）
        conn.execute_batch(BASE_SCHEMA_SQL)?;
        // 第二阶段：版本化迁移
        apply_migrations(&conn)?;
    }

    Ok(pool)
}

/// 基础建表脚本：定义全部表的初始结构。
/// 字段后续若有变更，必须通过 `apply_migrations` 中的 ALTER/补丁语句完成，
/// 而不是修改这里的 CREATE TABLE（否则老用户的库不会更新）。
const BASE_SCHEMA_SQL: &str = "
-- 记录数据库的升级版本历史
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 来源表：记录用户添加的本地目录或 WebDAV 远程端点
CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('local', 'webdav')), -- 限定类型
    root_uri TEXT NOT NULL, -- 根路径
    config_json TEXT NOT NULL DEFAULT '{}',
    credential_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_scan_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 来源能力探测表：记录对应远程端点的具体支持能力（如是否支持分段下载）
CREATE TABLE IF NOT EXISTS source_capabilities (
    source_id INTEGER PRIMARY KEY REFERENCES sources(id) ON DELETE CASCADE,
    supports_range INTEGER,
    supports_etag INTEGER,
    supports_last_modified INTEGER,
    supports_propfind INTEGER,
    max_parallel_requests INTEGER,
    checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    raw_json TEXT NOT NULL DEFAULT '{}'
);

-- 艺人表：保存从歌曲标签解析出的歌手/乐队信息
CREATE TABLE IF NOT EXISTS artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE, -- 用于去重的小写或归一化名字
    sort_name TEXT,
    kind TEXT DEFAULT 'unknown' CHECK (kind IN ('unknown', 'person', 'group', 'various')),
    mbid TEXT, -- MusicBrainz ID
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 专辑表：保存解析出的专辑信息
CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    sort_title TEXT,
    album_artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL, -- 专辑艺人
    album_type TEXT DEFAULT 'unknown' CHECK (album_type IN ('unknown', 'album', 'single', 'ep', 'compilation')),
    release_date TEXT,
    release_year INTEGER,
    total_discs INTEGER,
    cover_artwork_id INTEGER, -- 封面图片关联
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 歌曲实体表：这是用户视角的“一首歌”，独立于具体文件
CREATE TABLE IF NOT EXISTS tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    sort_title TEXT,
    album_id INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    disc_no INTEGER,
    track_no INTEGER,
    year INTEGER,
    primary_file_id INTEGER, -- 指向最高质量或最快访问的媒体文件
    rating INTEGER CHECK (rating BETWEEN 0 AND 5),
    play_count INTEGER NOT NULL DEFAULT 0, -- 播放次数统计
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at TEXT,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 媒体文件表：具体存在于磁盘或网络上的物理文件
CREATE TABLE IF NOT EXISTS media_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
    relative_path TEXT NOT NULL,
    normalized_path TEXT NOT NULL, -- 组合 source_id 作为唯一标识，防重复
    file_name TEXT NOT NULL,
    file_ext TEXT,
    file_size INTEGER,
    modified_at TEXT,
    etag TEXT,
    content_hash TEXT,
    quick_fingerprint TEXT,
    duration_ms INTEGER, -- 音频时长
    bitrate INTEGER,
    sample_rate INTEGER,
    bit_depth INTEGER,
    channels INTEGER,
    availability TEXT NOT NULL DEFAULT 'available' -- 记录文件是否离线或丢失
        CHECK (availability IN ('available', 'missing', 'offline', 'error')),
    last_seen_at TEXT,
    last_scanned_at TEXT,
    scan_error TEXT,
    raw_tags_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (source_id, normalized_path)
);

-- 歌曲与艺人多对多关系表：解决一首歌有多个合作艺人的问题
CREATE TABLE IF NOT EXISTS track_artists (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'main'
        CHECK (role IN ('main', 'featured', 'composer', 'album_artist', 'remixer')),
    position INTEGER NOT NULL DEFAULT 0, -- 决定艺人显示顺序
    PRIMARY KEY (track_id, artist_id, role)
);

-- 流派字典表
CREATE TABLE IF NOT EXISTS genres (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE
);

-- 歌曲与流派关系表
CREATE TABLE IF NOT EXISTS track_genres (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

-- 用户自建歌单表
CREATE TABLE IF NOT EXISTS playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    cover_artwork_id INTEGER,
    kind TEXT NOT NULL DEFAULT 'manual' CHECK (kind IN ('manual', 'smart')), -- 预留智能歌单
    smart_rules_json TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 歌单条目表：记录被添加进歌单的歌曲，独立 ID 允许同一首歌添加多次
CREATE TABLE IF NOT EXISTS playlist_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position REAL NOT NULL, -- 浮点数用于高效的拖拽排序
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 喜欢的歌曲收藏表
CREATE TABLE IF NOT EXISTS favorite_tracks (
    track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    favorited_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 喜欢的专辑收藏表
CREATE TABLE IF NOT EXISTS favorite_albums (
    album_id INTEGER PRIMARY KEY REFERENCES albums(id) ON DELETE CASCADE,
    favorited_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 喜欢的艺人收藏表
CREATE TABLE IF NOT EXISTS favorite_artists (
    artist_id INTEGER PRIMARY KEY REFERENCES artists(id) ON DELETE CASCADE,
    favorited_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 播放历史流水表：每一次播放都会产生一条记录，用于高级分析
CREATE TABLE IF NOT EXISTS play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
    media_file_id INTEGER REFERENCES media_files(id) ON DELETE SET NULL,
    played_at TEXT NOT NULL DEFAULT (datetime('now')),
    play_duration_ms INTEGER,
    completed INTEGER NOT NULL DEFAULT 0,
    source_kind TEXT,
    error TEXT
);

-- 歌词表：缓存解析到的内嵌或外部歌词文件
CREATE TABLE IF NOT EXISTS lyrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    media_file_id INTEGER REFERENCES media_files(id) ON DELETE SET NULL,
    format TEXT NOT NULL CHECK (format IN ('lrc', 'plain')),
    synced INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 播放队列持久化预留表
CREATE TABLE IF NOT EXISTS play_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    media_file_id INTEGER REFERENCES media_files(id) ON DELETE SET NULL,
    position REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 封面图片缓存表：避免重复提取图片
CREATE TABLE IF NOT EXISTS artwork (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_file_id INTEGER REFERENCES media_files(id) ON DELETE SET NULL,
    cache_path TEXT NOT NULL, -- 存在本地的物理缓存路径
    mime_type TEXT,
    width INTEGER,
    height INTEGER,
    content_hash TEXT UNIQUE -- 防止存储重复图片
);
";

/// 读取当前已应用到的迁移版本（0 表示全新库）
fn get_current_version(conn: &Connection) -> Result<i64> {
    // schema_migrations 表已经在 BASE_SCHEMA_SQL 中创建
    let v: Option<i64> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row: &rusqlite::Row| row.get(0))
        .ok()
        .flatten();
    Ok(v.unwrap_or(0))
}

/// 标记某次迁移已应用
fn mark_migration_applied(conn: &Connection, version: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version) VALUES (?1)",
        rusqlite::params![version],
    )?;
    Ok(())
}

/// 版本化迁移：按顺序应用每个版本的补丁，每个版本只执行一次。
/// 每个迁移函数应当做到幂等（IF NOT EXISTS / 包裹在 try 中），以便重试安全。
fn apply_migrations(conn: &Connection) -> Result<()> {
    let mut current = get_current_version(conn)?;

    // ===== V1: 索引与去重唯一约束（首次为已有库补齐索引） =====
    if current < 1 {
        // 关键索引：覆盖常用查询路径，避免曲库变大后全表扫描
        conn.execute_batch(
            "
            -- 艺人/专辑去重所依赖的字段
            CREATE INDEX IF NOT EXISTS idx_artists_normalized_name ON artists(normalized_name);
            CREATE INDEX IF NOT EXISTS idx_albums_normalized_title ON albums(normalized_title);
            CREATE INDEX IF NOT EXISTS idx_albums_artist_title ON albums(album_artist_id, normalized_title);

            -- 歌曲常用过滤/排序
            CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id);
            CREATE INDEX IF NOT EXISTS idx_tracks_added_at ON tracks(added_at);
            CREATE INDEX IF NOT EXISTS idx_tracks_last_played_at ON tracks(last_played_at);
            CREATE INDEX IF NOT EXISTS idx_tracks_normalized_title_album ON tracks(normalized_title, album_id);

            -- 多对多关系表
            CREATE INDEX IF NOT EXISTS idx_track_artists_artist ON track_artists(artist_id);
            CREATE INDEX IF NOT EXISTS idx_track_artists_track ON track_artists(track_id);

            -- 媒体文件按路径反查（文件夹加入歌单、播放定位等）
            CREATE INDEX IF NOT EXISTS idx_media_files_path ON media_files(source_id, normalized_path);
            CREATE INDEX IF NOT EXISTS idx_media_files_track_id ON media_files(track_id);

            -- 歌单条目按歌单排序
            CREATE INDEX IF NOT EXISTS idx_playlist_items_playlist ON playlist_items(playlist_id, position);

            -- 播放队列按 position 读取
            CREATE INDEX IF NOT EXISTS idx_play_queue_position ON play_queue(position);
            ",
        )?;
        mark_migration_applied(conn, 1)?;
        current = 1;
        tracing::info!("数据库迁移：已升级至 V1（索引补齐）");
    }

    // ===== V2: albums/ artists 加冗余统计字段，并回填老库 =====
    // 背景：get_albums_paginated 用子查询算 track_count，1000+ 张专辑要 1000+ 次索引查找，
    // 分页接口耗时 30-50ms。把 track_count 冗余到 albums 表后变成 O(1) 直接读字段。
    // 维护点：① index_file 新建 track 时 +1；② source_remove 删孤儿 track 时同步减。
    if current < 2 {
        // albums.track_count
        conn.execute_batch("ALTER TABLE albums ADD COLUMN track_count INTEGER NOT NULL DEFAULT 0;")?;
        // artists.track_count / artists.album_count（艺人页 stats 查询同样受益）
        conn.execute_batch("ALTER TABLE artists ADD COLUMN track_count INTEGER NOT NULL DEFAULT 0;")?;
        conn.execute_batch("ALTER TABLE artists ADD COLUMN album_count INTEGER NOT NULL DEFAULT 0;")?;

        // 一次性回填：albums.track_count = 该专辑下的 track 数
        conn.execute_batch(
            "UPDATE albums
             SET track_count = (
                 SELECT COUNT(*) FROM tracks t WHERE t.album_id = albums.id
             );"
        )?;
        // artists.track_count = 该艺人作为 track_artists 关联的 track 数
        conn.execute_batch(
            "UPDATE artists
             SET track_count = (
                 SELECT COUNT(DISTINCT ta.track_id) FROM track_artists ta WHERE ta.artist_id = artists.id
             );"
        )?;
        // artists.album_count = 该艺人作为 album_artist 的专辑数
        conn.execute_batch(
            "UPDATE artists
             SET album_count = (
                 SELECT COUNT(*) FROM albums al WHERE al.album_artist_id = artists.id
             );"
        )?;

        // 覆盖索引：让分页查询（带排序）能完全走索引，不必回表
        // albums(normalized_title) 覆盖 ORDER BY，配合 rowid 主键查 LIMIT/OFFSET 很快
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_albums_normalized_title_covering
             ON albums(normalized_title, album_artist_id, cover_artwork_id, track_count);"
        )?;
        // artists 列表页排序索引
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_artists_name_covering
             ON artists(normalized_name, track_count);"
        )?;

        mark_migration_applied(conn, 2)?;
        current = 2;
        tracing::info!("数据库迁移：已升级至 V2（冗余统计字段 + 覆盖索引）");
    }

    let _ = current; // 当前版本号，未来可用于日志/诊断
    Ok(())
}
