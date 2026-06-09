// 了解更多关于 Tauri 命令的信息，请访问 https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("你好，{}！您已成功从 Rust 接收到问候！", name)
}

#[derive(serde::Serialize)]
struct InitStatus {
    app_name: String,
    version: String,
    database_ok: bool,
    audio_device_ok: bool,
    audio_device_name: Option<String>,
}

// 获取项目 Phase 0 初始化及依赖库检查状态
#[tauri::command]
fn get_initialization_status() -> InitStatus {
    // 验证 SQLite 本地数据库能否正常工作
    let db_ok = rusqlite::Connection::open_in_memory().is_ok();
    
    // 验证音频设备和播放模块 (rodio) 是否就绪
    let (audio_ok, device_name) = match rodio::OutputStream::try_default() {
        Ok((_stream, _handle)) => {
            (true, Some("系统默认音频输出设备".to_string()))
        }
        Err(_) => (false, None),
    };

    InitStatus {
        app_name: "Lumo (轻音)".to_string(),
        version: "v1.0 (Phase 0 - 技术验证)".to_string(),
        database_ok: db_ok,
        audio_device_ok: audio_ok,
        audio_device_name: device_name,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_initialization_status])
        .run(tauri::generate_context!())
        .expect("运行 tauri 应用程序时出错");
}

