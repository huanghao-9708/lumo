use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::info;

/// 播放速率下限/上限（前端只提供 0.5–1.5 五档，这里留一点余量做防御）
const SPEED_MIN: f32 = 0.25;
const SPEED_MAX: f32 = 2.0;

/// rodio 的 `Sink::get_pos()` 返回的是**播放时间轴**，不是曲目内容位置：
/// 速率 1.5x 时它 1 秒才走 1 秒，而曲目内容已经走了 1.5 秒
/// （rodio 把 Speed 的 sample_rate 乘上了 factor，位置按该采样率折算）。
/// 所以内容位置必须自己累：`content += Δwall × speed`。
///
/// 为什么不能简单地 `content = wall × speed`：用户中途改速率时，
/// 之前那段已经按旧速率播过，直接乘会让进度条整体跳一下。
#[derive(Debug, Default)]
struct PositionTracker {
    /// 曲目内容位置（ms）
    content_ms: f64,
    /// 上次同步时的播放时间轴位置（ms）
    wall_ms: f64,
}

impl PositionTracker {
    /// 把时间轴推进到 `wall_now_ms`，按 `speed` 折算进内容位置。
    fn sync(&mut self, wall_now_ms: f64, speed: f64) {
        if wall_now_ms < self.wall_ms {
            // 时间轴回退 = 换了新音源（gapless 无缝切歌）或 seek 后 rodio 重算：
            // 视为新一段，内容位置直接跟时间轴对齐，别把上一首的时长算进来。
            self.content_ms = wall_now_ms * speed;
        } else {
            self.content_ms += (wall_now_ms - self.wall_ms) * speed;
        }
        self.wall_ms = wall_now_ms;
    }

    /// 对齐到指定内容位置（seek 成功后调用，避免进度条假跳）
    fn align(&mut self, content_ms: f64, wall_ms: f64) {
        self.content_ms = content_ms;
        self.wall_ms = wall_ms;
    }

    fn reset(&mut self) {
        self.content_ms = 0.0;
        self.wall_ms = 0.0;
    }
}

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
///
///   1. 每个进程只此一份，量级固定（一个输出设备句柄），不是持续增长的泄漏；
///   2. 进程退出时操作系统会自动回收所有资源；
///   3. cpal/cpal 内部对 stream 也并未提供安全的显式释放 API。
///
/// 因此这里保留 `Box::leak` 模式，并显式记录此设计权衡。
pub struct PlaybackManager {
    sink: Sink,
    /// 队列播放生命周期是否仍处于活动状态。
    ///
    /// `Sink::empty()` 只能说明当前没有音频，无法区分「自然播完」「用户主动停止」和
    /// 「应用刚启动尚未播放」。队列观察器需要这个状态，才能只消费一次自然结束事件。
    active: AtomicBool,
    /// 播放速率。写进 rodio 的 `Sink::set_speed`——音频线程每 5ms 读一次该值，
    /// 所以改速率立即生效、不需要重建音源（变速=重采样，会有音高变化，这是无时间拉伸依赖下的取舍）。
    speed: Mutex<f32>,
    /// 播放时间轴 → 曲目内容位置的换算（见 `PositionTracker`）
    position: Mutex<PositionTracker>,
}

impl PlaybackManager {
    pub fn new() -> Result<Self, String> {
        let stream = OutputStreamBuilder::open_default_stream()
            .map_err(|e| format!("Failed to get default audio output: {}", e))?;
        let sink = Sink::connect_new(stream.mixer());
        // 详见类型注释：Sink 只持有 mixer 句柄，stream 一旦 drop 就没有输出，
        // 所以把 stream 钉死在堆上保活。
        Box::leak(Box::new(stream));

        info!("Initialized default audio output stream");
        Ok(Self {
            sink,
            active: AtomicBool::new(false),
            speed: Mutex::new(1.0),
            position: Mutex::new(PositionTracker::default()),
        })
    }

    /// 播放速率（1.0 为原速）
    pub fn get_speed(&self) -> f32 {
        *self.speed.lock().unwrap()
    }

    /// 设置播放速率。立即生效（rodio 音频线程每 5ms 取一次该值）。
    pub fn set_speed(&self, speed: f32) {
        if !speed.is_finite() {
            return;
        }
        let speed = speed.clamp(SPEED_MIN, SPEED_MAX);
        // 先把「改速率之前」的进度按旧速率折算掉，否则进度条会跳
        let previous = self.get_speed();
        let wall_ms = self.sink.get_pos().as_millis() as f64;
        if let Ok(mut tracker) = self.position.lock() {
            tracker.sync(wall_ms, previous as f64);
        }
        *self.speed.lock().unwrap() = speed;
        self.sink.set_speed(speed);
        info!("Playback speed set to {:.2}x", speed);
    }

