use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tracing::{info, warn};

/// Tauri 全局状态包装，用于 app.manage() 注册。
pub struct AudioCacheState {
    pub cache: Mutex<AudioCache>,
}

/// 全局下载中集合：确保同一首歌不会重复下载。
/// 用全局 static 而非 AudioCache 内部字段，是因为后台下载线程
/// 在 download 完成/失败后需要清理标记，而 Background 线程拿不到 Tauri State。
static DOWNLOADING: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// 标记某首歌开始下载。返回 true 表示成功抢占，false 表示已有下载进行。
pub fn mark_downloading(media_file_id: i64) -> bool {
    DOWNLOADING
        .lock()
        .map(|mut set| set.insert(media_file_id))
        .unwrap_or(false)
}

/// 标记下载结束（播放命令后台线程调用）
pub fn unmark_downloading(media_file_id: i64) {
    if let Ok(mut set) = DOWNLOADING.lock() {
        set.remove(&media_file_id);
    }
}

/// 是否正在下载中
pub fn is_downloading(media_file_id: i64) -> bool {
    DOWNLOADING
        .lock()
        .map(|set| set.contains(&media_file_id))
        .unwrap_or(false)
}

/// 云端音频文件透明缓存服务。
///
/// 设计目标：用户播放 WebDAV 歌曲时，后台异步下载完整文件到本地缓存目录，
/// 下次播放同一首歌时直接走本地文件路径，实现「零网络请求」秒开 + gapless。
///
/// 存储路径：`{app_data_dir}/audio_cache/{media_file_id}`
/// （用 media_file_id 作文件名，因为它是 DB 中物理文件的唯一标识）
///
/// 并发控制：同一首歌不会重复下载，用全局 static DOWNLOADING HashSet（见上）。
///
/// 原子写入：先下载到 `.tmp` 临时文件，完成后 rename，避免半截文件被当成有效缓存。
///
/// 缓存淘汰：MVP 阶段不做 LRU，仅提供手动清除（设置页）。后续可加大小阈值淘汰。
pub struct AudioCache {
    cache_dir: PathBuf,
}

impl AudioCache {
    /// 创建缓存服务，自动建目录。`app_data_dir` 通常是 Tauri 的 app_data_dir。
    pub fn new(app_data_dir: &Path) -> Self {
        let cache_dir = app_data_dir.join("audio_cache");
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            warn!("Failed to create audio cache dir {:?}: {}", cache_dir, e);
        }
        info!("Audio cache initialized at {:?}", cache_dir);
        Self { cache_dir }
    }

    /// 某个 media_file_id 对应的缓存文件完整路径
    pub fn cache_path(&self, media_file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}", media_file_id))
    }

    /// 临时文件路径（下载中用，完成后 rename）
    fn tmp_path(&self, media_file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}.tmp", media_file_id))
    }

    /// 是否已缓存（文件存在且大小 > 0）
    pub fn is_cached(&self, media_file_id: i64) -> bool {
        self.get_cached_path(media_file_id).is_some()
    }

    /// 获取已缓存文件的本地路径，未缓存返回 None
    pub fn get_cached_path(&self, media_file_id: i64) -> Option<PathBuf> {
        let path = self.cache_path(media_file_id);
        match fs::metadata(&path) {
            Ok(meta) if meta.len() > 0 => Some(path),
            _ => None,
        }
    }

    /// 阻塞下载 WebDAV 文件到缓存（原子写入：先 .tmp 后 rename）。
    /// 成功后返回缓存文件的本地路径。调用方需先在外部通过 `mark_downloading` 做并发控制。
    pub fn store_from_webdav(
        &self,
        media_file_id: i64,
        file_url: &str,
        client: &crate::services::webdav::WebdavClient,
    ) -> Result<PathBuf, String> {
        let tmp = self.tmp_path(media_file_id);
        let final_path = self.cache_path(media_file_id);

        // 下载到临时文件
        let bytes = client.download_to_file(file_url, &tmp)?;
        info!(
            "Audio cache downloaded media_file_id={} ({} bytes)",
            media_file_id, bytes
        );

        if bytes == 0 {
            // 空文件，清理并报错
            let _ = fs::remove_file(&tmp);
            return Err("Downloaded file is empty".to_string());
        }

        // 原子 rename 到最终路径
        fs::rename(&tmp, &final_path)
            .map_err(|e| format!("Failed to finalize cache file: {}", e))?;

        Ok(final_path)
    }

    /// 清空整个音频缓存目录
    pub fn clear(&self) -> Result<u64, String> {
        let mut freed: u64 = 0;
        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("Failed to read cache dir: {}", e))?;
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                freed += meta.len();
            }
            if let Err(e) = fs::remove_file(entry.path()) {
                warn!("Failed to remove cache file {:?}: {}", entry.path(), e);
            }
        }
        info!("Audio cache cleared, freed {} bytes", freed);
        Ok(freed)
    }

    /// 计算缓存目录总大小（字节）
    pub fn size_bytes(&self) -> u64 {
        let mut total: u64 = 0;
        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }
}
