use tauri::{State, AppHandle, Manager, Emitter};
use crate::error::AppError;
use crate::services::queue::{QueueState, QueueItem, PlayMode, PlaybackQueueStateDto};
use crate::commands::playback::{PlaybackState, resolve_media_file, spawn_background_cache_download};
use crate::db::DbState;
use crate::services::cache::AudioCacheState;
use std::time::Duration;
use std::path::PathBuf;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TrackChangedEvent {
    index: usize,
    track: QueueItem,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    position: u64,
}

pub fn internal_play_item(
    app: &AppHandle,
    queue_state: &State<'_, QueueState>,
    playback_state: &State<'_, PlaybackState>,
    index: usize,
    force_local: bool,
) -> Result<(), AppError> {
    let mut q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    if index >= q.items.len() {
        return Ok(());
    }
    q.index = index;
    let item = q.items[index].clone();
    drop(q);

    let db_state = app.state::<DbState>();
    let cache_state = app.state::<AudioCacheState>();
    
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(&db_state, &audio_cache, item.media_file_id, &key, force_local)?;
    drop(audio_cache);

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let _duration = if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        let dur = manager.play_stream(buffered_reader)?;
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            drop(manager);
            spawn_background_cache_download(&cache_state, item.media_file_id, client, url);
        }
        dur
    } else if let Some(path) = path_buf {
        manager.play_file(&path)?
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    };

    let _ = app.emit("playback-track-changed", TrackChangedEvent {
        index,
        track: item.clone(),
    });
    
    if let Ok(conn) = db_state.db.get() {
        let _ = crate::repositories::track_repo::TrackRepo::record_play(
            &conn,
            item.track_id,
            item.duration_ms.unwrap_or(0) as i64,
            Some(item.media_file_id),
        );
    }
    if let Ok(q) = queue_state.queue.lock() {
        crate::services::queue::save_state_to_disk(&app_dir, &q, 0);
    }
    
    // MA2 前台服务与媒体通知同步
    let _ = crate::services::platform::update_foreground(
        &item.title,
        &item.artist,
        &item.album,
        true,
        0,
        item.duration_ms.unwrap_or(0),
    );

    Ok(())
}

pub fn internal_enqueue_next(
    app: &AppHandle,
    queue_state: &State<'_, QueueState>,
    playback_state: &State<'_, PlaybackState>,
    index: usize,
) -> Result<(), AppError> {
    let q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    if index >= q.items.len() {
        return Ok(());
    }
    let item = q.items[index].clone();
    drop(q);

    let db_state = app.state::<DbState>();
    let cache_state = app.state::<AudioCacheState>();
    
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state.cache.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(&db_state, &audio_cache, item.media_file_id, &key, false)?;
    drop(audio_cache);

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.enqueue_next_stream(buffered_reader)?;
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            drop(manager);
            spawn_background_cache_download(&cache_state, item.media_file_id, client, url);
        }
    } else if let Some(path) = path_buf {
        manager.enqueue_next_file(&path).map_err(|e| AppError::Internal(e.to_string()))?;
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    }
    Ok(())
}

