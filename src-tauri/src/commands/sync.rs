use tauri::{Manager, State};
use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::models::{SyncConfigDTO, SyncResult, RemoteCheckResult};
use crate::services::sync::SyncService;
use crate::services::webdav::WebdavFile;
use std::path::PathBuf;

// ========================= 配置读写 =========================

#[tauri::command]
pub fn sync_get_config(db_state: State<'_, DbState>) -> Result<SyncConfigDTO, AppError> {
    let conn = db_state.db.get()?;
    let config = SyncService::get_config(&conn)?;
    Ok(config)
}

#[tauri::command]
pub fn sync_save_config(db_state: State<'_, DbState>, config: SyncConfigDTO) -> Result<(), AppError> {
    let conn = db_state.db.get()?;
    SyncService::save_config(&conn, &config)?;
    Ok(())
}

// ========================= 文件夹浏览器 =========================

/// 浏览 WebDAV 目录树（PROPFIND 过滤后仅返回目录）。
/// url / username / password 来自同步配置，前端在调用前已从 config 读取。
#[tauri::command]
pub fn sync_browse_webdav(url: String, username: Option<String>, password: Option<String>, path: String) -> Result<Vec<WebdavFile>, AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: None,
        last_sync_at: None,
        last_sync_direction: None,
    };
    let files = SyncService::browse(&config, &path)?;
    Ok(files)
}

/// 在 WebDAV 上新建文件夹（文件夹浏览器内使用）。
#[tauri::command]
pub fn sync_create_folder(url: String, username: Option<String>, password: Option<String>, path: String) -> Result<(), AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: None,
        last_sync_at: None,
        last_sync_direction: None,
    };
    SyncService::create_remote_folder(&config, &path)?;
    Ok(())
}

// ========================= 同步操作 =========================

/// 立即同步上传：VACUUM INTO → PUT 到远程路径。
#[tauri::command]
pub fn sync_upload_now(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<SyncResult, AppError> {
    let _trace = ipc_trace!("sync_upload_now");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let conn = db_state.db.get()?;
    let config = SyncService::get_config(&conn)?;
    if !config.enabled {
        return Err(AppError::Internal("同步未启用，请在设置中配置并启用".to_string()));
    }
    let result = SyncService::sync_upload(&conn, &app_dir, &config)?;
    Ok(result)
}


/// Restore a validated snapshot into the live pool without replacing Tauri-managed state.
#[tauri::command]
pub fn sync_restore_now(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<String, AppError> {
    let _trace = ipc_trace!("sync_restore_now");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config = {
        let conn = db_state.db.get()?;
        SyncService::get_config(&conn)?
    };
    if !config.enabled {
        return Err(AppError::Internal("同步未启用，请在设置中配置并启用".to_string()));
    }

    let temp_path = SyncService::sync_download_to_temp(&app_dir, &config)?;
    let meta = std::fs::metadata(&temp_path)
        .map_err(|e| AppError::Internal(format!("无法读取下载的文件: {}", e)))?;
    if meta.len() == 0 {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Internal("下载的文件为空，恢复终止".to_string()));
    }
    if let Err(e) = SyncService::validate_snapshot(&temp_path, &app_dir) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Internal(e));
    }

    // Online Backup API keeps the managed DbState valid and gives us a rollback copy.
    let backup_path = app_dir.join("lumo.sqlite.restore_bak");
    let _ = std::fs::remove_file(&backup_path);
    let restore_result = (|| -> Result<(), AppError> {
        let mut conn = db_state.db.get()?;
        conn.backup(rusqlite::DatabaseName::Main, &backup_path, None)
            .map_err(|e| AppError::Internal(format!("备份当前数据库失败: {}", e)))?;
        if let Err(e) = conn.restore(rusqlite::DatabaseName::Main, &temp_path, Some(|_| {})) {
            let _ = conn.restore(rusqlite::DatabaseName::Main, &backup_path, Some(|_| {}));
            return Err(AppError::Internal(format!("恢复数据库失败，已尝试回滚: {}", e)));
        }

        use chrono::Utc;
        let ts = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(e) = conn.execute(
            "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'download' WHERE id = 1",
            rusqlite::params![ts],
        ) {
            let _ = conn.restore(rusqlite::DatabaseName::Main, &backup_path, Some(|_| {}));
            return Err(AppError::Internal(format!("更新同步状态失败，已尝试回滚: {}", e)));
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&temp_path);
    let _ = std::fs::remove_file(&backup_path);
    restore_result?;
    Ok("数据库已通过校验并从云端恢复成功".to_string())
}

/// 检查云端是否有同步数据（首次开启同步时检测用）。
#[tauri::command]
pub fn sync_check_remote(url: String, username: Option<String>, password: Option<String>, path: String) -> Result<RemoteCheckResult, AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: Some(path),
        last_sync_at: None,
        last_sync_direction: None,
    };
    let result = SyncService::check_remote(&config)?;
    Ok(result)
}
