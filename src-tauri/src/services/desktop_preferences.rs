//! 桌面偏好事实源（desktop-modes 01 §5）：`desktop_ui_preferences.json`。
//!
//! 职责与边界（DM-01）：
//! - 版本化：`schemaVersion` 不等于当前版本时整体回退默认（未知 schema 不猜测）。
//! - 原子写入：先写临时文件再 rename 覆盖，进程被杀不会留下半写配置。
//! - 字段级回退：未知模式字符串、非法几何逐字段回退，不因单字段损坏丢弃整份文件。
//! - 旧 `lumo_window_size`（localStorage）由前端在首次提交时迁移为完整几何，
//!   本模块只接收迁移结果，不读 WebView 存储。
//!
//! 设计偏差记录：计划要求「负坐标回退」，但多显示器环境中 x/y 为负是合法布局，
//! 这里改为「坐标有界（|值| ≤ 100000）即接受，越界视为垃圾回退」；宽高必须为正。

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DESKTOP_PREFS_SCHEMA_VERSION: i64 = 1;
pub const FILE_NAME: &str = "desktop_ui_preferences.json";

pub const EXPERIENCE_NORMAL: &str = "normal";
pub const EXPERIENCE_MINIMAL: &str = "minimal";
pub const FORM_FULL: &str = "full";
pub const FORM_MINI: &str = "mini";

/// 逻辑像素坐标与尺寸（desktop-modes 01 §5：所有 geometry 使用逻辑像素）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl WindowGeometry {
    fn is_plausible(&self) -> bool {
        const COORD_LIMIT: i32 = 100_000;
        self.width > 0
            && self.height > 0
            && self.width <= COORD_LIMIT
            && self.height <= COORD_LIMIT
            && self.x.unsigned_abs() <= COORD_LIMIT as u32
            && self.y.unsigned_abs() <= COORD_LIMIT as u32
    }
}

/// 两维偏好：体验模式（normal/minimal）× 窗口形态（full/mini），彼此独立。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DesktopPreferences {
    pub schema_version: i64,
    pub experience_mode: String,
    pub window_form: String,
    pub full_geometry: Option<WindowGeometry>,
    pub mini_geometry: Option<WindowGeometry>,
    pub mini_always_on_top: bool,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            schema_version: DESKTOP_PREFS_SCHEMA_VERSION,
            experience_mode: EXPERIENCE_NORMAL.to_string(),
            window_form: FORM_FULL.to_string(),
            full_geometry: None,
            mini_geometry: None,
            mini_always_on_top: false,
        }
    }
}

/// 字段级回退：非法值回落默认，保留其余合法字段
fn sanitize(prefs: &mut DesktopPreferences) {
    if prefs.experience_mode != EXPERIENCE_NORMAL && prefs.experience_mode != EXPERIENCE_MINIMAL {
        prefs.experience_mode = EXPERIENCE_NORMAL.to_string();
    }
    if prefs.window_form != FORM_FULL && prefs.window_form != FORM_MINI {
        prefs.window_form = FORM_FULL.to_string();
    }
    if prefs
        .full_geometry
        .as_ref()
        .is_some_and(|g| !g.is_plausible())
    {
        prefs.full_geometry = None;
    }
    if prefs
        .mini_geometry
        .as_ref()
        .is_some_and(|g| !g.is_plausible())
    {
        prefs.mini_geometry = None;
    }
}

/// 读取结果：`file_existed` 供前端区分「首次运行（需要迁移旧尺寸）」与「已有偏好」。
/// 文件缺失或损坏不视为错误——回退默认并照常返回。
pub fn load(app_dir: &Path) -> (DesktopPreferences, bool) {
    let file_path = app_dir.join(FILE_NAME);
    let content = match std::fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => return (DesktopPreferences::default(), false),
    };
    match serde_json::from_str::<DesktopPreferences>(&content) {
        Ok(mut prefs) => {
            // 未知 schema（更高版本写入的文件）不猜测字段语义，整体回退默认
            if prefs.schema_version != DESKTOP_PREFS_SCHEMA_VERSION {
                tracing::warn!(
                    "[desktop_prefs] 未知 schemaVersion {}（当前 {}），回退默认",
                    prefs.schema_version,
                    DESKTOP_PREFS_SCHEMA_VERSION
                );
                return (DesktopPreferences::default(), true);
            }
            sanitize(&mut prefs);
            (prefs, true)
        }
        Err(e) => {
            tracing::warn!("[desktop_prefs] 配置损坏（{e}），回退默认");
            (DesktopPreferences::default(), true)
        }
    }
}

