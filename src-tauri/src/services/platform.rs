//! MA1：Android 平台能力桥。
//!
//! Kotlin 侧薄辅助方法在 gen/android 的 MainActivity（LUMO-CUSTOM 段），
//! Rust 经 initLumoAudioContext 存入的 Activity GlobalRef 以 JNI 调用；
//! Kotlin → Rust 的事件（返回键、权限结果）经 JNI 导出函数回注并转为 Tauri 事件。
//!
//! 桌面构建下所有命令返回安全默认值（移动端 UI 由 isAndroid 门控，不会触达）。

/// 存放 JavaVM 与 Activity GlobalRef 的原始指针。
/// 由 `Java_com_hao_lumo_MainActivity_initLumoAudioContext` 在 Activity 创建时写入。
#[cfg(target_os = "android")]
mod android {
    use jni::objects::JObject;
    use jni::JNIEnv;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::OnceLock;
    use tauri::{AppHandle, Emitter};

    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
    static VM_RAW: AtomicIsize = AtomicIsize::new(0);
    static ACTIVITY_RAW: AtomicIsize = AtomicIsize::new(0);

    pub fn set_app_handle(app: AppHandle) {
        let _ = APP_HANDLE.set(app);
    }

    /// 由 JNI 导出函数在 Activity onCreate 时调用（指针来自 GlobalRef，进程内持续有效）。
    pub fn set_context(vm_raw: isize, activity_raw: isize) {
        VM_RAW.store(vm_raw, Ordering::SeqCst);
        ACTIVITY_RAW.store(activity_raw, Ordering::SeqCst);
    }

    fn with_activity<F>(f: F) -> Result<(), String>
    where
        F: FnOnce(&mut JNIEnv, &JObject) -> Result<(), String>,
    {
        let vm_raw = VM_RAW.load(Ordering::SeqCst);
        let act_raw = ACTIVITY_RAW.load(Ordering::SeqCst);
        if vm_raw == 0 || act_raw == 0 {
            return Err("android context not initialized".into());
        }
        let vm = unsafe { jni::JavaVM::from_raw(vm_raw as *mut _) }.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
        let activity = unsafe { JObject::from_raw(act_raw as *mut _) };
        f(&mut env, &activity)
    }

    fn call_bool_method(name: &str) -> Result<bool, String> {
        let mut out = false;
        with_activity(|env, activity| {
            out = env
                .call_method(activity, name, "()Z", &[])
                .and_then(|v| v.z())
                .map_err(|e| e.to_string())?;
            Ok(())
        })?;
        Ok(out)
    }

