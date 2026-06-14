pub mod models;
pub mod db;
pub mod services;
pub mod commands;
pub mod ipc_trace;

use db::{init_db, DbState};
use services::playback::PlaybackManager;
use crate::commands::{
    PlaybackState, source_add_local, source_scan, source_list, source_remove, library_get_tracks, library_get_albums, library_get_artists,
    library_get_album_tracks, library_get_artist_albums, library_get_artist_tracks, library_get_artist_stats,
    playback_play, playback_pause, playback_resume, playback_stop,
    playback_set_volume, playback_get_pos, playback_seek, playback_is_finished,
    library_toggle_favorite, library_create_playlist, library_get_playlists,
    library_add_to_playlist, library_get_playlist_tracks, library_record_play,
    library_get_recently_played, library_get_favorite_tracks, library_get_lyrics,
    library_get_track_file_info, library_delete_playlist, library_remove_playlist_item,
    library_save_play_queue, library_get_play_queue, library_get_cache_size, library_clear_cache,
    library_get_folder_contents, library_add_folder_to_playlist
};
use tracing_subscriber;
use std::sync::Mutex;
use std::path::PathBuf;
use tauri::Manager;

/// 把 Unix 秒数格式化为 HTTP-date（IMF-fixdate）格式，
/// 用于 `Last-Modified` / `Date` 响应头。
///
/// 格式样例：`Wed, 14 Jun 2026 07:28:00 GMT`
/// 手写实现是为了避免引入 chrono 这类额外依赖（agent.md 约束）。
fn format_http_date(unix_secs: u64) -> Option<String> {
    // 我们需要"年月日 + 时分秒"，但 std 只提供了从 1970-01-01 起的天数算法。
    // 下面用经典的 civil_from_days 算法把"天数"换算为公历日期。
    // 参考：Howard Hinnant, http://howardhinnant.github.io/date_algorithms.html
    let days = (unix_secs / 86400) as i64;
    let secs_of_day = (unix_secs % 86400) as u64;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    // civil_from_days：days 是自 1970-01-01 起的天数
    let z = days + 719468; // 偏移到 0000-03-01 为起点
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    // 星期几：1970-01-01 是周四（对应 days=0 → dow=4）
    // dow = (days + 4) mod 7，结果 0=周日, 6=周六
    let dow = ((days % 7 + 4 + 7) % 7) as u8;
    const WEEKDAY: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTH: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let weekday = WEEKDAY.get(dow as usize)?;
    let month = MONTH.get((m - 1) as usize)?;
    Some(format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday, d, month, year, hour, minute, second
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).unwrap();
            let db_path = app_dir.join("lumo.sqlite");

            let pool = init_db(db_path).expect("Failed to initialize database");
            app.manage(DbState {
                db: pool,
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
            // ===== [诊断日志] 统计 artwork 请求频率 =====
            use std::sync::atomic::{AtomicU64, Ordering};
            static ARTWORK_REQ_COUNT: AtomicU64 = AtomicU64::new(0);
            static ARTWORK_LAST_LOG: AtomicU64 = AtomicU64::new(0);
            let n = ARTWORK_REQ_COUNT.fetch_add(1, Ordering::Relaxed);
            if n / 100 != ARTWORK_LAST_LOG.load(Ordering::Relaxed) / 100 {
                ARTWORK_LAST_LOG.store(n, Ordering::Relaxed);
                tracing::info!("[PERF] artwork_requests_total={} (every 100th logged)", n + 1);
            }

            let app = ctx.app_handle();
            let uri = request.uri().to_string();
            // 兼容 Windows WebView2 (`http://lumo.localhost/artwork/1`) 和 标准 (`lumo://artwork/1`)
            let uri_without_query = uri.split('?').next().unwrap_or(&uri);
            let artwork_id = uri_without_query.trim_end_matches('/').split('/').last().unwrap_or("").parse::<i64>().unwrap_or(0);

            if artwork_id > 0 {
                use tauri::Manager;
                // 解析 If-None-Match 头（锁外做，省得在锁内多一次字符串操作）
                let headers = request.headers();
                let req_etag = headers
                    .get("if-none-match")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                // ---- 锁内：只做 SQLite 查询（毫秒级），查完立即释放锁 ----
                // 关键：把 std::fs::read 移出锁范围，避免持锁读磁盘阻塞其他 IPC。
                let meta: Option<(String, Option<String>, Option<String>)> = {
                    let db_state = app.try_state::<crate::db::DbState>();
                    let conn = db_state.as_ref().and_then(|s| s.db.get().ok());
                    conn.as_ref().and_then(|conn| {
                        conn.query_row(
                            "SELECT cache_path, mime_type, content_hash FROM artwork WHERE id = ?1",
                            rusqlite::params![artwork_id],
                            |row| {
                                let p: String = row.get(0)?;
                                let m: Option<String> = row.get(1)?;
                                let h: Option<String> = row.get(2)?;
                                Ok((p, m, h))
                            },
                        )
                        .ok()
                    })
                };
                // ← 锁在这里已经释放

                let Some((cache_path, mime, content_hash)) = meta else {
                    return tauri::http::Response::builder()
                        .status(404)
                        .header("Cache-Control", "no-store")
                        .body(Vec::new())
                        .unwrap();
                };

                // ---- 锁外：ETag 判定 + 读磁盘 ----
                let etag_value = content_hash
                    .as_deref()
                    .filter(|h| !h.is_empty())
                    .map(|h| format!("W/\"{}\"", h));

                // 命中缓存：ETag 一致，直接 304
                if let (Some(req), Some(etag)) = (req_etag.as_deref(), etag_value.as_deref()) {
                    if req == etag || req == "*" {
                        return tauri::http::Response::builder()
                            .status(304)
                            .header("ETag", etag)
                            .header("Cache-Control", "public, max-age=86400")
                            .body(Vec::new())
                            .unwrap();
                    }
                }

                // 未命中：读磁盘返回完整 body（此时已不持锁，不阻塞其他 IPC）
                let file_meta = std::fs::metadata(&cache_path).ok();
                let data = std::fs::read(&cache_path).ok();
                let (Some(file_meta), Some(data)) = (file_meta, data) else {
                    return tauri::http::Response::builder()
                        .status(404)
                        .header("Cache-Control", "no-store")
                        .body(Vec::new())
                        .unwrap();
                };

                let mime_type = mime.unwrap_or_else(|| "image/jpeg".to_string());
                let mut resp = tauri::http::Response::builder()
                    .header("Content-Type", mime_type)
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Cache-Control", "public, max-age=31536000, immutable");

                if let Some(etag) = &etag_value {
                    resp = resp.header("ETag", etag);
                }
                if let Ok(mtime) = file_meta.modified() {
                    if let Ok(secs) = mtime.duration_since(std::time::UNIX_EPOCH) {
                        if let Some(date_str) = format_http_date(secs.as_secs()) {
                            resp = resp.header("Last-Modified", date_str);
                        }
                    }
                }

                return resp.body(data).unwrap();
            }

            tauri::http::Response::builder()
                .status(404)
                .header("Cache-Control", "no-store")
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
            library_get_artist_stats,
            playback_play,
            playback_pause,
            playback_resume,
            playback_stop,
            playback_set_volume,
            playback_get_pos,
            playback_seek,
            playback_is_finished,
            library_toggle_favorite,
            library_create_playlist,
            library_get_playlists,
            library_add_to_playlist,
            library_get_playlist_tracks,
            library_record_play,
            library_get_recently_played,
            library_get_favorite_tracks,
            library_get_lyrics,
            library_get_track_file_info,
            library_delete_playlist,
            library_remove_playlist_item,
            library_save_play_queue,
            library_get_play_queue,
            library_get_cache_size,
            library_clear_cache,
            library_get_folder_contents,
            library_add_folder_to_playlist
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