/// 原子写入：临时文件 + rename 覆盖。写入前先做字段校验，拒绝保存非法状态。
pub fn save(app_dir: &Path, prefs: &DesktopPreferences) -> Result<(), String> {
    if prefs.schema_version != DESKTOP_PREFS_SCHEMA_VERSION {
        return Err(format!(
            "拒绝保存未知 schemaVersion {}（当前 {}）",
            prefs.schema_version, DESKTOP_PREFS_SCHEMA_VERSION
        ));
    }
    let mut checked = prefs.clone();
    sanitize(&mut checked);
    if checked.experience_mode != prefs.experience_mode
        || checked.window_form != prefs.window_form
        || checked.full_geometry != prefs.full_geometry
        || checked.mini_geometry != prefs.mini_geometry
    {
        return Err("拒绝保存非法偏好（模式或几何未通过校验）".to_string());
    }

    let json = serde_json::to_string_pretty(&checked).map_err(|e| e.to_string())?;
    let final_path = app_dir.join(FILE_NAME);
    let tmp_path = app_dir.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&tmp_path, json).map_err(|e| format!("写临时文件失败: {e}"))?;
    // std::fs::rename 在 Windows 上使用 MOVEFILE_REPLACE_EXISTING，可覆盖已存在文件
    std::fs::rename(&tmp_path, &final_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("覆盖配置失败: {e}")
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lumo_desktop_prefs_test_{}_{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mini_prefs() -> DesktopPreferences {
        DesktopPreferences {
            experience_mode: EXPERIENCE_MINIMAL.to_string(),
            window_form: FORM_MINI.to_string(),
            mini_geometry: Some(WindowGeometry {
                x: 8,
                y: 16,
                width: 560,
                height: 96,
                maximized: false,
            }),
            mini_always_on_top: true,
            ..Default::default()
        }
    }

    #[test]
    fn missing_file_returns_default_and_not_existed() {
        let dir = temp_dir("missing");
        let (prefs, existed) = load(&dir);
        assert!(!existed);
        assert_eq!(prefs, DesktopPreferences::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let prefs = mini_prefs();
        save(&dir, &prefs).unwrap();
        let (loaded, existed) = load(&dir);
        assert!(existed);
        assert_eq!(loaded, prefs);
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let dir = temp_dir("corrupt");
        std::fs::write(dir.join(FILE_NAME), "{ not json !!!").unwrap();
        let (prefs, existed) = load(&dir);
        assert!(existed);
        assert_eq!(prefs, DesktopPreferences::default());
    }

    #[test]
    fn unknown_schema_version_falls_back_to_default() {
        let dir = temp_dir("schema");
        let mut prefs = mini_prefs();
        prefs.schema_version = 999;
        save_raw(&dir, &serde_json::to_value(&prefs).unwrap());
        let (loaded, existed) = load(&dir);
        assert!(existed);
        assert_eq!(loaded, DesktopPreferences::default());
    }

    #[test]
    fn invalid_fields_fall_back_per_field() {
        let dir = temp_dir("fields");
        save_raw(
            &dir,
            &serde_json::json!({
                "schemaVersion": 1,
                "experienceMode": "turbo",
                "windowForm": "floating",
                "fullGeometry": { "x": -999_999, "y": 0, "width": -5, "height": 0, "maximized": false },
                "miniGeometry": { "x": 4, "y": 2, "width": 560, "height": 96, "maximized": false },
                "miniAlwaysOnTop": true
            }),
        );
        let (prefs, _) = load(&dir);
        assert_eq!(prefs.experience_mode, EXPERIENCE_NORMAL);
        assert_eq!(prefs.window_form, FORM_FULL);
        assert_eq!(prefs.full_geometry, None);
        assert_eq!(
            prefs.mini_geometry,
            Some(WindowGeometry {
                x: 4,
                y: 2,
                width: 560,
                height: 96,
                maximized: false
            })
        );
        assert!(prefs.mini_always_on_top);
    }

    #[test]
    fn save_rejects_invalid_mode_and_geometry() {
        let dir = temp_dir("reject");
        let mut prefs = mini_prefs();
        prefs.experience_mode = "turbo".to_string();
        assert!(save(&dir, &prefs).is_err());

        let mut prefs = mini_prefs();
        prefs.full_geometry = Some(WindowGeometry {
            width: 0,
            ..Default::default()
        });
        assert!(save(&dir, &prefs).is_err());
        assert!(!dir.join(FILE_NAME).exists(), "失败的保存不应留下最终文件");
    }

    #[test]
    fn negative_coordinates_within_bound_are_legal() {
        let dir = temp_dir("negcoord");
        let mut prefs = mini_prefs();
        prefs.full_geometry = Some(WindowGeometry {
            x: -1920,
            y: -64,
            width: 1200,
            height: 720,
            maximized: false,
        });
        save(&dir, &prefs).unwrap();
        let (loaded, _) = load(&dir);
        assert_eq!(loaded.full_geometry.unwrap().x, -1920);
    }

    #[test]
    fn extreme_negative_coordinates_fall_back_without_overflow() {
        let dir = temp_dir("mincoord");
        save_raw(
            &dir,
            &serde_json::json!({
                "schemaVersion": 1, "windowForm": "mini",
                "miniGeometry": { "x": i32::MIN, "y": i32::MIN, "width": 560, "height": 96 }
            }),
        );
        let (loaded, _) = load(&dir);
        assert_eq!(loaded.mini_geometry, None);
        assert_eq!(loaded.window_form, FORM_MINI);
    }

    /// 直接写入任意 JSON（绕过 save 的校验，用于构造损坏/未知版本样本）
    fn save_raw(dir: &Path, value: &serde_json::Value) {
        std::fs::write(dir.join(FILE_NAME), serde_json::to_string(value).unwrap()).unwrap();
    }
}
