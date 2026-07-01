pub mod models;
pub mod db;
pub mod services;
pub mod repositories;
pub mod commands;
pub mod ipc_trace;
pub mod error;

use db::{init_db, DbState};
use services::playback::PlaybackManager;
use services::cache::AudioCache;
use crate::commands::playback::PlaybackState;
use tracing_subscriber;
use std::sync::Mutex;
use std::sync::{Condvar, Mutex as StdMutex};
use std::path::PathBuf;
use tauri::Manager;
use crate::db::DbPool;

/// 简易计数信号量:限制 `lumo://artwork` 协议的并发处理数。
///
/// 背景:Windows WebView2 下,自定义 scheme 的 HTTP 请求与 IPC invoke 共享
/// WebView2 的消息/网络线程池。前端一次滚动会触发 30+ 个封面请求,把线程池
/// 占满,导致 `library_get_albums` 的 invoke 排队 1-2s(后端 SQL 仅 0.4ms)。
///
/// 限流到 4 并发后,封面请求排队等待,给 invoke 留出线程余量,彻底消除卡顿。
/// 排队在独立线程上发生(Tauri 用线程池处理 URI scheme),不会阻塞 invoke。
struct CountingSemaphore {
    count: StdMutex<usize>,
    condvar: Condvar,
    max: usize,
}

impl CountingSemaphore {
    const fn new(max: usize) -> Self {
        Self {
            count: StdMutex::new(0),
            condvar: Condvar::new(),
            max,
        }
    }

    /// 获取一个许可。若当前并发已达上限,会阻塞当前线程直到有许可释放。
    /// 返回的 RAII guard 在 drop 时自动释放许可,确保即使提前 return 也不会泄漏。
    fn acquire(&self) -> SemaphoreGuard<'_> {
        let mut count = self.count.lock().unwrap();
        while *count >= self.max {
            count = self.condvar.wait(count).unwrap();
        }
        *count += 1;
        SemaphoreGuard { sem: self }
    }
}

/// 信号量许可 guard,drop 时自动 release。
struct SemaphoreGuard<'a> {
    sem: &'a CountingSemaphore,
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        let mut count = self.sem.count.lock().unwrap();
        *count -= 1;
        self.sem.condvar.notify_one();
    }
}

/// 全局封面请求限流器:最多同时处理 4 个 `lumo://artwork` 请求。
/// 4 这个值是经验值:既保证封面加载吞吐(4 路并行),又给 IPC invoke 留足通道。
static ARTWORK_SEMAPHORE: CountingSemaphore = CountingSemaphore::new(4);

/// 后台异步回填 artwork 缩略图。
///
/// 在应用启动后 spawn 的独立线程里执行,不阻塞 UI。
/// 遍历所有 `thumbnail_blob IS NULL` 的 artwork 记录,读取原图文件生成 200x200 缩略图。
/// 期间前端正常使用 `lumo://` 协议加载封面(有 semaphore 限流保护),
/// 回填完成后 emit `artwork-backfill-complete` 事件,前端收到后重新拉取专辑列表。
///
/// 优化:每 50 条一批开启独立事务提交,避免 600+ 条逐条事务的写放大开销。
/// 每批完成后暂停 50ms,避免吃满 CPU 导致前端卡顿。
fn backfill_artwork_thumbnails(app: tauri::AppHandle, pool: &DbPool) {
    use rusqlite::params;
    use tauri::Emitter;

    let rows: Vec<(i64, String)> = {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[回填] 获取数据库连接失败: {}", e);
                return;
            }
        };
        let mut stmt = match conn.prepare("SELECT id, cache_path FROM artwork WHERE thumbnail_blob IS NULL") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("[回填] 准备查询失败: {}", e);
                return;
            }
        };
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        });
        match rows {
            Ok(r) => r.filter_map(Result::ok).collect(),
            Err(e) => {
                tracing::error!("[回填] 查询失败: {}", e);
                return;
            }
        }
    };

    let total = rows.len();
    if total == 0 {
        tracing::info!("[回填] 无需回填，所有 artwork 已有缩略图");
        let _ = app.emit("artwork-backfill-complete", ());
        return;
    }
    tracing::info!("[回填] 发现 {} 条 artwork 记录待生成缩略图，后台异步执行中...", total);

    let batch_size = 50;
    let mut done = 0usize;
    let mut failed = 0usize;

    for chunk in rows.chunks(batch_size) {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[回填] 获取数据库连接失败: {}", e);
                break;
            }
        };

        let tx = match conn.unchecked_transaction() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("[回填] 开启事务失败: {}", e);
                break;
            }
        };

        for (id, cache_path) in chunk {
            let thumb = std::fs::read(cache_path)
                .ok()
                .and_then(|data| crate::services::library::LibraryService::generate_thumbnail(&data));

            if let Some(blob) = thumb {
                let _ = tx.execute(
                    "UPDATE artwork SET thumbnail_blob = ?1 WHERE id = ?2",
                    params![blob, id],
                );
                done += 1;
            } else {
                failed += 1;
                tracing::warn!("[回填] artwork id={} 无法生成缩略图（路径={}）", id, cache_path);
            }
        }

        let _ = tx.commit();

        std::thread::sleep(std::time::Duration::from_millis(50));

        if (done + failed) % 100 == 0 {
            tracing::info!("[回填] 进度：{}/{}（成功 {}，失败 {}）", done + failed, total, done, failed);
        }
    }

    tracing::info!("[回填] 完成：成功 {}，失败 {}，总计 {}", done, failed, total);
    let _ = app.emit("artwork-backfill-complete", ());
}

