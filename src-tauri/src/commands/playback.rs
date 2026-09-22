use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::services::cache::{AudioCache, AudioCacheState, DownloadGuard, DEFAULT_MAX_BYTES};
use crate::services::playback::PlaybackManager;
use crate::services::webdav::WebdavClient;
use rusqlite::OptionalExtension;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{Manager, State};

pub struct PlaybackState {
    pub manager: Mutex<PlaybackManager>,
    /// 音频能量的原子量（与 manager 内部共享同一份）。
    /// `playback_get_level` 是 30Hz 高频采样，必须**绕过 manager 锁**直接读，
    /// 否则任何一条慢命令（远端流解码等）都会把采样全部堵在锁上。
    pub level: Arc<AtomicU32>,
}

/// 解析媒体文件到可播放的源。
///
/// 缓存优先策略（迭代一新增）：
/// - 先查本地音频缓存，命中则返回 PathBuf（走 play_file / enqueue_next_file，零网络 + gapless）
/// - 未命中：
///   - local → 返回本地文件路径
///   - webdav → 返回 HttpRangeReader（流播），同时调用方会 spawn 后台线程下载缓存
///
/// 返回 (本地路径, WebDAV 流)。两者互斥：命中缓存或本地文件时 stream 为 None。
pub fn resolve_media_file(
    db_state: &State<'_, DbState>,
    audio_cache: &AudioCache,
    mut media_file_id: i64,
    key: &[u8; 32],
    force_local: bool,
) -> Result<
    (
        Option<PathBuf>,
        Option<crate::services::webdav::HttpRangeReader>,
        WebdavResolveInfo,
    ),
    AppError,
