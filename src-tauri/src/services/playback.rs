use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
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
        Ok(Self { sink })
    }

    pub fn play_file(&self, path: &std::path::Path) -> Result<Option<u64>, String> {
        info!("Playing file: {:?}", path);
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        self.play_stream(file)
    }

    pub fn play_stream<R: std::io::Read + std::io::Seek + Send + Sync + 'static>(&self, reader: R) -> Result<Option<u64>, String> {
        let decoder = Decoder::new(BufReader::new(reader))
            .map_err(|e| format!("Failed to decode stream: {}", e))?;

        use rodio::Source;
        let duration = decoder.total_duration().map(|d| d.as_millis() as u64);

        self.sink.stop(); // 清掉旧队列，避免叠加
        self.sink.append(decoder);
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
        info!("Playback stopped");
        self.sink.stop();
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
}
