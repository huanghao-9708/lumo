pub mod library;
pub mod playback;
pub mod scanner;
pub mod sync;

// Re-export PlaybackState
pub use playback::PlaybackState;

// And we can just have tauri registration function or we can register directly in main
