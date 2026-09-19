use crate::commands::playback::{
    resolve_media_file, spawn_background_cache_download, PlaybackState,
};
use crate::db::DbState;
use crate::error::AppError;
use crate::services::cache::AudioCacheState;
use crate::services::queue::{PlayMode, PlaybackQueueStateDto, QueueItem, QueueState};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

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

/// `Sink::empty()` 在自然播完、主动停止和从未播放时都为 true。
/// 只有活动播放生命周期里的空队列才是观察器需要消费的「自然播完」。
fn playback_just_finished(
    is_active: bool,
    is_empty: bool,
    queue_len: usize,
    has_preloaded_next: bool,
) -> bool {
    is_active && (is_empty || (queue_len == 1 && has_preloaded_next))
}

/// 解析队列条目实际应使用的 media_file_id：跟随 tracks.primary_file_id 的当前值。
/// 用户切换版本 / 扫描重指主文件后，自动切歌与 gapless 预加载无需重建队列即自动跟随；
/// 主文件为空或查询失败时回退到入队时的快照。
fn current_media_file_id(conn: &rusqlite::Connection, track_id: i64, snapshot: i64) -> i64 {
    conn.query_row(
        "SELECT COALESCE(primary_file_id, ?2) FROM tracks WHERE id = ?1",
        rusqlite::params![track_id, snapshot],
        |row| row.get(0),
    )
    .unwrap_or(snapshot)
}

pub fn internal_play_item(
    app: &AppHandle,
    queue_state: &State<'_, QueueState>,
    playback_state: &State<'_, PlaybackState>,
    index: usize,
    force_local: bool,
) -> Result<(), AppError> {
    // 只读取要播的这一首，**不**先把队列位置挪过去：播放器没接下之前挪位，
    // 等于把失败的那一首标记成「已播过」，自动切歌就会跳过它（CR-002）。
    let item = {
        let q = queue_state
            .queue
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if index >= q.items.len() {
            return Ok(());
        }
        q.items[index].clone()
    };

    let db_state = app.state::<DbState>();
    let cache_state = app.state::<AudioCacheState>();

    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let effective_media_file_id = match db_state.db.get() {
        Ok(conn) => current_media_file_id(&conn, item.track_id, item.media_file_id),
        Err(_) => item.media_file_id,
    };

    let audio_cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(
        &db_state,
        &audio_cache,
        effective_media_file_id,
        &key,
        force_local,
    )?;
    drop(audio_cache);

    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let _duration = if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        let dur = manager.play_stream(buffered_reader)?;
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            let expected_size = webdav_info.expected_size;
            drop(manager);
            spawn_background_cache_download(
                &cache_state,
                effective_media_file_id,
                client,
                url,
                expected_size,
            );
        }
        dur
    } else if let Some(path) = path_buf {
        manager.play_file(&path)?
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    };

    // 播放器已接下这一首，此时才把队列位置写定（CR-002）。
    if let Ok(mut q) = queue_state.queue.lock() {
        q.commit_index(index);
    }

    let _ = app.emit(
        "playback-track-changed",
        TrackChangedEvent {
            index,
            track: item.clone(),
        },
    );

    if let Ok(conn) = db_state.db.get() {
        let _ = crate::repositories::track_repo::TrackRepo::record_play(
            &conn,
            item.track_id,
            item.duration_ms.unwrap_or(0) as i64,
            Some(effective_media_file_id),
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

/// 播放队列里的某一首，并顺带维护自动切歌的重试状态（CR-002）。
///
/// 三个调用方（设置队列、用户切歌、观察者自动推进）要做的善后完全一样，集中在这一处才不会漏：
/// 成功就把退避整份作废；失败则把这一首登记成「下次仍要重试的目标」——否则观察者会从
/// 它的下一首开始数，重试就变成了跳过。错误事件按「曲目 + 错误类别」去重：同一首反复失败
/// 只报一次，换一首或换了故障类型必须再报，前端不会因为一次网络抖动刷出一串 toast，
/// 也不会因为前一首的报错把后一首的报错吞掉。
fn play_item(
    app: &AppHandle,
    queue_state: &State<'_, QueueState>,
    playback_state: &State<'_, PlaybackState>,
    index: usize,
    force_local: bool,
) -> Result<(), AppError> {
    match internal_play_item(app, queue_state, playback_state, index, force_local) {
        Ok(()) => {
            if let Ok(mut q) = queue_state.queue.lock() {
                q.retry.clear();
            }
            Ok(())
        }
        Err(e) => {
            let (failures, notify) = match queue_state.queue.lock() {
                Ok(mut q) => {
                    let key = q
                        .items
                        .get(index)
                        .map(|item| {
                            format!(
                                "{}:{}:{}",
                                item.track_id,
                                item.media_file_id,
                                e.retry_class()
                            )
                        })
                        .unwrap_or_else(|| format!("index-{}:{}", index, e.retry_class()));
                    let notify = q.retry.record_failure(index, &key, Instant::now());
                    (q.retry.failures(), notify)
                }
                // 连队列锁都拿不到时宁可多报一次：把故障吞掉比刷屏更糟
                Err(_) => (0, true),
            };
            tracing::error!(
                "[队列] 播放失败（index={}，连续第 {} 次）: {}",
                index,
                failures,
                e
            );
            if notify {
                let _ = app.emit(
                    "playback-error",
                    serde_json::json!({
                        "index": index,
                        "message": e.to_string(),
                    }),
                );
            }
            let _ = crate::services::platform::update_foreground("", "", "", false, 0, 0);
            Err(e)
        }
    }
}

pub fn internal_enqueue_next(
    app: &AppHandle,
    queue_state: &State<'_, QueueState>,
    playback_state: &State<'_, PlaybackState>,
    index: usize,
) -> Result<(), AppError> {
    let q = queue_state
        .queue
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if index >= q.items.len() {
        return Ok(());
    }
    let item = q.items[index].clone();
    drop(q);

    let db_state = app.state::<DbState>();
    let cache_state = app.state::<AudioCacheState>();

    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let effective_media_file_id = match db_state.db.get() {
        Ok(conn) => current_media_file_id(&conn, item.track_id, item.media_file_id),
        Err(_) => item.media_file_id,
    };
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(
        &db_state,
        &audio_cache,
        effective_media_file_id,
        &key,
        false,
    )?;
    drop(audio_cache);

    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.enqueue_next_stream(buffered_reader)?;
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            let expected_size = webdav_info.expected_size;
            drop(manager);
            spawn_background_cache_download(
                &cache_state,
                effective_media_file_id,
                client,
                url,
                expected_size,
            );
        }
    } else if let Some(path) = path_buf {
        manager
            .enqueue_next_file(&path)
            .map_err(|e| AppError::Internal(e.to_string()))?;
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
        let mut q = queue_state
            .queue
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        q.set_queue(items, index, mode);
    }

    if should_play {
        // 失败善后（登记重试目标、去重上报、前台通知）都在 play_item 里，这里只把错误回传给前端
        play_item(&app, &queue_state, &playback_state, index, false)?;
    }
    Ok(())
}

