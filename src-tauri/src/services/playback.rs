use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::info;

/// 集中管理音频输出流与播放 Sink。
///
/// 关于 `OutputStream` 的存活期：
/// rodio 的 `Sink` 本身不持有 `OutputStream` 的引用，如果 stream 比 sink 先 drop，
/// 声音会立刻停止或 panic。因此 stream 必须活得不短于整个应用进程。
///
/// 另一个约束是：`OutputStream` 内部含 `*mut ()`（指向 cpal 的非 Send 资源），
/// 因此它不是 `Send`。而 `PlaybackManager` 要被 `app.manage()` 注册到 Tauri 全局状态，
/// Tauri 要求被管理的状态满足 `Send + Sync`。这两者直接冲突。
///
/// 业界对 rodio + Tauri 的通用解法是 `Box::leak(Box::new(stream))`：把 stream
/// 钉死在堆上活到进程结束，让 `PlaybackManager` 自身满足 `Send + Sync`。
/// 这块内存确实不会显式释放，但：
///   1. 每个进程只此一份，量级固定（一个输出设备句柄），不是持续增长的泄漏；
///   2. 进程退出时操作系统会自动回收所有资源；
///   3. cpal/cpal 内部对 stream 也并未提供安全的显式释放 API。
/// 因此这里保留 `Box::leak` 模式，并显式记录此设计权衡。
pub struct PlaybackManager {
    sink: Sink,
    /// 当前音频能量（f32 的位模式），由 `LevelSource` 在音频线程逐窗写入。
    /// 用原子量而非 Mutex：音频回调路径上绝不能阻塞。
    level: Arc<AtomicU32>,
}

impl PlaybackManager {
    pub fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to get default audio output: {}", e))?;
        // 详见类型注释：stream 被钉死在堆上保活，避免 Sink 悬空
        Box::leak(Box::new(stream));

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        info!("Initialized default audio output stream");
        Ok(Self {
            sink,
            level: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn play_file(&self, path: &std::path::Path) -> Result<Option<u64>, String> {
        info!("Playing file: {:?}", path);
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        self.play_stream(file)
    }

    pub fn play_stream<R: std::io::Read + std::io::Seek + Send + Sync + 'static>(&self, reader: R) -> Result<Option<u64>, String> {
        let decoder = Decoder::new(BufReader::new(reader))
            .map_err(|e| format!("Failed to decode stream: {}", e))?;

        let duration = decoder.total_duration().map(|d| d.as_millis() as u64);

        self.sink.stop(); // 清掉旧队列，避免叠加
        // convert_samples 把解码器的 i16 样本统一成 f32（LevelSource 按 f32 度量能量）；
        // 该转换器由 rodio 实现，会把 try_seek 原样透传给解码器，因此拖拽进度条语义不变。
        self.sink.append(LevelSource::new(
            decoder.convert_samples::<f32>(),
            self.level.clone(),
        ));
        self.sink.play();
        Ok(duration)
    }

    /// [Gapless Playback] 将下一首曲目直接加入到当前播放队列的末尾。
    ///
    /// 与 `play_file` 不同，此方法不会调用 `self.sink.stop()`。
    /// rodio 的 Sink 会在当前曲目播放完毕后，立刻无缝开始播放这首曲目。
    pub fn enqueue_next_file(&self, path: &std::path::Path) -> Result<(), String> {
        info!("Enqueuing next file for gapless playback: {:?}", path);
        let file = File::open(path).map_err(|e| format!("Failed to open file for enqueuing: {}", e))?;
        let decoder = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Failed to decode stream for enqueuing: {}", e))?;
        self.sink
            .append(LevelSource::new(decoder.convert_samples::<f32>(), self.level.clone()));
        Ok(())
    }

    /// [Gapless Playback] 流式版本：将 WebDAV 等远程流的下一首曲目加入队列末尾。
    ///
    /// 与 `enqueue_next_file` 对称，区别是数据源是任意 `Read+Seek` 流而非本地文件。
    /// 缓存命中时走 `enqueue_next_file`，未命中走此方法，两种情况都实现无缝切歌。
    pub fn enqueue_next_stream<R: std::io::Read + std::io::Seek + Send + Sync + 'static>(&self, reader: R) -> Result<(), String> {
        info!("Enqueuing next stream for gapless playback");
        let decoder = Decoder::new(BufReader::new(reader))
            .map_err(|e| format!("Failed to decode stream for enqueuing: {}", e))?;
        self.sink
            .append(LevelSource::new(decoder.convert_samples::<f32>(), self.level.clone()));
        Ok(())
    }