    /// 构建解码器（**耗时操作，必须在 manager 锁外调用**）。
    ///
    /// **必须带上 `byte_len`**：symphonia 的 FLAC / MP3 / MP4 解复用器在 seek 时要先知道
    /// 流的总字节数（`bundle-flac/demuxer.rs`、`bundle-mp3/demuxer.rs` 里都是
    /// `reader.byte_len().ok_or(SeekError::Unseekable)`），拿不到就直接报 Unseekable。
    /// rodio 0.19 的 `ReadSeekSource::byte_len()` 恒为 `None`，所以那些格式根本拖不动
    /// 进度条；0.21 支持通过 `with_byte_len` 显式提供（顺带会把 is_seekable 置真）。
    ///
    /// 远端流的探测/解码可能耗时数十秒 —— 这正是它必须在锁外的原因：
    /// 否则 seek / 进度查询全都要排在它后面。
    pub fn build_decoder<R>(reader: R, byte_len: Option<u64>) -> Result<Decoder<R>, String>
    where
        R: std::io::Read + std::io::Seek + Send + Sync + 'static,
    {
        let mut builder = Decoder::builder().with_data(reader);
        if let Some(len) = byte_len {
            builder = builder.with_byte_len(len);
        }
        let decoder = builder
            .build()
            .map_err(|e| format!("Failed to decode stream: {}", e))?;
        Ok(decoder)
    }

    /// 把已构建好的解码器接上 Sink 开始播放。
    ///
    /// **在 manager 锁内调用**，只做微秒级的换源操作（stop/reset/append/play），
    /// 任何耗时的准备工作都应在 `build_decoder`（锁外）完成。
    pub fn play_prepared<R: std::io::Read + std::io::Seek + Send + Sync + 'static>(
        &self,
        decoder: Decoder<R>,
    ) -> Result<(), String> {
        self.sink.stop(); // 清掉旧队列，避免叠加
        if let Ok(mut tracker) = self.position.lock() {
            tracker.reset();
        }
        // 速率设置要重新下发：rodio 的 speed 是挂在「被 append 的音源」上的
        self.sink.set_speed(self.get_speed());
        self.sink.append(decoder);
        self.sink.play();
        self.active.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// gapless：把已构建好的解码器排到队尾（不 stop，**锁内微秒级**）。
    pub fn enqueue_prepared<R: std::io::Read + std::io::Seek + Send + Sync + 'static>(
        &self,
        decoder: Decoder<R>,
    ) -> Result<(), String> {
        self.sink.append(decoder);
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
        // stop 是幂等操作：队尾处理完毕或平台重复下发停止命令时，不再重复操作和刷日志。
        let was_active = self.active.swap(false, Ordering::Relaxed);
        if !was_active && self.sink.empty() {
            return;
        }
        info!("Playback stopped");
        self.sink.stop();
        if let Ok(mut tracker) = self.position.lock() {
            tracker.reset();
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }

    /// 当前曲目内容位置（ms）。
    ///
    /// 注意不是直接返回 `Sink::get_pos()`：那个是「播放时间轴」，变速后两者不一致
    /// （详见 `PositionTracker`）。
    pub fn get_pos(&self) -> u64 {
        let speed = self.get_speed() as f64;
        let wall_ms = self.sink.get_pos().as_millis() as f64;
        let Ok(mut tracker) = self.position.lock() else {
            return wall_ms as u64;
        };
        tracker.sync(wall_ms, speed);
        tracker.content_ms.max(0.0) as u64
    }

    /// 跳转到曲目内的 `position_ms`（内容位置）。
    pub fn try_seek(&self, position_ms: u64) -> Result<(), String> {
        let speed = self.get_speed().max(0.01) as f64;
        // rodio 的 seek 接受的是「播放时间轴」上的位置，需要按速率换算回内容位置
        let wall_ms = position_ms as f64 / speed;
        self.sink
            .try_seek(std::time::Duration::from_millis(wall_ms as u64))
            .map_err(|e| format!("Failed to seek: {:?}", e))?;
        // 成功后再对齐累计器；失败时不动，避免进度条假跳
        if let Ok(mut tracker) = self.position.lock() {
            tracker.align(position_ms as f64, wall_ms);
        }
        Ok(())
    }

    /// 当前是否已播放完毕（解码队列为空）。前端在时长未知时也能据此自动切下一首。
    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    /// 是否存在一段尚未被消费掉结束事件的队列播放生命周期。
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
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
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        1
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f32(
            self.total as f32 / self.sample_rate as f32,
        ))
    }
}