/// 把 Unix 秒数格式化为 HTTP-date（IMF-fixdate）格式，
/// 用于 `Last-Modified` / `Date` 响应头。
///
/// 格式样例：`Wed, 14 Jun 2026 07:28:00 GMT`
/// 使用已有的 chrono crate（Cargo.toml 已引入），替换手写 civil_from_days 算法。
fn format_http_date(unix_secs: u64) -> Option<String> {
    use chrono::DateTime;
    let dt = DateTime::from_timestamp(unix_secs as i64, 0)?;
    Some(dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
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

            // 后台异步回填 artwork 缩略图（V4 迁移只标记版本号，实际回填在这里执行）。
            // 674 张图片用 Triangle 滤镜约需 15-40s，放后台线程不阻塞应用启动。
            // 期间前端用 lumo:// 协议加载封面（有 semaphore 限流保护），
            // 回填完成后 emit artwork-backfill-complete 事件，前端收到后重新拉取专辑列表。
            {
                let pool_clone = pool.clone();
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    backfill_artwork_thumbnails(app_handle, &pool_clone);
                });
            }

            app.manage(DbState {
                db: pool,
            });

            let playback_manager = PlaybackManager::new().expect("Failed to init playback");
            app.manage(PlaybackState {
                manager: Mutex::new(playback_manager),
            });

            // 云端音频文件透明缓存：WebDAV 歌曲播放时后台下载完整文件到本地，
            // 下次播放命中缓存走本地路径（零网络 + 自动 gapless）
            let audio_cache = AudioCache::new(&app_dir);
            app.manage(services::cache::AudioCacheState {
                cache: Mutex::new(audio_cache),
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
                // 限流:最多 4 个封面请求同时处理,其余排队等待。
                // guard 在作用域结束(含所有 return)时自动释放,不会泄漏。
                let _guard = ARTWORK_SEMAPHORE.acquire();
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
            crate::commands::scanner::source_add_local,
            crate::commands::scanner::source_add_webdav,
            crate::commands::scanner::source_scan,
            crate::commands::scanner::source_list,
            crate::commands::scanner::source_remove,
            crate::commands::library::library_get_tracks,
            crate::commands::library::library_get_albums,
            crate::commands::library::library_get_album_by_id,
            crate::commands::library::library_get_album_count,
            crate::commands::library::library_get_artists,
            crate::commands::library::library_get_artist_by_id,
            crate::commands::library::library_get_album_tracks,
            crate::commands::library::library_get_artist_albums,
            crate::commands::library::library_get_artist_album_count,
            crate::commands::library::library_get_artist_tracks,
            crate::commands::library::library_get_artist_stats,
            crate::commands::playback::playback_play,
            crate::commands::playback::playback_enqueue_next,
            crate::commands::playback::playback_get_queue_len,
            crate::commands::playback::playback_pause,
            crate::commands::playback::playback_resume,
            crate::commands::playback::playback_stop,
            crate::commands::playback::playback_set_volume,
            crate::commands::playback::playback_get_pos,
            crate::commands::playback::playback_seek,
            crate::commands::playback::playback_is_finished,
            crate::commands::library::library_toggle_favorite,
            crate::commands::library::library_create_playlist,
            crate::commands::library::library_get_playlists,
            crate::commands::library::library_add_to_playlist,
            crate::commands::library::library_get_playlist_tracks,
            crate::commands::library::library_record_play,
            crate::commands::library::library_get_recently_played,
            crate::commands::library::library_get_favorite_tracks,
            crate::commands::library::library_get_favorite_albums,
            crate::commands::library::library_get_favorite_artists,
            crate::commands::library::library_toggle_favorite_album,
            crate::commands::library::library_toggle_favorite_artist,
            crate::commands::library::library_get_lyrics,
            crate::commands::library::library_get_track_file_info,
            crate::commands::library::library_delete_playlist,
            crate::commands::library::library_remove_playlist_item,
            crate::commands::library::library_save_play_queue,
            crate::commands::library::library_get_play_queue,
            crate::commands::library::library_get_cache_size,
            crate::commands::library::library_clear_cache,
            crate::commands::playback::playback_is_cached,
            crate::commands::playback::playback_clear_audio_cache,
            crate::commands::playback::playback_get_audio_cache_size,
            crate::commands::library::library_get_folder_contents,
            crate::commands::library::library_add_folder_to_playlist,
            crate::commands::library::library_get_folder_children,
            crate::commands::library::library_get_folder_tracks,
            crate::commands::library::library_get_counts,
            crate::commands::library::library_fetch_missing_album_cover,
            crate::commands::library::library_fetch_missing_artist_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
