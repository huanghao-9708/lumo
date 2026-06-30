use tauri::{State, Manager};
use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::services::playback::PlaybackManager;
use std::sync::Mutex;
use std::path::PathBuf;

pub struct PlaybackState {
    pub manager: Mutex<PlaybackManager>,
}


fn resolve_media_file(db_state: &State<'_, DbState>, media_file_id: i64, key: &[u8; 32]) -> Result<(Option<PathBuf>, Option<crate::services::webdav::HttpRangeReader>), AppError> {
    let conn = db_state.db.get()?;
    let (relative_path, root_uri, kind, cred, size): (String, String, String, Option<String>, i64) = conn.query_row(
        "SELECT mf.relative_path, s.root_uri, s.kind, s.credential_ref, mf.file_size
         FROM media_files mf JOIN sources s ON mf.source_id = s.id
         WHERE mf.id = ?1",
        rusqlite::params![media_file_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )?;

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
        let webdav = crate::services::webdav::WebdavClient::new(root_uri.clone(), username, password);
        let base_str = if root_uri.ends_with('/') { root_uri.clone() } else { format!("{}/", root_uri) };
        let base = reqwest::Url::parse(&base_str).map_err(|e| AppError::Internal(e.to_string()))?;
        let relative_url_path = relative_path.replace('\\', "/");
        let file_url = base.join(&relative_url_path).map_err(|e| AppError::Internal(e.to_string()))?.to_string();

        let http_reader = crate::services::webdav::HttpRangeReader::new(&webdav, file_url, size as u64);
        Ok((None, Some(http_reader)))
    } else {
        Ok((Some(PathBuf::from(&root_uri).join(relative_path)), None))
    }
}


#[tauri::command]
pub fn playback_play(app: tauri::AppHandle, playback_state: State<'_, PlaybackState>, db_state: State<'_, DbState>, media_file_id: i64) -> Result<Option<u64>, AppError> {
    let _trace = ipc_trace!("playback_play");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);
    let (path_buf, webdav_reader) = resolve_media_file(&db_state, media_file_id, &key)?;

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    let duration = if let Some(reader) = webdav_reader {
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.play_stream(buffered_reader)?
    } else if let Some(path) = path_buf {
        manager.play_file(&path)?
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    };

    Ok(duration)
}

#[tauri::command]
pub fn playback_enqueue_next(app: tauri::AppHandle, playback_state: State<'_, PlaybackState>, db_state: State<'_, DbState>, media_file_id: i64) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_enqueue_next");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);
    let (path_buf, webdav_reader) = resolve_media_file(&db_state, media_file_id, &key)?;

    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(_reader) = webdav_reader {
        // TODO: Enqueue stream is not implemented yet. Just ignoring webdav gapless for now.
        return Err(AppError::Internal("Gapless playback for WebDAV is not yet supported".to_string()));
    } else if let Some(path) = path_buf {
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
pub fn playback_pause(playback_state: State<'_, PlaybackState>) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_pause");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.pause();
    Ok(())
}

#[tauri::command]
pub fn playback_resume(playback_state: State<'_, PlaybackState>) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_resume");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.resume();
    Ok(())
}

#[tauri::command]
pub fn playback_stop(playback_state: State<'_, PlaybackState>) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_stop");
    let manager = playback_state.manager.lock().map_err(|e| AppError::Internal(e.to_string()))?;
    manager.stop();
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
