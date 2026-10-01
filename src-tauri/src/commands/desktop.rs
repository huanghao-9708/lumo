//! 桌面偏好读写命令（DM-01）与视觉资源策略运行态（DM-03）。
//! 读取永不失败：文件缺失/损坏都回退默认，让启动路径不因偏好问题中断；
//! 只有保存失败向上返回错误。

use crate::error::AppError;
use crate::ipc_trace;
use crate::services::desktop_preferences::{self, DesktopPreferences};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// 桌面视觉资源策略运行态（DM-03）。
///
/// `visual_paused` = true 时，可选视觉任务（启动缩略图回填等）不再启动新批次；
/// 桌面端初始为 **true**（「配置确认前不启动」，计划 01 §1），前端读偏好后按
/// `visualAllowed` 下发实际值；Android 初始为 false，且前端不调用策略命令，
/// 行为与既往完全一致（后端策略只影响桌面）。
///
/// Clone 共享 Arc 原子状态：回填线程立即看到 managed 实例的暂停/恢复指令。
#[derive(Clone)]
pub struct ResourcePolicyState {
    visual_paused: Arc<AtomicBool>,
    artwork_requests: Arc<AtomicU64>,
}

impl ResourcePolicyState {
    pub fn new(visual_paused: bool) -> Self {
        Self {
            visual_paused: Arc::new(AtomicBool::new(visual_paused)),
            artwork_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn visual_paused(&self) -> bool {
        self.visual_paused.load(Ordering::Relaxed)
    }

    pub fn set_visual_paused(&self, paused: bool) {
        self.visual_paused.store(paused, Ordering::Relaxed);
    }

    pub fn count_artwork_request(&self) {
        self.artwork_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn artwork_requests_total(&self) -> u64 {
        self.artwork_requests.load(Ordering::Relaxed)
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopResourceStats {
    pub artwork_requests_total: u64,
}

/// 前端按 visualAllowed 下发视觉策略（DM-03）。advisory：失败只影响可选任务时序。
#[tauri::command]
pub fn desktop_set_visual_policy(
    policy: State<'_, ResourcePolicyState>,
    allowed: bool,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("desktop_set_visual_policy");
    policy.set_visual_paused(!allowed);
    Ok(())
}

/// R02 观测：artwork 协议请求累计计数（M2/M4 的「轻量态增量为 0」证据通道）
#[tauri::command]
pub fn desktop_get_resource_stats(
    policy: State<'_, ResourcePolicyState>,
) -> Result<DesktopResourceStats, AppError> {
    let _trace = ipc_trace!("desktop_get_resource_stats");
    Ok(DesktopResourceStats {
        artwork_requests_total: policy.artwork_requests.load(Ordering::Relaxed),
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_policy_clone_observes_pause_resume_and_shared_counters() {
        let policy = ResourcePolicyState::new(true);
        let worker = policy.clone();
        policy.set_visual_paused(false);
        assert!(!worker.visual_paused());
        policy.set_visual_paused(true);
        assert!(worker.visual_paused());
        worker.count_artwork_request();
        assert_eq!(policy.artwork_requests_total(), 1);
    }
}
