use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tracing::{info, warn};

/// Tauri 全局状态包装，用于 app.manage() 注册。
pub struct AudioCacheState {
    pub cache: Mutex<AudioCache>,
}

/// 音频缓存默认上限：约 2GB。
pub const DEFAULT_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 下载中的临时文件后缀。淘汰时必须跳过——它在写入过程中。
const TMP_SUFFIX: &str = ".tmp";

fn is_tmp_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(TMP_SUFFIX))
}

/// 全局下载中集合：确保同一首歌不会重复下载。
/// 用全局 static 而非 AudioCache 内部字段，是因为后台下载线程
/// 在 download 完成/失败后需要清理标记，而 Background 线程拿不到 Tauri State。
static DOWNLOADING: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// 一次下载的所有权凭证：Drop 时释放「正在缓存中」标记。
///
/// 用 RAII 而非手工 `unmark_downloading`：下载线程在 rename 前后 panic 时，
/// 手工清理会被跳过，该 media_file_id 会**永久**留在集合里，
/// 之后每次播放这首歌都直接返回「正在缓存中」，用户端表现为缓存功能彻底卡死且只能重启进程。
#[derive(Debug)]
pub struct DownloadGuard {
    media_file_id: i64,
}

impl DownloadGuard {
    /// 抢占某首歌的下载权；已有下载在进行时返回 None。
    /// 抢占成功即持有所有权，释放（含 panic 展开）自动结束"正在缓存中"状态。
    pub fn try_new(media_file_id: i64) -> Option<Self> {
        let acquired = DOWNLOADING
            .lock()
            .map(|mut set| set.insert(media_file_id))
            .unwrap_or(false);
        acquired.then_some(Self { media_file_id })
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = DOWNLOADING.lock() {
            set.remove(&self.media_file_id);
        }
    }
}

/// 云端音频文件透明缓存服务。
///
/// 设计目标：用户播放 WebDAV 歌曲时，后台异步下载完整文件到本地缓存目录，
/// 下次播放同一首歌时直接走本地文件路径，实现「零网络请求」秒开 + gapless。
///
/// 存储路径：`{app_data_dir}/audio_cache/{media_file_id}`
/// （用 media_file_id 作文件名，因为它是 DB 中物理文件的唯一标识）
///
/// 并发控制：同一首歌不会重复下载，见 [`DownloadGuard`]（全局 static，因后台线程拿不到 Tauri State）。
///
/// 原子写入：先下载到 `.tmp`，校验字节数后 rename，任何失败路径都会删掉临时文件。
///
/// 有效性判定：命中缓存必须与 DB 记录的 `file_size` 一致。只判「文件非空」会让
/// 一次截断下载永久变成坏缓存——播放时才发现，且用户无法自愈。
///
/// 缓存淘汰：按 mtime 升序清到上限之下。mtime 表示「最近一次真正播放」而非下载时间
/// ——只有 [`AudioCache::acquire_cached_path`] 会推进它，状态查询不会（CR-005），
/// 否则正在播放的老文件会被第一个淘汰，而只是被列表扫过的文件活得最久。
pub struct AudioCache {
    cache_dir: PathBuf,
}

impl AudioCache {
    /// 创建缓存服务，自动建目录。`app_data_dir` 通常是 Tauri 的 app_data_dir。
    pub fn new(app_data_dir: &Path) -> Self {
        Self::from_cache_dir(app_data_dir.join("audio_cache"))
    }