    /// 获取当前音频队列中剩余的曲目数。
    /// 
    /// 前端可利用此接口轮询。当队列长度从 2 变为 1 时，意味着已经无缝切入了下一首歌。
    pub fn get_queue_len(&self) -> usize {
        self.sink.len()
    }

    pub fn pause(&self) {
        info!("Playback paused");
        self.sink.pause();
    }

    pub fn resume(&self) {
        info!("Playback resumed");
        self.sink.play();
    }

    pub fn stop(&self) {
        info!("Playback stopped");
        self.sink.stop();
        self.level.store(0f32.to_bits(), Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    pub fn get_pos(&self) -> u64 {
        self.sink.get_pos().as_millis() as u64
    }

    pub fn try_seek(&self, position_ms: u64) -> Result<(), String> {
        self.sink.try_seek(std::time::Duration::from_millis(position_ms))
            .map_err(|e| format!("Failed to seek: {:?}", e))
    }

    /// 当前是否已播放完毕（解码队列为空）。前端在时长未知时也能据此自动切下一首。
    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    /// 当前音频能量（RMS，0.0–1.0）。未播放或静音段为 0。
    /// 前端在沉浸式页以约 30Hz 采样，驱动封面「随音乐呼吸」。
    pub fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// [MA0 Spike] 播放正弦测试音，验证移动端音频输出链路。
    /// 刻意与正式播放共用 PlaybackManager 的初始化与 Sink 路径，
    /// 使 Spike 的结论（能否出声、采样率是否正确）可直接迁移到正式链路。
    /// 仅 debug 构建编译；MA1 收尾时随 debug 命令一并移除。
    #[cfg(debug_assertions)]
    pub fn play_tone(&self, freq: f32, seconds: u32) -> Result<Option<u64>, String> {
        let sample_rate = 48_000u32;
        let total = sample_rate as usize * seconds as usize;
        self.sink.stop();
        self.sink.append(DebugTone {
            freq,
            sample_rate,
            sample_idx: 0,
            total,
        });
        self.sink.play();
        info!("Playing debug tone: {}Hz for {}s", freq, seconds);
        Ok(Some(seconds as u64 * 1000))
    }
}

/// 在解码器外层包一层，逐窗统计 RMS 能量并写入共享原子量（音频可视化用）。
///
/// 设计约束：
/// - 只在音频线程做「乘加 + 每窗一次原子写」，不加锁、不分配，对播放链路零感知开销；
/// - **必须转发 `try_seek`**：rodio 的 `Sink::try_seek` 会调用当前 Source 的 `try_seek`，
///   不转发就会静默退化为 `NotSupported`，导致进度条拖拽失效；
/// - 声道 / 采样率 / 时长 / 帧长度全部原样透传，保持 Sink 行为不变。
struct LevelSource<S> {
    inner: S,
    level: Arc<AtomicU32>,
    acc: f32,
    count: u32,
}

/// 统计窗口 1024 个样本：48kHz 下约 21ms，足够驱动 30fps 视觉反馈，
/// 同时把原子写频率压到约 47 次/秒。
const LEVEL_WINDOW: u32 = 1024;

impl<S> LevelSource<S> {
    fn new(inner: S, level: Arc<AtomicU32>) -> Self {
        Self {
            inner,
            level,
            acc: 0.0,
            count: 0,
        }
    }
}

impl<S> Iterator for LevelSource<S>
where
    S: Iterator<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.acc += sample * sample;
        self.count += 1;

        if self.count >= LEVEL_WINDOW {
            let rms = (self.acc / self.count as f32).sqrt();
            self.level
                .store(rms.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
            self.acc = 0.0;
            self.count = 0;
        }

        Some(sample)
    }
}

impl<S> rodio::Source for LevelSource<S>
where
    S: rodio::Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }

    /// 关键：转发 seek，否则进度条拖拽会失效（见类型注释）。
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

/// [MA0 Spike] 正弦波测试音源（仅 debug 构建），避免依赖 rodio::source::SineWave 的 API 漂移。
#[cfg(debug_assertions)]
struct DebugTone {
    freq: f32,
    sample_rate: u32,
    sample_idx: usize,
    total: usize,
}

#[cfg(debug_assertions)]
impl Iterator for DebugTone {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.sample_idx >= self.total {
            return None;
        }
        let t = self.sample_idx as f32 / self.sample_rate as f32;
        self.sample_idx += 1;
        Some((t * self.freq * std::f32::consts::TAU).sin() * 0.35)
    }
}

#[cfg(debug_assertions)]
impl rodio::Source for DebugTone {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f32(
            self.total as f32 / self.sample_rate as f32,
        ))
    }
}
