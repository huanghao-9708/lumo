use tauri::{State, Manager};
use crate::db::{DbState, init_db};
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

/// 从云端恢复：下载 → 校验 → 替换本地 DB → 热重载连接池。
#[tauri::command]
pub fn sync_restore_now(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<String, AppError> {
    let _trace = ipc_trace!("sync_restore_now");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    {
        let conn = db_state.db.get()?;
        let config = SyncService::get_config(&conn)?;
        if !config.enabled {
            return Err(AppError::Internal("同步未启用".to_string()));
        }

        // 1. 下载云端快照到临时文件
        let temp_path = SyncService::sync_download_to_temp(&app_dir, &config)?;

        // 2. 验证下载文件完整性（非空）
        let meta = std::fs::metadata(&temp_path)
            .map_err(|e| AppError::Internal(format!("无法读取下载的文件: {}", e)))?;
        if meta.len() == 0 {
            return Err(AppError::Internal("下载的文件为空，恢复终止".to_string()));
        }

        // 3. 备份旧 DB，替换为新文件（原子 rename）
        let db_path = app_dir.join("lumo.sqlite");
        let backup_path = app_dir.join("lumo.sqlite.restore_bak");

        // 删除旧备份（如果存在）
        let _ = std::fs::remove_file(&backup_path);
        // 重命名当前 DB 为备份
        std::fs::rename(&db_path, &backup_path)
            .map_err(|e| AppError::Internal(format!("备份当前数据库失败: {}", e)))?;
        // 将下载的新文件重命名为 DB 路径
        std::fs::rename(&temp_path, &db_path)
            .map_err(|e| AppError::Internal(format!("替换数据库文件失败: {}", e)))?;

        // 注意：连接池在当前作用域结束时释放，然后我们新建 pool
    }

    // 5. 热重载连接池：关闭旧池，创建新池，重新注册到 Tauri
    //    首先获取旧的 DbState（当前已用过的引用会被丢弃）
    let db_path = app_dir.join("lumo.sqlite");
    let new_pool = init_db(db_path)
        .map_err(|e| AppError::Internal(format!("重新打开数据库失败: {}", e)))?;

    // 更新同步时间（需要用新连接）
    {
        let conn = new_pool.get().map_err(|e| AppError::Pool(e.into()))?;
        use chrono::Utc;
        let ts = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'download' WHERE id = 1",
            rusqlite::params![ts],
        ).ok();
    }

    // 覆盖 Tauri 管理的 DbState（extensions().insert() 替换已有值）
    app.manage(DbState { db: new_pool });

    Ok("数据库已从云端恢复并热重载成功".to_string())
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
