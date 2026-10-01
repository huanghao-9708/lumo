//! Result-wide selection must have the same matching and ordering as visible pages.
use rusqlite::params;
use tauri_app_lib::{db::init_db, repositories::track_repo::TrackRepo};

#[test]
fn ids_match_pages_for_titles_albums_and_combined_artists() {
    let path =
        std::env::temp_dir().join(format!("lumo-result-scope-{}.sqlite", std::process::id()));
    let pool = init_db(path.clone()).unwrap();
    let conn = pool.get().unwrap();
    conn.execute_batch(
        "INSERT INTO sources (id,name,kind,root_uri) VALUES (1,'test','local','X:/test');
        INSERT INTO artists (id,name,normalized_name) VALUES (1,'甲','甲'), (2,'乙','乙');
        INSERT INTO albums (id,title,normalized_title) VALUES (1,'共享专辑','共享专辑');",
    )
    .unwrap();
    for id in 1..=240 {
        conn.execute("INSERT INTO tracks (id,title,normalized_title,album_id,added_at) VALUES (?1,?2,?2,1,'2026-09-30')", params![id, format!("测试曲 {id}")]).unwrap();
        conn.execute("INSERT INTO media_files (id,track_id,source_id,relative_path,normalized_path,file_name,file_ext,file_size,modified_at,duration_ms) VALUES (?1,?1,1,?2,?2,?2,'flac',100,'2026-09-30',10000)", params![id, format!("{id}.flac")]).unwrap();
        conn.execute(
            "UPDATE tracks SET primary_file_id=?1 WHERE id=?1",
            params![id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO track_artists (track_id,artist_id,position) VALUES (?1,1,0),(?1,2,1)",
            params![id],
        )
        .unwrap();
    }
    // Orphan rows without playable media must be excluded by both APIs.
    conn.execute("INSERT INTO tracks (id,title,normalized_title,album_id) VALUES (999,'测试曲 orphan','orphan',1)", []).unwrap();
    for keyword in [
        None,
        Some("测试曲"),
        Some("共享专辑"),
        Some("甲, 乙"),
        Some("  甲  "),
        Some("无匹配"),
        Some(" "),
    ] {
        let query = keyword.map(str::to_string);
        let ids = TrackRepo::get_track_ids(&conn, query.clone()).unwrap();
        let mut paged_ids = Vec::new();
        for offset in (0..300).step_by(50) {
            paged_ids.extend(
                TrackRepo::get_tracks_paginated(&conn, 50, offset, query.clone())
                    .unwrap()
                    .into_iter()
                    .map(|t| t.id),
            );
        }
        assert_eq!(ids, paged_ids, "query={keyword:?}");
        if keyword != Some("无匹配") {
            assert_eq!(ids.len(), 240);
        }
        assert!(!ids.contains(&999));
    }
    drop(conn);
    drop(pool);
    let _ = std::fs::remove_file(path);
}