    /// 直接以缓存目录构造：后台下载线程只持有目录路径，没有 app_data_dir。
    /// 拆出来是为了避免调用方自己 `join("audio_cache")` 造成嵌套目录。
    pub fn from_cache_dir(cache_dir: PathBuf) -> Self {
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            warn!("Failed to create audio cache dir {:?}: {}", cache_dir, e);
        }
        info!("Audio cache initialized at {:?}", cache_dir);
        Self { cache_dir }
    }

    /// 缓存根目录（测试与诊断用）
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// 某个 media_file_id 对应的缓存文件完整路径
    pub fn cache_path(&self, media_file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}", media_file_id))
    }

    /// 临时文件路径（下载中用，完成后 rename）
    fn tmp_path(&self, media_file_id: i64) -> PathBuf {
        self.cache_dir
            .join(format!("{}{}", media_file_id, TMP_SUFFIX))
    }

    /// 是否已缓存且大小与远端一致。`expected_size` 取自 `media_files.file_size`，
    /// 未知时传 None（退化为「非空即有效」）。
    ///
    /// 纯查询：**不**刷新 mtime。可播性批量查询和 `playback_is_cached` 都会逐首调用它，
    /// 若顺手 touch，打开一次列表就等于把整批缓存标记成"刚刚使用"，
    /// 淘汰顺序会变成列表浏览顺序而不是真实播放历史（CR-005）。
    pub fn is_cached(&self, media_file_id: i64, expected_size: Option<u64>) -> bool {
        self.validate_cached_path(media_file_id, expected_size)
            .is_some()
    }

    /// 只校验缓存是否有效（存在、非空、大小与 `expected_size` 一致），**不更新 mtime**。
    /// 命中但体积不符时仍会删掉坏缓存——那是数据修正，不是"使用"。
    pub fn validate_cached_path(
        &self,
        media_file_id: i64,
        expected_size: Option<u64>,
    ) -> Option<PathBuf> {
        let path = self.cache_path(media_file_id);
        let meta = fs::metadata(&path).ok()?;
        if meta.len() == 0 {
            return None;
        }
        if let Some(expected) = expected_size {
            if meta.len() != expected {
                warn!(
                    "丢弃尺寸不符的音频缓存 media_file_id={}：本地 {} 字节，远端 {} 字节",
                    media_file_id,
                    meta.len(),
                    expected
                );
                // 删掉而不是每次告警：下次播放会重新下载并覆盖
                if let Err(e) = fs::remove_file(&path) {
                    warn!("删除坏缓存失败 {:?}: {}", path, e);
                }
                return None;
            }
        }
        Some(path)
    }

    /// 取缓存路径**并**把 mtime 推到当前时间，供 [`Self::prune_to_max_bytes`] 做近似 LRU。
    /// 只有真正把缓存文件交给播放器时才调用——"用过一次"才应该改变它的淘汰顺序。
    pub fn acquire_cached_path(
        &self,
        media_file_id: i64,
        expected_size: Option<u64>,
    ) -> Option<PathBuf> {
        let path = self.validate_cached_path(media_file_id, expected_size)?;
        touch_mtime(&path);
        Some(path)
    }

    /// 阻塞下载 WebDAV 文件到缓存（原子写入：先 .tmp 校验后 rename）。
    /// 成功后返回缓存文件的本地路径。调用方需持有 [`DownloadGuard`] 做并发控制。
    ///
    /// `expected_size` 为 DB 记录的文件大小：不一致即判定为截断/被替换的响应，
    /// 丢弃临时文件并返回错误，绝不落地成正式缓存。
    pub fn store_from_webdav(
        &self,
        media_file_id: i64,
        file_url: &str,
        client: &crate::services::webdav::WebdavClient,
        expected_size: Option<u64>,
    ) -> Result<PathBuf, String> {
        let tmp = self.tmp_path(media_file_id);
        let final_path = self.cache_path(media_file_id);

        // 任何失败路径都要清掉临时文件，否则坏 .tmp 会长期占盘并被后续误用
        // 体积同时用于推导传输总预算：服务器停滞时请求要在明确时间内返回（CR-007）
        let result = client.download_to_file(file_url, &tmp, expected_size);
        let bytes = match result {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
        };
        if let Err(e) = Self::assert_download_size(bytes, expected_size) {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }

        if let Err(e) = fs::rename(&tmp, &final_path) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("缓存写入失败: {}", e));
        }

        info!(
            "Audio cache stored media_file_id={} ({} bytes)",
            media_file_id, bytes
        );
        Ok(final_path)
    }

    fn assert_download_size(bytes: u64, expected_size: Option<u64>) -> Result<(), String> {
        if bytes == 0 {
            return Err("下载内容为空".to_string());
        }
        if let Some(expected) = expected_size {
            if bytes != expected {
                return Err(format!(
                    "下载不完整：收到 {} 字节，服务器记录 {} 字节",
                    bytes, expected
                ));
            }
        }
        Ok(())
    }

    /// 清空整个音频缓存目录，返回**实际**释放的字节数。
    /// 正在被占用的文件（Windows 上播放中的文件句柄）删除会失败，
    /// 只告警并跳过，不计入释放量——否则设置页会显示"已清空 X GB"而磁盘没变。
    pub fn clear(&self) -> Result<u64, String> {
        let mut freed: u64 = 0;
        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("Failed to read cache dir: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            match fs::remove_file(&path) {
                Ok(_) => freed += size,
                Err(e) => warn!("缓存文件占用中，跳过 {:?}: {}", path, e),
            }
        }
        info!("Audio cache cleared, freed {} bytes", freed);
        Ok(freed)
    }

    /// 计算缓存目录总大小（字节），含下载中的临时文件
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

    /// [MA3 A3-3] 按最旧使用时间清理缓存直到低于 max_bytes 上限。
    /// 跳过 `.tmp`：那是正在写入的文件，删掉会让正在进行的下载在 rename 时才失败。
    pub fn prune_to_max_bytes(&self, max_bytes: u64) -> u64 {
        let mut files: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
        let mut total_size: u64 = 0;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_tmp_path(&path) {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        let size = meta.len();
                        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        total_size += size;
                        files.push((path, size, mtime));
                    }
                }
            }
        }

        if total_size <= max_bytes {
            return 0;
        }

        // 按修改时间升序排列（最久未用的在前面）
        files.sort_by_key(|f| f.2);

        let mut freed: u64 = 0;
        for (path, size, _) in files {
            if total_size.saturating_sub(freed) <= max_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                freed += size;
            }
        }
        info!(
            "Audio cache pruned, freed {} bytes, remaining ~{} bytes",
            freed,
            total_size.saturating_sub(freed)
        );
        freed
    }
}

