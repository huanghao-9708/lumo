//! 桌面偏好读写命令（DM-01）。读取永不失败：文件缺失/损坏都回退默认，
//! 让启动路径不因偏好问题中断；只有保存失败向上返回错误。

use crate::error::AppError;
use crate::ipc_trace;
use crate::services::desktop_preferences::{self, DesktopPreferences};
use tauri::{AppHandle, Manager};

/// 读取结果包装：`fileExisted=false` 表示首次运行，前端据此执行旧窗口尺寸迁移
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreferencesSnapshot {
    pub preferences: DesktopPreferences,
    pub file_existed: bool,
}

#[tauri::command]
pub fn desktop_get_preferences(app: AppHandle) -> Result<DesktopPreferencesSnapshot, AppError> {
    let _trace = ipc_trace!("desktop_get_preferences");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let (preferences, file_existed) = desktop_preferences::load(&app_dir);
    Ok(DesktopPreferencesSnapshot {
        preferences,
        file_existed,
    })
}

#[tauri::command]
pub fn desktop_update_preferences(
    app: AppHandle,
    preferences: DesktopPreferences,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("desktop_update_preferences");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    desktop_preferences::save(&app_dir, &preferences).map_err(AppError::Internal)
}