#[tauri::command]
pub fn playback_queue_state(
    queue_state: State<'_, QueueState>,
    playback_state: State<'_, PlaybackState>,
) -> Result<PlaybackQueueStateDto, AppError> {
    let q = queue_state
        .queue
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
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
        let mut q = queue_state
            .queue
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        q.advance(direction)
    };

    if let Some(idx) = target_index {
        play_item(&app, &queue_state, &playback_state, idx, false)?;
    }
    Ok(())
}

#[tauri::command]
pub fn playback_set_mode(
    queue_state: State<'_, QueueState>,
    mode: PlayMode,
) -> Result<(), AppError> {
    let mut q = queue_state
        .queue
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    q.set_mode(mode);
    Ok(())
}

pub fn queue_watcher_loop(app: AppHandle) {
    let mut next_enqueued_index: Option<usize> = None;
    let mut last_progress_ms = 0u64;
    let mut last_save_time = std::time::Instant::now();
    // 自动切歌失败的退避状态挂在 `PlaybackQueue::retry` 上而不是这里的局部变量：
    // 用户手动切歌、换队列、停止播放都在命令侧发生，它们必须能就地作废旧的退避。
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));

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

        let (position_ms, is_active, is_empty, queue_len) = {
            if let Ok(manager) = playback_state.manager.lock() {
                (
                    manager.get_pos(),
                    manager.is_active(),
                    manager.is_finished(),
                    manager.get_queue_len(),
                )
            } else {
                continue;
            }
        };

        // 每秒 emit 一次播放进度给前端
        if position_ms / 1000 != last_progress_ms / 1000 {
            last_progress_ms = position_ms;
            let _ = app.emit(
                "playback-progress",
                ProgressEvent {
                    position: position_ms,
                },
            );
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
        let remaining = current_item_duration.saturating_sub(position_ms);

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
        let just_finished = playback_just_finished(
            is_active,
            is_empty,
            queue_len,
            next_enqueued_index.is_some(),
        );
        if !just_finished {
            // 已经正常播起来了：解除退避，避免上一次故障拖慢之后的正常切歌
            q.retry.clear();
        }

        if just_finished {
            if let Some(next_idx) = next_enqueued_index.take() {
                // Gapless 无缝切歌成功触发
                q.index = next_idx;
                q.retry.clear();
                let track = q.items[next_idx].clone();
                drop(q);
                let _ = app.emit(
                    "playback-track-changed",
                    TrackChangedEvent {
                        index: next_idx,
                        track: track.clone(),
                    },
                );
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
                // 先判退避窗口、再取目标：窗口内的空转不能被算成又一次失败（CR-002）
                if q.retry.waiting(Instant::now()) {
                    // 退避窗口内：不打网络、不发错误事件，等下一跳再看
                    continue;
                }
                if let Some(target) = q.retry_or_next_index() {
                    if q.retry.pending().is_some() {
                        tracing::info!("[队列推进] 退避结束，重试 index={}", target);
                    }
                    drop(q);
                    // 失败登记（锁定这一首、排退避、按「曲目 + 错误类别」去重上报）都在 play_item 里
                    let _ = play_item(&app, &queue_state, &playback_state, target, false);
                } else {
                    // Normal 模式播到队尾，停止
                    if let Ok(manager) = playback_state.manager.lock() {
                        manager.stop();
                    }
                    let _ = crate::services::platform::stop_foreground();
                    let _ = app.emit(
                        "playback-status-changed",
                        serde_json::json!({ "is_playing": false }),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::playback_just_finished;

    #[test]
    fn idle_empty_sink_is_not_treated_as_finished() {
        assert!(!playback_just_finished(false, true, 0, false));
    }

    #[test]
    fn active_empty_sink_is_treated_as_finished() {
        assert!(playback_just_finished(true, true, 0, false));
    }

    #[test]
    fn gapless_transition_is_treated_as_finished() {
        assert!(playback_just_finished(true, false, 1, true));
    }

    #[test]
    fn active_track_is_not_treated_as_finished_without_transition() {
        assert!(!playback_just_finished(true, false, 1, false));
    }
}
