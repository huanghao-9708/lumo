//! [MA0 Spike] 移动端技术验证专用命令。
//!
//! 仅供 MA0 真机/模拟器验证使用：音频输出链路（测试音）与 WebDAV TLS 连通性（PROPFIND）。
//! 前端入口只在 `import.meta.env.DEV` 下渲染；MA1 收尾时移除本模块。

#[cfg(debug_assertions)]
use super::PlaybackState;
#[cfg(debug_assertions)]
use tauri::State;

/// 播放正弦测试音（默认 440Hz / 3 秒）。真机听到声音 = 音频输出链路验证通过。
///
/// `PlaybackManager::play_tone` 仅 debug 构建编译，release 下的注册保留但返回错误
/// （前端调试入口只在 `import.meta.env.DEV` 渲染，正常不会触达此分支）。
#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_play_tone(
    playback_state: State<'_, PlaybackState>,
    freq: Option<f32>,
) -> Result<Option<u64>, String> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| format!("playback lock: {}", e))?;
    manager.play_tone(freq.unwrap_or(440.0), 3)
}

/// release 构建下的占位实现：测试音源随 MA0 Spike 仅 debug 编译。
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn debug_play_tone() -> Result<Option<u64>, String> {
    Err("debug_play_tone 仅在 debug 构建可用".into())
}

/// 对 WebDAV 服务器发一次 `PROPFIND /`（Depth 1），
/// 验证 rustls(ring) TLS 握手、认证与目录枚举，返回耗时与条目摘要。
#[tauri::command]
pub fn debug_webdav_probe(
    base_url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<serde_json::Value, String> {
    let start = std::time::Instant::now();
    let client = crate::services::webdav::WebdavClient::new(base_url, username, password);
    let files = client.propfind("/")?;
    Ok(serde_json::json!({
        "ok": true,
        "latency_ms": start.elapsed().as_millis() as u64,
        "entries": files.len(),
        "dirs": files.iter().filter(|f| f.is_dir).count(),
        "sample": files.iter().take(5).map(|f| f.path.clone()).collect::<Vec<_>>(),
    }))
}
