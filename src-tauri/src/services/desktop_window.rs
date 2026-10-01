//! Apply desktop geometry while the main window is still hidden.
use super::desktop_preferences::WindowGeometry;
#[cfg(not(target_os = "android"))]
use super::desktop_preferences::{DesktopPreferences, FORM_MINI};

#[derive(Clone, Copy)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn fit_geometry(saved: Option<WindowGeometry>, area: WorkArea, mini: bool) -> WindowGeometry {
    let mut g = saved.unwrap_or(WindowGeometry {
        x: area.x + (area.width - if mini { 560 } else { 1200 }) / 2,
        y: area.y + (area.height - if mini { 96 } else { 720 }) / 2,
        width: if mini { 560 } else { 1200 },
        height: if mini { 96 } else { 720 },
        maximized: false,
    });
    g.width = g.width.max(if mini { 480 } else { 1024 }).min(area.width);
    g.height = if mini {
        96.min(area.height)
    } else {
        g.height.max(640).min(area.height)
    };
    g.x = g.x.clamp(area.x, area.x + area.width - g.width);
    g.y = g.y.clamp(area.y, area.y + area.height - g.height);
    g.maximized &= !mini;
    g
}

#[cfg(not(target_os = "android"))]
pub fn apply_startup(
    window: &tauri::WebviewWindow,
    prefs: &DesktopPreferences,
) -> tauri::Result<()> {
    let enabled = cfg!(debug_assertions) || option_env!("VITE_LUMO_DESKTOP_MODES") == Some("1");
    let mini = enabled && prefs.window_form == FORM_MINI;
    let saved = if mini {
        prefs.mini_geometry
    } else {
        prefs.full_geometry
    };
    let monitors = window.available_monitors()?;
    let monitor_area = |monitor: &tauri::Monitor| {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        WorkArea {
            x: (f64::from(area.position.x) / scale).round() as i32,
            y: (f64::from(area.position.y) / scale).round() as i32,
            width: (f64::from(area.size.width) / scale).round() as i32,
            height: (f64::from(area.size.height) / scale).round() as i32,
        }
    };
    let matched = saved.and_then(|g| {
        monitors.iter().find(|monitor| {
            let a = monitor_area(monitor);
            g.x >= a.x && g.x < a.x + a.width && g.y >= a.y && g.y < a.y + a.height
        })
    });
    let fallback = window.current_monitor()?;
    let area = matched
        .or(fallback.as_ref())
        .or(monitors.first())
        .map(monitor_area)
        .unwrap_or(WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        });
    let g = fit_geometry(saved, area, mini);
    window.set_size_constraints(tauri::WindowSizeConstraints {
        min_width: Some(
            tauri::LogicalUnit(f64::from(if mini { 480 } else { 1024 }.min(area.width))).into(),
        ),
        min_height: Some(
            tauri::LogicalUnit(f64::from(if mini { 96 } else { 640 }.min(area.height))).into(),
        ),
        max_height: if mini {
            Some(tauri::LogicalUnit(f64::from(96.min(area.height))).into())
        } else {
            None
        },
        ..Default::default()
    })?;
    window.set_size(tauri::LogicalSize::new(g.width, g.height))?;
    window.set_position(tauri::LogicalPosition::new(g.x, g.y))?;
    window.set_always_on_top(mini && prefs.mini_always_on_top)?;
    if g.maximized {
        window.maximize()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_monitor_and_small_work_area_keep_controls_visible() {
        let g = fit_geometry(
            Some(WindowGeometry {
                x: -4000,
                y: 8000,
                width: 100,
                height: 20,
                maximized: true,
            }),
            WorkArea {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            true,
        );
        assert_eq!(
            g,
            WindowGeometry {
                x: 0,
                y: 624,
                width: 480,
                height: 96,
                maximized: false
            }
        );
        let small = fit_geometry(
            None,
            WorkArea {
                x: -800,
                y: 0,
                width: 800,
                height: 600,
            },
            false,
        );
        assert_eq!(
            (small.x, small.y, small.width, small.height),
            (-800, 0, 800, 600)
        );
    }
}
