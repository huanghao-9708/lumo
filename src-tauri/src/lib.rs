pub mod models;
pub mod db;
pub mod services;
pub mod commands;

use db::{init_db, DbState};
use services::playback::PlaybackManager;
use crate::commands::{
    PlaybackState, source_add_local, source_scan, source_list, source_remove, library_get_tracks, library_get_albums, library_get_artists,
    library_get_album_tracks, library_get_artist_albums, library_get_artist_tracks,
    playback_play, playback_pause, playback_resume, playback_stop,
    playback_set_volume, playback_get_pos, playback_seek,
    library_toggle_favorite, library_create_playlist, library_get_playlists,
    library_add_to_playlist, library_get_playlist_tracks, library_record_play,
    library_get_recently_played, library_get_favorite_tracks
};
use tracing_subscriber;
use std::sync::Mutex;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).unwrap();
            let db_path = app_dir.join("lumo.sqlite");
            
            let conn = init_db(db_path).expect("Failed to initialize database");
            app.manage(DbState {
                db: Mutex::new(conn),
            });

            let playback_manager = PlaybackManager::new().expect("Failed to init playback");
            app.manage(PlaybackState {
                manager: Mutex::new(playback_manager),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("lumo", |ctx, request| {
            let app = ctx.app_handle();
            let uri = request.uri().to_string();
            // 兼容 Windows WebView2 (`http://lumo.localhost/artwork/1`) 和 标准 (`lumo://artwork/1`)
            // 去除可能存在的查询参数 (e.g., ?v=123)
            let uri_without_query = uri.split('?').next().unwrap_or(&uri);
            let artwork_id = uri_without_query.trim_end_matches('/').split('/').last().unwrap_or("").parse::<i64>().unwrap_or(0);
            
            if artwork_id > 0 {
                use tauri::Manager;
                if let Some(db_state) = app.try_state::<crate::db::DbState>() {
                    if let Ok(conn) = db_state.db.lock() {
                        if let Ok((cache_path, mime)) = conn.query_row(
                            "SELECT cache_path, mime_type FROM artwork WHERE id = ?1",
                            rusqlite::params![artwork_id],
                            |row| {
                                let p: String = row.get(0)?;
                                let m: Option<String> = row.get(1)?;
                                Ok((p, m))
                            },
                        ) {
                            if let Ok(data) = std::fs::read(&cache_path) {
                                let mime_type = mime.unwrap_or_else(|| "image/jpeg".to_string());
                                return tauri::http::Response::builder()
                                    .header("Content-Type", mime_type)
                                    .header("Access-Control-Allow-Origin", "*")
                                    .body(data)
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            
            tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap()
        })
        .invoke_handler(tauri::generate_handler![
            source_add_local,
            source_scan,
            source_list,
            source_remove,
            library_get_tracks,
            library_get_albums,
            library_get_artists,
            library_get_album_tracks,
            library_get_artist_albums,
            library_get_artist_tracks,
            playback_play,
            playback_pause,
            playback_resume,
            playback_stop,
            playback_set_volume,
            playback_get_pos,
            playback_seek,
            library_toggle_favorite,
            library_create_playlist,
            library_get_playlists,
            library_add_to_playlist,
            library_get_playlist_tracks,
            library_record_play,
            library_get_recently_played,
            library_get_favorite_tracks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
