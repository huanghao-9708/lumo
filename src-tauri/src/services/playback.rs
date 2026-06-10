use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use tracing::info;

pub struct PlaybackManager {
    sink: Sink,
}

impl PlaybackManager {
    pub fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default().map_err(|e| format!("Failed to get default audio output: {}", e))?;
        
        Box::leak(Box::new(stream));

        let sink = Sink::try_new(&stream_handle).map_err(|e| format!("Failed to create audio sink: {}", e))?;
        
        info!("Initialized default audio output stream");
        Ok(Self {
            sink,
        })
    }

    pub fn play_file(&self, path: &std::path::Path) -> Result<(), String> {
        info!("Playing file: {:?}", path);
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let decoder = Decoder::new(BufReader::new(file)).map_err(|e| format!("Failed to decode file: {}", e))?;
        
        self.sink.stop(); // Clear any existing queue
        self.sink.append(decoder);
        self.sink.play();
        Ok(())
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
}
