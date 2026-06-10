use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct DbState {
    pub db: Mutex<Connection>,
}

pub fn init_db(db_path: PathBuf) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('local', 'webdav')),
            root_uri TEXT NOT NULL,
            config_json TEXT NOT NULL DEFAULT '{}',
            credential_ref TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_scan_at TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS artists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            sort_name TEXT,
            kind TEXT DEFAULT 'unknown' CHECK (kind IN ('unknown', 'person', 'group', 'various')),
            mbid TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS albums (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            normalized_title TEXT NOT NULL,
            sort_title TEXT,
            album_artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
            album_type TEXT DEFAULT 'unknown' CHECK (album_type IN ('unknown', 'album', 'single', 'ep', 'compilation')),
            release_date TEXT,
            release_year INTEGER,
            total_discs INTEGER,
            cover_artwork_id INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            normalized_title TEXT NOT NULL,
            sort_title TEXT,
            album_id INTEGER REFERENCES albums(id) ON DELETE SET NULL,
            disc_no INTEGER,
            track_no INTEGER,
            year INTEGER,
            primary_file_id INTEGER,
            rating INTEGER CHECK (rating BETWEEN 0 AND 5),
            play_count INTEGER NOT NULL DEFAULT 0,
            skip_count INTEGER NOT NULL DEFAULT 0,
            last_played_at TEXT,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS track_favorites (
            track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER REFERENCES playlists(id) ON DELETE CASCADE,
            track_id INTEGER REFERENCES tracks(id) ON DELETE CASCADE,
            position INTEGER NOT NULL,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (playlist_id, track_id)
        );

        CREATE TABLE IF NOT EXISTS media_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
            relative_path TEXT NOT NULL,
            normalized_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_ext TEXT,
            file_size INTEGER,
            modified_at TEXT,
            etag TEXT,
            content_hash TEXT,
            quick_fingerprint TEXT,
            duration_ms INTEGER,
            bitrate INTEGER,
            sample_rate INTEGER,
            bit_depth INTEGER,
            channels INTEGER,
            availability TEXT NOT NULL DEFAULT 'available'
                CHECK (availability IN ('available', 'missing', 'offline', 'error')),
            last_seen_at TEXT,
            last_scanned_at TEXT,
            scan_error TEXT,
            raw_tags_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE (source_id, normalized_path)
        );

        CREATE TABLE IF NOT EXISTS artwork (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cache_path TEXT NOT NULL,
            mime_type TEXT,
            content_hash TEXT UNIQUE
        );
        "
    )?;

    Ok(conn)
}
