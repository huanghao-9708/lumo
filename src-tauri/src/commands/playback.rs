use tauri::{State, Manager};
use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::services::playback::PlaybackManager;
use crate::services::cache::{AudioCache, AudioCacheState, is_downloading, mark_downloading, unmark_downloading};
use crate::services::webdav::WebdavClient;
use std::sync::Mutex;
use std::path::PathBuf;
use rusqlite::OptionalExtension;

pub struct PlaybackState {
    pub manager: Mutex<PlaybackManager>,
}

/// 解析媒体文件到可播放的源。
///
/// 缓存优先策略（迭代一新增）：
/// - 先查本地音频缓存，命中则返回 PathBuf（走 play_file / enqueue_next_file，零网络 + gapless）
/// - 未命中：
///   - local → 返回本地文件路径
///   - webdav → 返回 HttpRangeReader（流播），同时调用方会 spawn 后台线程下载缓存
///
/// 返回 (本地路径, WebDAV 流)。两者互斥：命中缓存或本地文件时 stream 为 None。
pub fn resolve_media_file(
    db_state: &State<'_, DbState>,
    audio_cache: &AudioCache,
    mut media_file_id: i64,
    key: &[u8; 32],
    force_local: bool,
) -> Result<(Option<PathBuf>, Option<crate::services::webdav::HttpRangeReader>, WebdavResolveInfo), AppError> {
    let conn = db_state.db.get()?;
    
    // 如果要求强行使用本地版本（断网降级），我们查找当前 track_id 下最好的 local 音源
    if force_local {
        let local_fallback_id: Option<i64> = conn.query_row(
            "SELECT mf.id FROM media_files mf 
             JOIN sources s ON s.id = mf.source_id 
             WHERE mf.track_id = (SELECT track_id FROM media_files WHERE id = ?1) 
               AND s.kind = 'local' 
               AND mf.availability = 'available' 
             ORDER BY mf.file_size DESC LIMIT 1",
            rusqlite::params![media_file_id],
            |row| row.get(0)
        ).optional()?;
        
        if let Some(id) = local_fallback_id {
            media_file_id = id;
            tracing::info!("Offline auto-degraded to local media_file_id={}", id);
        } else if !audio_cache.is_cached(media_file_id) {
            return Err(AppError::Internal("离线模式下没有可用的本地文件或音频缓存".to_string()));
        }
    }

    let (source_id, relative_path, root_uri, kind, cred, size): (i64, String, String, String, Option<String>, i64) = conn.query_row(
        "SELECT mf.source_id, mf.relative_path, s.root_uri, s.kind, s.credential_ref, mf.file_size
         FROM media_files mf JOIN sources s ON mf.source_id = s.id
         WHERE mf.id = ?1",
        rusqlite::params![media_file_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    )?;

    // ===== 缓存优先：WebDAV 文件已缓存则直接走本地路径 =====
    if kind == "webdav" {
        if let Some(cached) = audio_cache.get_cached_path(media_file_id) {
            tracing::info!("Audio cache hit for media_file_id={}", media_file_id);
            return Ok((Some(cached), None, WebdavResolveInfo::default()));
        }
    }

    if kind == "webdav" {
        // 兼容三种 credential_ref 格式：
        // 1) "username##base64_encrypted" ─ V6+ 加密格式
        // 2) "username:password" ─ V5 及之前明文
        // 3) "username" ─ 仅有用户名
        let (username, password): (Option<String>, Option<String>) = cred.as_deref()
            .and_then(|c| {
                if let Some((u, enc)) = c.split_once("##") {
                    crate::commands::scanner::decrypt_password(key, enc)
                        .map(|p| (u.to_string(), p))
                } else if let Some((u, p)) = c.split_once(':') {
                    Some((u.to_string(), p.to_string()))
                } else {
                    Some((c.to_string(), String::new()))
                }
            })
            .map(|(u, p)| (Some(u), Some(p)))
            .unwrap_or((None, None));
        let webdav = WebdavClient::new(root_uri.clone(), username, password);
        let base_str = if root_uri.ends_with('/') { root_uri.clone() } else { format!("{}/", root_uri) };
        let base = reqwest::Url::parse(&base_str).map_err(|e| AppError::Internal(e.to_string()))?;
        let relative_url_path = relative_path.replace('\\', "/");
        let file_url = base.join(&relative_url_path).map_err(|e| AppError::Internal(e.to_string()))?.to_string();

        // 能力门控：来源支持 Range → 流播；不支持（或不曾探测）→ 探测/整文件下载降级。
        // 探测结果落 source_capabilities 表，7 天内复用，不重复发请求。
        let supports_range = match load_range_support(&conn, source_id)? {
            Some(v) => v,
            None => probe_and_persist_range_support(&conn, &webdav, source_id, &file_url)?,
        };

        if supports_range {
            let http_reader = crate::services::webdav::HttpRangeReader::new(&webdav, file_url.clone(), size as u64);
            Ok((
                None,
                Some(http_reader),
                WebdavResolveInfo {
                    webdav_client: Some(webdav),
                    file_url: Some(file_url),
                },
            ))
        } else {
            // 服务器不支持分段读取：整文件下载进缓存后按本地文件播放。
            // 同步等待期间前端停留在 isBuffering，失败按分类文案报错。
            let path = download_full_to_cache(audio_cache, &webdav, media_file_id, &file_url)?;
            Ok((Some(path), None, WebdavResolveInfo::default()))
        }
    } else {
        Ok((Some(PathBuf::from(&root_uri).join(relative_path)), None, WebdavResolveInfo::default()))
    }
}

/// 读取来源的 Range 能力缓存记录；缺失、未探测过或超过 7 天视为 None。
fn load_range_support(conn: &rusqlite::Connection, source_id: i64) -> Result<Option<bool>, AppError> {
    let v: Option<i64> = conn.query_row(
        "SELECT supports_range FROM source_capabilities
         WHERE source_id = ?1 AND supports_range IS NOT NULL
           AND julianday('now') - julianday(checked_at) <= 7",
        rusqlite::params![source_id],
        |row| row.get(0),
    ).optional()?;
    Ok(v.map(|v| v != 0))
}

/// 用真实文件 URL 探测 Range 支持并落库。探测失败（认证/网络等）返回分类后的可读错误。
fn probe_and_persist_range_support(
    conn: &rusqlite::Connection,
    webdav: &WebdavClient,
    source_id: i64,
    file_url: &str,
) -> Result<bool, AppError> {
    let supports_range = webdav.probe_range_support(file_url).map_err(AppError::Internal)?;
    conn.execute(
        "INSERT INTO source_capabilities (source_id, supports_range, checked_at, raw_json)
         VALUES (?1, ?2, datetime('now'), '{}')
         ON CONFLICT(source_id) DO UPDATE SET supports_range = ?2, checked_at = datetime('now')",
        rusqlite::params![source_id, supports_range],
    )?;
    tracing::info!("WebDAV source_id={} supports_range={} (probed)", source_id, supports_range);
    Ok(supports_range)
}

/// 整文件同步下载进音频缓存（服务器不支持 Range 时的播放降级路径）。
/// 复用全局 DOWNLOADING 标记防止与后台缓存下载互相踩踏。
fn download_full_to_cache(
    audio_cache: &AudioCache,
    webdav: &WebdavClient,
    media_file_id: i64,
    file_url: &str,
) -> Result<PathBuf, AppError> {
    if is_downloading(media_file_id) {
        return Err(AppError::Internal("该歌曲正在缓存中，请稍后再试".to_string()));
    }
    if !mark_downloading(media_file_id) {
        return Err(AppError::Internal("该歌曲正在缓存中，请稍后再试".to_string()));
    }

    let result = (|| -> Result<PathBuf, AppError> {
        let cache_dir = audio_cache
            .cache_path(media_file_id)
            .parent()
            .ok_or_else(|| AppError::Internal("缓存目录无效".to_string()))?
            .to_path_buf();
        let tmp_path = cache_dir.join(format!("{}.tmp", media_file_id));
        let bytes = webdav
            .download_to_file(file_url, &tmp_path)
            .map_err(AppError::Internal)?;
        if bytes == 0 {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(AppError::Internal("缓存下载内容为空".to_string()));
        }
        let final_path = cache_dir.join(format!("{}", media_file_id));
        std::fs::rename(&tmp_path, &final_path)
            .map_err(|e| AppError::Internal(format!("缓存写入失败: {}", e)))?;
        tracing::info!("Full-file degradation download done media_file_id={} ({} bytes)", media_file_id, bytes);
        Ok(final_path)
    })();

    unmark_downloading(media_file_id);
    result
}

/// WebDAV 解析附加信息：用于流播失败后后台缓存下载。
/// 仅在 WebDAV 未命中缓存时填充。
#[derive(Default)]
pub struct WebdavResolveInfo {
    pub webdav_client: Option<WebdavClient>,
    pub file_url: Option<String>,
}

/// 在后台线程异步下载 WebDAV 文件到缓存（播放同时进行，不阻塞音频）。
/// 下载完成或失败都不影响当前播放，仅影响「下次播放这首歌」的缓存命中。
pub fn spawn_background_cache_download(
    audio_cache_state: &State<'_, AudioCacheState>,
    media_file_id: i64,
    webdav_client: WebdavClient,
    file_url: String,
) {
    let cache_guard = match audio_cache_state.cache.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    // 已在下载或已缓存，跳过
    if is_downloading(media_file_id) || cache_guard.is_cached(media_file_id) {
        return;
    }
    if !mark_downloading(media_file_id) {
        return;
    }

    // 克隆缓存目录路径后释放锁，不阻塞后台线程
    let cache_dir = cache_guard.cache_path(media_file_id)
        .parent()
        .map(|p| p.to_path_buf());
    drop(cache_guard);

    let Some(cache_dir) = cache_dir else {
        unmark_downloading(media_file_id);
        return;
    };

    std::thread::spawn(move || {
        let tmp_path = cache_dir.join(format!("{}.tmp", media_file_id));
        let result = webdav_client.download_to_file(&file_url, &tmp_path);
        match result {
            Ok(bytes) => {
                if bytes == 0 {
                    tracing::warn!("Audio cache download empty for media_file_id={}", media_file_id);
                    let _ = std::fs::remove_file(&tmp_path);
                } else {
                    let final_path = cache_dir.join(format!("{}", media_file_id));
                    match std::fs::rename(&tmp_path, &final_path) {
                        Ok(_) => {
                            tracing::info!("Audio cache stored media_file_id={} ({} bytes)", media_file_id, bytes);
                            if let Some(parent) = cache_dir.parent() {
                                let cache_obj = crate::services::cache::AudioCache::new(parent);
                                cache_obj.prune_to_max_bytes(2 * 1024 * 1024 * 1024);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Audio cache rename failed: {}", e);
                            let _ = std::fs::remove_file(&tmp_path);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Audio cache download failed for media_file_id={}: {}", media_file_id, e);
                let _ = std::fs::remove_file(&tmp_path);
            }
        }
        unmark_downloading(media_file_id);
    });
}

#[tauri::command]
pub fn playback_play(
    app: tauri::AppHandle,
    playback_state: State<'_, PlaybackState>,
    db_state: State<'_, DbState>,
    cache_state: State<'_, AudioCacheState>,
    media_file_id: i64,
    force_local: Option<bool>,
) -> Result<Option<u64>, AppError> {
    let _trace = ipc_trace!("playback_play");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(&db_state, &audio_cache, media_file_id, &key, force_local.unwrap_or(false))?;
    drop(audio_cache); // 释放缓存锁，不阻塞后续播放

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let duration = if let Some(reader) = webdav_reader {
        // WebDAV 流播
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        let dur = manager.play_stream(buffered_reader)?;

        // 后台异步下载缓存（不影响当前播放）
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            drop(manager); // 释放播放锁再 spawn
            spawn_background_cache_download(&cache_state, media_file_id, client, url);
        }
        dur
    } else if let Some(path) = path_buf {
        // 本地文件或缓存命中
        manager.play_file(&path)?
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    };

    Ok(duration)
}

#[tauri::command]
pub fn playback_enqueue_next(
    app: tauri::AppHandle,
    playback_state: State<'_, PlaybackState>,
    db_state: State<'_, DbState>,
    cache_state: State<'_, AudioCacheState>,
    media_file_id: i64,
    force_local: Option<bool>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_enqueue_next");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(&db_state, &audio_cache, media_file_id, &key, force_local.unwrap_or(false))?;
    drop(audio_cache);

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(reader) = webdav_reader {
        // WebDAV 流式 gapless：直接 append 到 sink（不 stop，无缝衔接）
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.enqueue_next_stream(buffered_reader)?;

        // 后台异步下载缓存
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            drop(manager);
            spawn_background_cache_download(&cache_state, media_file_id, client, url);
        }
    } else if let Some(path) = path_buf {
        // 本地文件或缓存命中 → 标准 gapless
        manager.enqueue_next_file(&path).map_err(|e| AppError::Internal(e.to_string()))?;
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    }
    Ok(())
}

#[tauri::command]
pub fn playback_get_queue_len(playback_state: State<'_, PlaybackState>) -> Result<usize, AppError> {
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.get_queue_len())
}


#[tauri::command]
pub fn playback_pause(
    playback_state: State<'_, PlaybackState>,
    queue_state: State<'_, crate::services::queue::QueueState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_pause");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.pause();
    let pos = manager.get_pos();
    if let Ok(q) = queue_state.queue.lock() {
        if let Some(item) = q.items.get(q.index) {
            let _ = crate::services::platform::update_foreground(
                &item.title,
                &item.artist,
                &item.album,
                false,
                pos,
                item.duration_ms.unwrap_or(0),
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn playback_resume(
    playback_state: State<'_, PlaybackState>,
    queue_state: State<'_, crate::services::queue::QueueState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_resume");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.resume();
    let pos = manager.get_pos();
    if let Ok(q) = queue_state.queue.lock() {
        if let Some(item) = q.items.get(q.index) {
            let _ = crate::services::platform::update_foreground(
                &item.title,
                &item.artist,
                &item.album,
                true,
                pos,
                item.duration_ms.unwrap_or(0),
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn playback_stop(playback_state: State<'_, PlaybackState>) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_stop");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.stop();
    let _ = crate::services::platform::stop_foreground();
    Ok(())
}

#[tauri::command]
pub fn playback_set_volume(playback_state: State<'_, PlaybackState>, volume: f32) -> Result<(), AppError> {
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.set_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn playback_get_pos(playback_state: State<'_, PlaybackState>) -> Result<u64, AppError> {
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.get_pos())
}

#[tauri::command]
pub fn playback_seek(playback_state: State<'_, PlaybackState>, position_ms: u64) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_seek");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.try_seek(position_ms).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn playback_is_finished(playback_state: State<'_, PlaybackState>) -> Result<bool, AppError> {
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.is_finished())
}

/// 查询某首曲目是否已缓存到本地（前端用于离线置灰判断）。
#[tauri::command]
pub fn playback_is_cached(cache_state: State<'_, AudioCacheState>, media_file_id: i64) -> Result<bool, AppError> {
    let cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(cache.is_cached(media_file_id))
}

/// 清空全部音频缓存，返回释放的字节数。
#[tauri::command]
pub fn playback_clear_audio_cache(cache_state: State<'_, AudioCacheState>) -> Result<u64, AppError> {
    let cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    cache.clear().map_err(|e| AppError::Internal(e.to_string()))
}

/// 获取音频缓存总大小（字节），用于设置页显示。
#[tauri::command]
pub fn playback_get_audio_cache_size(cache_state: State<'_, AudioCacheState>) -> Result<u64, AppError> {
    let cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(cache.size_bytes())
}
