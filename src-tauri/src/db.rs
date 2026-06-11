use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

/// 数据库全局状态，用于 Tauri 应用状态管理
pub struct DbState {
    /// 使用 Mutex 包装 SQLite 连接，确保在多线程环境下安全访问
    pub db: Mutex<Connection>,
}

/// 初始化 SQLite 数据库，并执行所有数据表的建表操作
/// 如果表已存在则忽略 (`IF NOT EXISTS`)
pub fn init_db(db_path: PathBuf) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        -- 开启外键约束，以保证数据删除时（如 DELETE CASCADE）自动清理关联数据
        PRAGMA foreign_keys = ON;

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
        "
    )?;

    Ok(conn)
}
