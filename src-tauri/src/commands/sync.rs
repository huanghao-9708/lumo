use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::models::{RemoteCheckResult, SyncConfigDTO, SyncResult};
use crate::services::sync::SyncService;
use crate::services::webdav::WebdavFile;
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

// ========================= 配置读写 =========================

#[tauri::command]
pub fn sync_get_config(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<SyncConfigDTO, AppError> {
    let conn = db_state.db.get()?;
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = SyncService::get_config(&conn, &machine_key)?;
    Ok(config)
}

#[tauri::command]
pub fn sync_save_config(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    config: SyncConfigDTO,
) -> Result<(), AppError> {
    let conn = db_state.db.get()?;
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    SyncService::save_config(&conn, &config, &machine_key)?;
    Ok(())
}

// ========================= 文件夹浏览器 =========================

/// 浏览 WebDAV 目录树（PROPFIND 过滤后仅返回目录）。
/// url / username / password 来自同步配置，前端在调用前已从 config 读取。
#[tauri::command]
pub fn sync_browse_webdav(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<Vec<WebdavFile>, AppError> {
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
pub fn sync_create_folder(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<(), AppError> {
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

/// 从云端恢复中途失败后，用恢复前副本回滚 live 库，并把结果翻译成分级的错误文案。
/// 回滚也失败时必须给出副本路径：那是用户唯一的历史数据，静默丢弃等于让他丢库。
fn rollback(
    conn: &mut rusqlite::Connection,
    backup_path: &Path,
    cause: rusqlite::Error,
) -> AppError {
    match conn.restore(
        rusqlite::DatabaseName::Main,
        backup_path,
        Some(|_| {}),
    ) {
        Ok(_) => AppError::Internal(format!(
            "恢复数据库失败，已回滚到恢复前的本地副本: {}",
            cause
        )),
        Err(e) => AppError::Internal(format!(
            "恢复数据库失败（{}），自动回滚也失败（{}）。请先停止使用备份恢复功能：恢复前的副本仍保留在 {}，可手动还原",
            cause,
            e,
            backup_path.display()
        )),
    }
}

/// 立即同步上传：VACUUM INTO → PUT 到远程路径。
#[tauri::command]
pub fn sync_upload_now(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<SyncResult, AppError> {
    let _trace = ipc_trace!("sync_upload_now");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let conn = db_state.db.get()?;
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = SyncService::get_config(&conn, &machine_key)?;
    if !config.enabled {
        return Err(AppError::Internal(
            "同步未启用，请在设置中配置并启用".to_string(),
        ));
    }
    let result = SyncService::sync_upload(&conn, &app_dir, &config)?;
    Ok(result)
}

/// Restore a validated snapshot into the live pool without replacing Tauri-managed state.
#[tauri::command]
pub fn sync_restore_now(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<String, AppError> {
    let _trace = ipc_trace!("sync_restore_now");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = {
        let conn = db_state.db.get()?;
        SyncService::get_config(&conn, &machine_key)?
    };
    if !config.enabled {
        return Err(AppError::Internal(
            "同步未启用，请在设置中配置并启用".to_string(),
        ));
    }

    let temp_path = SyncService::sync_download_to_temp(&app_dir, &config)?;
    // 空文件 / HTML 错误页 / 非 Lumo 库 / 版本越界 / integrity_check 不通过
    // 全部由 validate_snapshot 拒绝，此时本地库尚未被触碰。
    if let Err(e) = SyncService::validate_snapshot(&temp_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Internal(e));
    }

    // 在线 Backup API：既保持受管 DbState 有效，又给回滚留一份副本。
    let backup_path = app_dir.join("lumo.sqlite.restore_bak");
    let _ = std::fs::remove_file(&backup_path);
    let restore_result = (|| -> Result<(), AppError> {
        let mut conn = db_state.db.get()?;
        conn.backup(rusqlite::DatabaseName::Main, &backup_path, None)
            .map_err(|e| AppError::Internal(format!("备份当前数据库失败: {}", e)))?;

        // 回滚失败是这条链路上最危险的分支：live 库可能停在半恢复状态，
        // 而唯一能救回的副本就在 backup_path。绝不能 let _ = 把失败吞掉。
        if let Err(e) = conn.restore(rusqlite::DatabaseName::Main, &temp_path, Some(|_| {})) {
            return Err(rollback(&mut conn, &backup_path, e));
        }

        use chrono::Utc;
        let ts = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(e) = conn.execute(
            "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'download' WHERE id = 1",
            rusqlite::params![ts],
        ) {
            return Err(rollback(&mut conn, &backup_path, e));
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&temp_path);
    if restore_result.is_err() {
        // 失败时**保留**副本：它此刻是用户唯一的本地历史数据。
        tracing::error!("从云端恢复失败，回滚副本保留于 {}", backup_path.display());
    } else {
        let _ = std::fs::remove_file(&backup_path);
    }
    restore_result?;
    Ok("数据库已通过校验并从云端恢复成功".to_string())
}

/// 检查云端是否有同步数据（首次开启同步时检测用）。
#[tauri::command]
pub fn sync_check_remote(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<RemoteCheckResult, AppError> {
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