> {
    let conn = db_state.db.get()?;

    // 如果要求强行使用本地版本（断网降级），我们查找当前 track_id 下最好的 local 音源
    if force_local {
        let local_fallback_id: Option<i64> = conn
            .query_row(
                "SELECT mf.id FROM media_files mf 
             JOIN sources s ON s.id = mf.source_id 
             WHERE mf.track_id = (SELECT track_id FROM media_files WHERE id = ?1) 
               AND s.kind = 'local' 
               AND mf.availability = 'available' 
             ORDER BY mf.file_size DESC LIMIT 1",
                rusqlite::params![media_file_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = local_fallback_id {
            media_file_id = id;
            tracing::info!("Offline auto-degraded to local media_file_id={}", id);
        } else if !audio_cache.is_cached(media_file_id, None) {
            return Err(AppError::Internal(
                "离线模式下没有可用的本地文件或音频缓存".to_string(),
            ));
        }
    }

    let (source_id, relative_path, root_uri, kind, cred, size): (
        i64,
        String,
        String,
        String,
        Option<String>,
        i64,
    ) = conn.query_row(
        "SELECT mf.source_id, mf.relative_path, s.root_uri, s.kind, s.credential_ref, mf.file_size
         FROM media_files mf JOIN sources s ON mf.source_id = s.id
         WHERE mf.id = ?1",
        rusqlite::params![media_file_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;

    // ===== 缓存优先：WebDAV 文件已缓存且大小与 DB 记录一致才走本地路径 =====
    // 比对 file_size 是 G-10 的修法：一次截断下载若被当成有效缓存，之后每次播放都会坏。
    // file_size <= 0 表示部分服务端不回 getcontentlength，此时无期望值可校验。
    if kind == "webdav" {
        let expected_size = (size > 0).then_some(size as u64);
        if let Some(cached) = audio_cache.acquire_cached_path(media_file_id, expected_size) {
            tracing::info!("Audio cache hit for media_file_id={}", media_file_id);
            return Ok((Some(cached), None, WebdavResolveInfo::default()));
        }
    }

    if kind == "webdav" {
        // 凭据解析（P1-08 统一入口）：支持钥匙串引用 / V6 加密 / V5 明文；
        // 桌面端解析成功后懒迁移进系统钥匙串。解析失败按分类错误透出给前端 toast。
        let (username, password): (Option<String>, Option<String>) = match cred.as_deref() {
            Some(cred) => {
                let (u, p) = crate::commands::scanner::resolve_source_credential(
                    &conn, source_id, cred, key,
                )?;
                (Some(u), p)
            }
            None => (None, None),
        };
        // 联网行为: §2E —— 远程曲库的 Range 读
        let webdav = WebdavClient::new(root_uri.clone(), username, password);
        let base_str = if root_uri.ends_with('/') {
            root_uri.clone()
        } else {
            format!("{}/", root_uri)
        };
        let base = reqwest::Url::parse(&base_str).map_err(|e| AppError::Internal(e.to_string()))?;
        let relative_url_path = relative_path.replace('\\', "/");
        let file_url = base
            .join(&relative_url_path)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();

        // 能力门控：来源支持 Range → 流播；不支持（或不曾探测）→ 探测/整文件下载降级。
        // 探测结果落 source_capabilities 表，7 天内复用，不重复发请求。
        let supports_range = match load_range_support(&conn, source_id)? {
            Some(v) => v,
            None => probe_and_persist_range_support(&conn, &webdav, source_id, &file_url)?,
        };

        if supports_range {
            let http_reader = crate::services::webdav::HttpRangeReader::new(
                &webdav,
                file_url.clone(),
                size as u64,
            );
            Ok((
                None,
                Some(http_reader),
                WebdavResolveInfo {
                    webdav_client: Some(webdav),
                    file_url: Some(file_url),
                    expected_size: (size > 0).then_some(size as u64),
                },
            ))
        } else {
            // 服务器不支持分段读取：整文件下载进缓存后按本地文件播放。
            // 同步等待期间前端停留在 isBuffering，失败按分类文案报错。
            let path = download_full_to_cache(
                audio_cache,
                &webdav,
                media_file_id,
                &file_url,
                (size > 0).then_some(size as u64),
            )?;
            Ok((Some(path), None, WebdavResolveInfo::default()))
        }
    } else {
        Ok((
            Some(PathBuf::from(&root_uri).join(relative_path)),
            None,
            WebdavResolveInfo::default(),
        ))
    }
}

/// 读取来源的 Range 能力缓存记录；缺失、未探测过或超过 7 天视为 None。
fn load_range_support(
    conn: &rusqlite::Connection,
    source_id: i64,
) -> Result<Option<bool>, AppError> {
    let v: Option<i64> = conn
        .query_row(
            "SELECT supports_range FROM source_capabilities
         WHERE source_id = ?1 AND supports_range IS NOT NULL
           AND julianday('now') - julianday(checked_at) <= 7",
            rusqlite::params![source_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(v.map(|v| v != 0))
}

/// 用真实文件 URL 探测 Range 支持并落库。探测失败（认证/网络等）返回分类后的可读错误。
fn probe_and_persist_range_support(
    conn: &rusqlite::Connection,
    webdav: &WebdavClient,
    source_id: i64,
    file_url: &str,
) -> Result<bool, AppError> {
    let supports_range = webdav
        .probe_range_support(file_url)
        .map_err(AppError::Internal)?;
    conn.execute(
        "INSERT INTO source_capabilities (source_id, supports_range, checked_at, raw_json)
         VALUES (?1, ?2, datetime('now'), '{}')
         ON CONFLICT(source_id) DO UPDATE SET supports_range = ?2, checked_at = datetime('now')",
        rusqlite::params![source_id, supports_range],
    )?;
    tracing::info!(
        "WebDAV source_id={} supports_range={} (probed)",
        source_id,
        supports_range
    );
    Ok(supports_range)
}

/// 整文件同步下载进音频缓存（服务器不支持 Range 时的播放降级路径）。
/// 与后台缓存下载共用 [`DownloadGuard`]，防止两条路径同时写同一个 media_file_id。
fn download_full_to_cache(
    audio_cache: &AudioCache,
    webdav: &WebdavClient,
    media_file_id: i64,
    file_url: &str,
    expected_size: Option<u64>,
) -> Result<PathBuf, AppError> {
    let Some(_guard) = DownloadGuard::try_new(media_file_id) else {
        return Err(AppError::Internal(
            "该歌曲正在缓存中，请稍后再试".to_string(),
        ));
    };
    audio_cache
        .store_from_webdav(media_file_id, file_url, webdav, expected_size)
        .map_err(AppError::Internal)
}

/// WebDAV 解析附加信息：用于流播失败后后台缓存下载。
/// 仅在 WebDAV 未命中缓存时填充。
#[derive(Default)]
pub struct WebdavResolveInfo {
    pub webdav_client: Option<WebdavClient>,
    pub file_url: Option<String>,
    /// DB 记录的文件大小，用于校验下载完整性；服务端未上报时为 None。
    pub expected_size: Option<u64>,
}

/// 在后台线程异步下载 WebDAV 文件到缓存（播放同时进行，不阻塞音频）。
/// 下载完成或失败都不影响当前播放，仅影响「下次播放这首歌」的缓存命中。
pub fn spawn_background_cache_download(
    audio_cache_state: &State<'_, AudioCacheState>,
    media_file_id: i64,
    webdav_client: WebdavClient,
    file_url: String,
    expected_size: Option<u64>,
) {
    // 先抢下载权再查缓存：两个动作之间没有窗口，也不会出现
    // "检查通过但抢权失败" 时把别人的下载标记顺手清掉的情况。
    let Some(guard) = DownloadGuard::try_new(media_file_id) else {
        return;
    };
    let cache_guard = match audio_cache_state.cache.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if cache_guard.is_cached(media_file_id, expected_size) {
        return;
    }
    // 缓存对象常驻，后台线程只需要目录路径
    let cache_dir = cache_guard.cache_dir().to_path_buf();
    drop(cache_guard);

    std::thread::spawn(move || {
        // guard 随线程结束释放：包括 panic 展开，不会再出现永久"正在缓存中"
        let _guard = guard;
        let cache = AudioCache::from_cache_dir(cache_dir);
        match cache.store_from_webdav(media_file_id, &file_url, &webdav_client, expected_size) {
            Ok(_) => {
                cache.prune_to_max_bytes(DEFAULT_MAX_BYTES);
            }
            Err(e) => {
                tracing::warn!(
                    "Audio cache download failed for media_file_id={}: {}",
                    media_file_id,
                    e
                );
            }
        }
    });
}

#[tauri::command(async)]
pub fn playback_play(
    app: tauri::AppHandle,
    playback_state: State<'_, PlaybackState>,
    db_state: State<'_, DbState>,
    cache_state: State<'_, AudioCacheState>,
    media_file_id: i64,
    force_local: Option<bool>,
) -> Result<Option<u64>, AppError> {
    let _trace = ipc_trace!("playback_play");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(
        &db_state,
        &audio_cache,
        media_file_id,
        &key,
        force_local.unwrap_or(false),
    )?;
    drop(audio_cache); // 释放缓存锁，不阻塞后续播放

    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let duration = if let Some(reader) = webdav_reader {
        // WebDAV 流播（expected_size 作为 byte_len 传给 symphonia，否则 seek 会报 Unseekable）
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        let dur = manager.play_stream(buffered_reader, webdav_info.expected_size)?;

        // 后台异步下载缓存（不影响当前播放）
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            let expected_size = webdav_info.expected_size;
            drop(manager); // 释放播放锁再 spawn
            spawn_background_cache_download(
                &cache_state,
                media_file_id,
                client,
                url,
                expected_size,
            );
        }
        dur
    } else if let Some(path) = path_buf {
        // 本地文件或缓存命中
        manager.play_file(&path)?
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    };

    Ok(duration)
}

#[tauri::command(async)]
pub fn playback_enqueue_next(
    app: tauri::AppHandle,
    playback_state: State<'_, PlaybackState>,
    db_state: State<'_, DbState>,
    cache_state: State<'_, AudioCacheState>,
    media_file_id: i64,
    force_local: Option<bool>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_enqueue_next");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let key = crate::commands::scanner::derive_credential_key(&app_dir);

    let audio_cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let (path_buf, webdav_reader, webdav_info) = resolve_media_file(
        &db_state,
        &audio_cache,
        media_file_id,
        &key,
        force_local.unwrap_or(false),
    )?;
    drop(audio_cache);

    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(reader) = webdav_reader {
        // WebDAV 流式 gapless：直接 append 到 sink（不 stop，无缝衔接）
        let buffered_reader = std::io::BufReader::with_capacity(64 * 1024, reader);
        manager.enqueue_next_stream(buffered_reader, webdav_info.expected_size)?;

        // 后台异步下载缓存
        if let (Some(client), Some(url)) = (webdav_info.webdav_client, webdav_info.file_url) {
            let expected_size = webdav_info.expected_size;
            drop(manager);
            spawn_background_cache_download(
                &cache_state,
                media_file_id,
                client,
                url,
                expected_size,
            );
        }
    } else if let Some(path) = path_buf {
        // 本地文件或缓存命中 → 标准 gapless
        manager
            .enqueue_next_file(&path)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    } else {
        return Err(AppError::Internal("No playable source found".to_string()));
    }
    Ok(())
}

#[tauri::command(async)]
pub fn playback_get_queue_len(playback_state: State<'_, PlaybackState>) -> Result<usize, AppError> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.get_queue_len())
}

#[tauri::command(async)]
pub fn playback_pause(
    playback_state: State<'_, PlaybackState>,
    queue_state: State<'_, crate::services::queue::QueueState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_pause");
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager.pause();
    let pos = manager.get_pos();
    if let Ok(q) = queue_state.queue.lock() {
        if let Some(item) = q.items.get(q.index) {
            let _ = crate::services::platform::update_foreground(
                &item.title,
                &item.artist,
                &item.album,
                false,
                pos,
                item.duration_ms.unwrap_or(0),
            );
        }
    }
    Ok(())
}

#[tauri::command(async)]
pub fn playback_resume(
    playback_state: State<'_, PlaybackState>,
    queue_state: State<'_, crate::services::queue::QueueState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_resume");
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager.resume();
    let pos = manager.get_pos();
    if let Ok(q) = queue_state.queue.lock() {
        if let Some(item) = q.items.get(q.index) {
            let _ = crate::services::platform::update_foreground(
                &item.title,
                &item.artist,
                &item.album,
                true,
                pos,
                item.duration_ms.unwrap_or(0),
            );
        }
    }
    Ok(())
}

#[tauri::command(async)]
pub fn playback_stop(
    playback_state: State<'_, PlaybackState>,
    queue_state: State<'_, crate::services::queue::QueueState>,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_stop");
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager.stop();
    // 用户主动停止：旧的自动重试目标与退避一并作废（CR-002）。播放结束后观察者会把
    // 「停掉」当成播完，若还挂着待重试的那一首，它会自作主张地补播一遍。
    if let Ok(mut q) = queue_state.queue.lock() {
        q.retry.clear();
    }
    let _ = crate::services::platform::stop_foreground();
    Ok(())
}

#[tauri::command(async)]
pub fn playback_set_volume(
    playback_state: State<'_, PlaybackState>,
    volume: f32,
) -> Result<(), AppError> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager.set_volume(volume);
    Ok(())
}

/// 设置播放速率（1.0 原速；前端提供 0.5/0.8/1/1.2/1.5）。
/// 立即生效，且对正在播放的曲目同样有效（rodio 音频线程每 5ms 取一次该值）。
#[tauri::command(async)]
pub fn playback_set_speed(
    playback_state: State<'_, PlaybackState>,
    speed: f32,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_set_speed");
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager.set_speed(speed);
    Ok(())
}

#[tauri::command(async)]
pub fn playback_get_speed(playback_state: State<'_, PlaybackState>) -> Result<f32, AppError> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.get_speed())
}

#[tauri::command(async)]
pub fn playback_get_pos(playback_state: State<'_, PlaybackState>) -> Result<u64, AppError> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.get_pos())
}

/// 读取当前音频能量（RMS，0.0–1.0），用于沉浸式播放页的封面「随音乐呼吸」。
///
/// 仅在沉浸式页可见且正在播放时由前端以约 30Hz 采样，非播放态不采样。
///
/// 两个刻意的设计（缺一不可，缺了就会出现 186 个采样堆在通道里的事故）：
/// 1. `#[tauri::command(async)]`：不占主线程。同步命令在主线程串行执行，
///    任何一条慢命令都会让排在后面的采样全部堆积。
/// 2. **不经过 manager 锁**（直接读原子量）：远端流解码等操作会长时间持有
///    manager 锁，采样一旦去抢锁就会逐个卡住、把整条 IPC 通道堵死。
#[tauri::command(async)]
pub fn playback_get_level(playback_state: State<'_, PlaybackState>) -> Result<f32, AppError> {
    Ok(f32::from_bits(
        playback_state.level.load(Ordering::Relaxed),
    ))
}

#[tauri::command(async)]
pub fn playback_seek(
    playback_state: State<'_, PlaybackState>,
    position_ms: u64,
) -> Result<(), AppError> {
    let _trace = ipc_trace!("playback_seek");
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    manager
        .try_seek(position_ms)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

#[tauri::command(async)]
pub fn playback_is_finished(playback_state: State<'_, PlaybackState>) -> Result<bool, AppError> {
    let manager = playback_state
        .manager
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(manager.is_finished())
}

/// 查询某首曲目是否已缓存到本地（前端用于离线置灰判断）。
/// 与播放路径同口径校验文件大小：只判存在会把截断缓存报成"已缓存"，
/// 用户离线时才发现放不出来。
#[tauri::command(async)]
pub fn playback_is_cached(
    db_state: State<'_, DbState>,
    cache_state: State<'_, AudioCacheState>,
    media_file_id: i64,
) -> Result<bool, AppError> {
    let size: Option<i64> = db_state
        .db
        .get()?
        .query_row(
            "SELECT file_size FROM media_files WHERE id = ?1",
            rusqlite::params![media_file_id],
            |row| row.get(0),
        )
        .optional()?;
    let expected_size = size.unwrap_or(0).max(0) as u64;
    let cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(cache.is_cached(media_file_id, (expected_size > 0).then_some(expected_size)))
}

/// 清空全部音频缓存，返回释放的字节数。
#[tauri::command(async)]
pub fn playback_clear_audio_cache(
    cache_state: State<'_, AudioCacheState>,
) -> Result<u64, AppError> {
    let cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cache.clear().map_err(|e| AppError::Internal(e.to_string()))
}

/// 获取音频缓存总大小（字节），用于设置页显示。
#[tauri::command(async)]
pub fn playback_get_audio_cache_size(
    cache_state: State<'_, AudioCacheState>,
) -> Result<u64, AppError> {
    let cache = cache_state
        .cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(cache.size_bytes())
}
