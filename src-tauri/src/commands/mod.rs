pub mod library;
pub mod playback;
pub mod scanner;
pub mod sync;
pub mod app;

// [MA0 Spike] 技术验证命令，MA1 收尾时移除
pub mod debug;

// Re-export PlaybackState
pub use playback::PlaybackState;

// And we can just have tauri registration function or we can register directly in main