    fn call_void_method(name: &str) -> Result<(), String> {
        with_activity(|env, activity| {
            env.call_method(activity, name, "()V", &[])
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    }

    pub fn has_audio_permission() -> bool {
        call_bool_method("lumoHasAudioPermission").unwrap_or(false)
    }

    /// 发起运行时申请；结果经 `lumo-permission-result` 事件异步回传。
    pub fn request_audio_permission() -> Result<(), String> {
        call_void_method("lumoRequestAudioPermission")
    }

    pub fn open_app_settings() -> Result<(), String> {
        call_void_method("lumoOpenAppSettings")
    }

    pub fn finish_app() -> Result<(), String> {
        call_void_method("lumoFinishApp")
    }

    pub fn restart_app() -> Result<(), String> {
        call_void_method("lumoRestartApp")
    }

    pub fn update_foreground(
        title: &str,
        artist: &str,
        album: &str,
        is_playing: bool,
        position_ms: u64,
        duration_ms: u64,
    ) -> Result<(), String> {
        with_activity(|env, activity| {
            let j_title = env.new_string(title).map_err(|e| e.to_string())?;
            let j_artist = env.new_string(artist).map_err(|e| e.to_string())?;
            let j_album = env.new_string(album).map_err(|e| e.to_string())?;
            env.call_method(
                activity,
                "lumoUpdateForeground",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;ZJJ)V",
                &[
                    (&j_title).into(),
                    (&j_artist).into(),
                    (&j_album).into(),
                    is_playing.into(),
                    (position_ms as i64).into(),
                    (duration_ms as i64).into(),
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
    }

    pub fn stop_foreground() -> Result<(), String> {
        call_void_method("lumoStopForeground")
    }

    /// 候选音乐目录探测（纯文件系统，无需 Kotlin）。
    /// 注意：无权限时 is_dir 返回 false，自然返回空列表。
    pub fn storage_suggestions() -> Vec<String> {
        const BASE: &str = "/storage/emulated/0";
        const CANDIDATES: &[&str] = &[
            "/Music",
            "/Download",
            "/DCIM/Music",
            "/Android/data/com.netease.cloudmusic/files/Music",
            "/netease/cloudmusic/Music",
            "/kgmusic/download",
            "/qqmusic/song",
            "/kugou/download",
            "/mymusic",
        ];
        CANDIDATES
            .iter()
            .map(|c| format!("{BASE}{c}"))
            .filter(|p| std::path::Path::new(p).is_dir())
            .collect()
    }

    // ===== Kotlin → Rust 回调（JNI 导出） =====

    fn emit(name: &str, payload: impl serde::Serialize + std::clone::Clone) {
        if let Some(app) = APP_HANDLE.get() {
            let _ = app.emit(name, payload);
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_com_hao_lumo_MainActivity_lumoNativeOnBackPressed(
        _env: JNIEnv,
        _this: JObject,
    ) {
        emit("lumo-back-pressed", ());
    }

    #[no_mangle]
    pub extern "system" fn Java_com_hao_lumo_MainActivity_lumoNativePermissionResult(
        _env: JNIEnv,
        _this: JObject,
        granted: jni::sys::jboolean,
    ) {
        emit("lumo-permission-result", granted != 0);
    }

    #[no_mangle]
    pub extern "system" fn Java_com_hao_lumo_MediaPlaybackServiceKt_lumoNativeMediaAction(
        _env: JNIEnv,
        _this: JObject,
        action: jni::sys::jint,
        param: jni::sys::jlong,
    ) {
        if let Some(app) = APP_HANDLE.get() {
            let app_handle = app.clone();
            std::thread::spawn(move || {
                match action {
                    1 => { // ACTION_PLAY
                        if let Some(ps) = app_handle.try_state::<crate::commands::playback::PlaybackState>() {
                            if let Ok(m) = ps.manager.lock() {
                                m.resume();
                                let pos = m.get_pos();
                                drop(m);
                                if let Some(qs) = app_handle.try_state::<crate::services::queue::QueueState>() {
                                    if let Ok(q) = qs.queue.lock() {
                                        if let Some(item) = q.current_item() {
                                            let _ = update_foreground(&item.title, &item.artist, &item.album, true, pos, item.duration_ms.unwrap_or(0));
                                        }
                                    }
                                }
                                let _ = app_handle.emit("playback-status-changed", serde_json::json!({ "is_playing": true }));
                            }
                        }
                    }
                    2 => { // ACTION_PAUSE
                        if let Some(ps) = app_handle.try_state::<crate::commands::playback::PlaybackState>() {
                            if let Ok(m) = ps.manager.lock() {
                                m.pause();
                                let pos = m.get_pos();
                                drop(m);
                                if let Some(qs) = app_handle.try_state::<crate::services::queue::QueueState>() {
                                    if let Ok(q) = qs.queue.lock() {
                                        if let Some(item) = q.current_item() {
                                            let _ = update_foreground(&item.title, &item.artist, &item.album, false, pos, item.duration_ms.unwrap_or(0));
                                        }
                                    }
                                }
                                let _ = app_handle.emit("playback-status-changed", serde_json::json!({ "is_playing": false }));
                            }
                        }
                    }
                    3 => { // ACTION_NEXT
                        if let (Some(qs), Some(ps)) = (
                            app_handle.try_state::<crate::services::queue::QueueState>(),
                            app_handle.try_state::<crate::commands::playback::PlaybackState>()
                        ) {
                            let _ = crate::commands::queue::playback_advance(app_handle.clone(), qs, ps, 1);
                        }
                    }
                    4 => { // ACTION_PREV
                        if let (Some(qs), Some(ps)) = (
                            app_handle.try_state::<crate::services::queue::QueueState>(),
                            app_handle.try_state::<crate::commands::playback::PlaybackState>()
                        ) {
                            let _ = crate::commands::queue::playback_advance(app_handle.clone(), qs, ps, -1);
                        }
                    }
                    5 => { // ACTION_SEEK
                        if let Some(ps) = app_handle.try_state::<crate::commands::playback::PlaybackState>() {
                            if let Ok(m) = ps.manager.lock() { let _ = m.try_seek(param as u64); }
                        }
                    }
                    6 => { // ACTION_STOP
                        if let Some(ps) = app_handle.try_state::<crate::commands::playback::PlaybackState>() {
                            if let Ok(m) = ps.manager.lock() { m.stop(); }
                        }
                        let _ = stop_foreground();
                        let _ = app_handle.emit("playback-status-changed", serde_json::json!({ "is_playing": false }));
                    }
                    _ => {}
                }
            });
        }
    }
}

#[cfg(target_os = "android")]
pub use android::*;

/// 桌面端空实现（移动 UI 不触达这些路径，返回安全默认值）。
#[cfg(not(target_os = "android"))]
pub mod desktop {
    pub fn set_app_handle(_app: tauri::AppHandle) {}
    pub fn has_audio_permission() -> bool {
        true
    }
    pub fn request_audio_permission() -> Result<(), String> {
        Ok(())
    }
    pub fn open_app_settings() -> Result<(), String> {
        Ok(())
    }
    pub fn finish_app() -> Result<(), String> {
        Ok(())
    }
    pub fn restart_app() -> Result<(), String> {
        Ok(())
    }
    pub fn storage_suggestions() -> Vec<String> {
        Vec::new()
    }
    pub fn update_foreground(
        _title: &str,
        _artist: &str,
        _album: &str,
        _is_playing: bool,
        _position_ms: u64,
        _duration_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
    pub fn stop_foreground() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop::*;
