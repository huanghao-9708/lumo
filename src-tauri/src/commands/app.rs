//! 应用信息与移动平台命令（MA1）。

use crate::services::platform;

#[tauri::command]
pub fn app_get_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// "granted" | "denied"
#[tauri::command]
pub fn platform_check_audio_permission() -> String {
    if platform::has_audio_permission() {
        "granted".into()
    } else {
        "denied".into()
    }
}

/// 发起运行时权限申请，结果经 `lumo-permission-result` 事件异步回传（payload: bool）。
#[tauri::command]
pub fn platform_request_audio_permission() -> Result<(), String> {
    platform::request_audio_permission()
}

#[tauri::command]
pub fn platform_open_app_settings() -> Result<(), String> {
    platform::open_app_settings()
}

#[tauri::command]
pub fn platform_storage_suggestions() -> Vec<String> {
    platform::storage_suggestions()
}

/// 关闭应用（返回栈到底时的退出路径）。
#[tauri::command]
pub fn platform_finish_app() -> Result<(), String> {
    platform::finish_app()
}