/// 把文件 mtime 设为当前时间；失败只告警（不影响播放，只影响淘汰顺序）。
fn touch_mtime(path: &Path) {
    let file = match fs::OpenOptions::new().write(true).open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!("无法打开缓存文件以更新访问时间 {:?}: {}", path, e);
            return;
        }
    };
    if let Err(e) = file.set_modified(std::time::SystemTime::now()) {
        warn!("无法更新缓存文件访问时间 {:?}: {}", path, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_util::TempDir;
    use std::time::{Duration, SystemTime};

    fn write_entry(dir: &Path, name: &str, len: usize) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, vec![0u8; len]).expect("写入测试缓存文件失败");
        path
    }

    fn set_age(path: &Path, secs: u64) {
        let old = SystemTime::now() - Duration::from_secs(secs);
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("打开测试文件失败")
            .set_modified(old)
            .expect("设置 mtime 失败");
    }

    fn age_secs(path: &Path) -> u64 {
        let mtime = fs::metadata(path).expect("文件应存在").modified().unwrap();
        SystemTime::now()
            .duration_since(mtime)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// G-10：截断的缓存文件不能被判为有效，且要被删掉以便重新下载
    #[test]
    fn truncated_cache_is_rejected_and_removed() {
        let tmp = TempDir::new("cache_truncated");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        let path = write_entry(tmp.path(), "101", 500);

        assert!(
            !cache.is_cached(101, Some(1000)),
            "本地 500 字节不应匹配远端 1000 字节"
        );
        assert!(!path.exists(), "坏缓存应被删除，否则下次仍会命中");
    }

    /// 大小一致才算命中；命中要更新 mtime（否则正在播放的文件会被淘汰掉）
    #[test]
    fn intact_cache_hits_and_refreshes_mtime() {
        let tmp = TempDir::new("cache_hit");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        let path = write_entry(tmp.path(), "201", 1000);
        set_age(&path, 3 * 24 * 60 * 60);

        assert_eq!(
            cache.acquire_cached_path(201, Some(1000)),
            Some(path.clone())
        );
        assert!(
            age_secs(&path) < 60,
            "播放取用后 mtime 应刷新为最近使用，实际 {} 秒前",
            age_secs(&path)
        );
    }

    /// CR-005：状态查询（可播性批量检查、`playback_is_cached`）只读，不得推进 mtime。
    #[test]
    fn status_queries_leave_mtime_alone() {
        let tmp = TempDir::new("cache_readonly");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        let path = write_entry(tmp.path(), "202", 1000);
        set_age(&path, 3600);

        assert!(cache.is_cached(202, Some(1000)));
        assert_eq!(
            cache.validate_cached_path(202, Some(1000)),
            Some(path.clone())
        );
        let after = age_secs(&path);
        assert!(
            after >= 3500,
            "只读查询把缓存标成了刚刚使用，实际年龄 {} 秒",
            after
        );
    }

    /// CR-005 的验收点：淘汰保留「近期真实播放过的文件」，浏览列表不能插队。
    ///
    /// 时间线故意排成"最后一次操作是只读查询"：只要查询会 touch，501 就挤到 502 之后，
    /// 被淘汰的就变成了刚播过的 502——正是本条要防的回归。
    #[test]
    fn eviction_follows_real_playback_not_queries() {
        let tmp = TempDir::new("cache_lru");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        let viewed = write_entry(tmp.path(), "501", 100);
        let played = write_entry(tmp.path(), "502", 100);
        set_age(&viewed, 3600);
        set_age(&played, 600);

        // 1) 真实播放 502：只有这一步算"使用"
        assert_eq!(
            cache.acquire_cached_path(502, Some(100)),
            Some(played.clone())
        );
        // 2) 之后用户打开歌曲列表，501 被反复只读查询
        assert!(cache.is_cached(501, Some(100)));
        assert_eq!(
            cache.validate_cached_path(501, Some(100)),
            Some(viewed.clone())
        );

        // 只容得下一首：让位必须是不曾被播放、只被浏览过的 501
        let freed = cache.prune_to_max_bytes(100);
        assert_eq!(freed, 100, "只该淘汰一个 100 字节的缓存");
        assert!(played.exists(), "刚播放过的缓存不能被淘汰");
        assert!(
            !viewed.exists(),
            "只被列表查询扫过的缓存应最先让出空间（查询不该算使用）"
        );
    }

    /// 服务端没上报 file_size 时退化为「非空即有效」；空文件永远不算命中
    #[test]
    fn unknown_expected_size_accepts_nonempty_but_not_empty() {
        let tmp = TempDir::new("cache_unknown");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        write_entry(tmp.path(), "301", 0);
        write_entry(tmp.path(), "302", 10);

        assert!(!cache.is_cached(301, None), "0 字节不是有效缓存");
        assert!(cache.is_cached(302, None));
        assert!(cache.is_cached(302, Some(10)));
        assert!(!cache.is_cached(302, Some(11)));
    }

    /// G-11：下载标记必须是 RAII——提前 return / panic 都要释放，
    /// 否则该 media_file_id 永久停在「正在缓存中」，只能重启进程恢复。
    /// 观测点就是抢占本身：抢不到说明仍被占用，抢得到说明已释放。
    #[test]
    fn download_guard_is_exclusive_and_releases_on_drop() {
        let first = DownloadGuard::try_new(9001).expect("首次抢占应成功");
        assert!(
            DownloadGuard::try_new(9001).is_none(),
            "同一首歌不应出现两个并发下载"
        );

        drop(first);
        let again = DownloadGuard::try_new(9001);
        assert!(again.is_some(), "释放后应能重新下载");
        drop(again);

        // 提前 return 路径：拿到 guard 的函数直接返回错误，不显式释放
        fn acquire_then_bail(media_file_id: i64) -> Result<(), ()> {
            let _guard = DownloadGuard::try_new(media_file_id).unwrap();
            Err(())
        }
        assert!(acquire_then_bail(9002).is_err());
        assert!(
            DownloadGuard::try_new(9002).is_some(),
            "下载函数提前返回后必须能再次下载"
        );
    }

    /// G-11：淘汰按最久未用清理，且绝不下手于写入中的 .tmp
    #[test]
    fn prune_removes_oldest_first_and_skips_tmp() {
        let tmp = TempDir::new("cache_prune");
        let cache = AudioCache::from_cache_dir(tmp.path().clone());
        let oldest = write_entry(tmp.path(), "401", 100);
        set_age(&oldest, 7200);
        let newest = write_entry(tmp.path(), "402", 100);
        set_age(&newest, 60);
        let inflight = write_entry(tmp.path(), "403.tmp", 100);
        set_age(&inflight, 9999);

        // 200 字节有效缓存 + 100 字节 .tmp，上限 150 → 应只删掉最久未用的 401
        let freed = cache.prune_to_max_bytes(150);
        assert_eq!(freed, 100);
        assert!(!oldest.exists(), "最久未用的应被淘汰");
        assert!(newest.exists(), "最近使用的应保留");
        assert!(
            inflight.exists(),
            ".tmp 正在被写入，删掉它只会让下载在 rename 时才失败"
        );
        // .tmp 占着磁盘，应计入容量统计
        assert_eq!(cache.size_bytes(), 200);
    }

    /// 落地前的完整性判定：0 字节与大小不符都必须拒绝（G-10 的入口）
    #[test]
    fn download_size_assertion_covers_empty_and_truncated() {
        assert!(AudioCache::assert_download_size(1000, Some(1000)).is_ok());
        assert!(AudioCache::assert_download_size(1000, None).is_ok());
        assert!(AudioCache::assert_download_size(0, None).is_err());
        assert!(AudioCache::assert_download_size(999, Some(1000)).is_err());
        // 服务端多给了字节（例如把错误页拼在体后）同样视为不符
        assert!(AudioCache::assert_download_size(1001, Some(1000)).is_err());
    }
}