#[tauri::command]
pub fn playback_set_queue(
    app: AppHandle,
    queue_state: State<'_, QueueState>,
    playback_state: State<'_, PlaybackState>,
    items: Vec<QueueItem>,
    index: usize,
    mode: PlayMode,
) -> Result<(), AppError> {
    let should_play = !items.is_empty();
    {
        let mut q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
        q.set_queue(items, index, mode);
    }

    if should_play {
        if let Err(e) = internal_play_item(&app, &queue_state, &playback_state, index, false) {
            tracing::error!("[队列] 初始播放失败（index={}）: {}", index, e);
            let _ = app.emit("playback-error", serde_json::json!({
                "index": index,
                "message": e.to_string(),
            }));
            let _ = crate::services::platform::update_foreground("", "", "", false, 0, 0);
            return Err(e);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn playback_queue_state(
    queue_state: State<'_, QueueState>,
    playback_state: State<'_, PlaybackState>,
) -> Result<PlaybackQueueStateDto, AppError> {
    let q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let position_ms = if let Ok(manager) = playback_state.manager.lock() {
        manager.get_pos()
    } else {
        0
    };
    Ok(PlaybackQueueStateDto {
        items: q.items.clone(),
        index: q.index,
        mode: q.mode,
        position_ms,
    })
}

#[tauri::command]
pub fn playback_advance(
    app: AppHandle,
    queue_state: State<'_, QueueState>,
    playback_state: State<'_, PlaybackState>,
    direction: i32,
) -> Result<(), AppError> {
    let target_index = {
        let mut q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
        q.advance(direction)
    };

    if let Some(idx) = target_index {
        if let Err(e) = internal_play_item(&app, &queue_state, &playback_state, idx, false) {
            tracing::error!("[队列] 切歌失败（idx={}）: {}", idx, e);
            let _ = app.emit("playback-error", serde_json::json!({
                "index": idx,
                "message": e.to_string(),
            }));
            let _ = crate::services::platform::update_foreground("", "", "", false, 0, 0);
            return Err(e);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn playback_set_mode(
    queue_state: State<'_, QueueState>,
    mode: PlayMode,
) -> Result<(), AppError> {
    let mut q = queue_state.queue.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    q.set_mode(mode);
    Ok(())
}

pub fn queue_watcher_loop(app: AppHandle) {
    let mut next_enqueued_index: Option<usize> = None;
    let mut last_progress_ms = 0u64;
    let mut last_save_time = std::time::Instant::now();
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));

    loop {
        std::thread::sleep(Duration::from_millis(250));
        
        let playback_state = match app.try_state::<PlaybackState>() {
            Some(s) => s,
            None => continue,
        };
        let queue_state = match app.try_state::<QueueState>() {
            Some(s) => s,
            None => continue,
        };
        
        let (position_ms, is_empty, queue_len) = {
            if let Ok(manager) = playback_state.manager.lock() {
                (manager.get_pos(), manager.is_finished(), manager.get_queue_len())
            } else {
                continue;
            }
        };

        // 每秒 emit 一次播放进度给前端
        if position_ms / 1000 != last_progress_ms / 1000 {
            last_progress_ms = position_ms;
            let _ = app.emit("playback-progress", ProgressEvent { position: position_ms });
        }

        // 每 30 秒自动持久化一次播放状态 (A2-6)
        if last_save_time.elapsed() >= Duration::from_secs(30) {
            if let Ok(q) = queue_state.queue.lock() {
                if !q.items.is_empty() {
                    crate::services::queue::save_state_to_disk(&app_dir, &q, position_ms);
                }
            }
            last_save_time = std::time::Instant::now();
        }

        let mut q = match queue_state.queue.lock() {
            Ok(lock) => lock,
            Err(_) => continue,
        };

        if q.items.is_empty() {
            continue;
        }

        let current_item_duration = q.items[q.index].duration_ms.unwrap_or(0);
        let remaining = if current_item_duration > position_ms { current_item_duration - position_ms } else { 0 };

        // 1. Gapless 预加载逻辑：曲尾前 3 秒送入底层队列
        if queue_len == 1 && remaining < 3000 && remaining > 0 && next_enqueued_index.is_none() {
            if let Some(next_idx) = q.next_index() {
                drop(q);
                if internal_enqueue_next(&app, &queue_state, &playback_state, next_idx).is_ok() {
                    next_enqueued_index = Some(next_idx);
                }
                continue;
            }
        }

        // 2. 切歌事件判定（底层队列从 2->1 变为下一首开始播放，或当前曲目播完）
        let just_finished = is_empty || (queue_len == 1 && next_enqueued_index.is_some());

        if just_finished {
            if let Some(next_idx) = next_enqueued_index.take() {
                // Gapless 无缝切歌成功触发
                q.index = next_idx;
                let track = q.items[next_idx].clone();
                drop(q);
                let _ = app.emit("playback-track-changed", TrackChangedEvent {
                    index: next_idx,
                    track: track.clone(),
                });
                // 同步前台媒体通知（MOB-004）
                let _ = crate::services::platform::update_foreground(
                    &track.title,
                    &track.artist,
                    &track.album,
                    true,
                    0,
                    track.duration_ms.unwrap_or(0),
                );
                continue;
            } else {
                // 非 gapless 切换或者最后一曲
                if let Some(next_idx) = q.next_index() {
                    drop(q);
                    if let Err(e) = internal_play_item(&app, &queue_state, &playback_state, next_idx, false) {
                        tracing::error!("[队列推进] 自动切歌下一首失败（next_idx={}）: {}", next_idx, e);
                        let _ = app.emit("playback-error", serde_json::json!({
                            "index": next_idx,
                            "message": e.to_string(),
                        }));
                        let _ = crate::services::platform::update_foreground("", "", "", false, 0, 0);
                    }
                } else {
                    // Normal 模式播到队尾，停止
                    if let Ok(manager) = playback_state.manager.lock() {
                        manager.stop();
                    }
                    let _ = crate::services::platform::stop_foreground();
                    let _ = app.emit("playback-status-changed", serde_json::json!({ "is_playing": false }));
                }
            }
        }
    }
}
