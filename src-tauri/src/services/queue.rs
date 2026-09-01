use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlaySource {
    Local(PathBuf),
    WebDav {
        url: String,
        headers: HashMap<String, String>,
        cached_path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub track_id: i64,
    pub media_file_id: i64,
    #[serde(skip)]
    pub play_path: Option<PlaySource>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork_id: Option<i64>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayMode {
    Normal,
    RepeatAll,
    RepeatOne,
    Shuffle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueStateDto {
    pub items: Vec<QueueItem>,
    pub index: usize,
    pub mode: PlayMode,
    pub position_ms: u64,
}

pub struct PlaybackQueue {
    pub items: Vec<QueueItem>,
    pub index: usize,
    pub mode: PlayMode,
    pub shuffle_order: Option<Vec<usize>>,
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
            mode: PlayMode::Normal,
            shuffle_order: None,
        }
    }

    pub fn set_queue(&mut self, items: Vec<QueueItem>, index: usize, mode: PlayMode) {
        self.items = items;
        self.index = if self.items.is_empty() { 0 } else { index.min(self.items.len() - 1) };
        self.set_mode(mode);
    }

    pub fn set_mode(&mut self, mode: PlayMode) {
        self.mode = mode;
        if mode == PlayMode::Shuffle {
            if self.items.is_empty() {
                self.shuffle_order = Some(Vec::new());
            } else {
                let mut order = generate_shuffle_order(self.items.len());
                if let Some(pos) = order.iter().position(|&i| i == self.index) {
                    order.swap(0, pos);
                }
                self.shuffle_order = Some(order);
            }
        } else {
            self.shuffle_order = None;
        }
    }

    pub fn next_index(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        match self.mode {
            PlayMode::Normal => {
                if self.index + 1 < self.items.len() {
                    Some(self.index + 1)
                } else {
                    None
                }
            }
            PlayMode::RepeatAll => Some((self.index + 1) % self.items.len()),
            PlayMode::RepeatOne => Some(self.index),
            PlayMode::Shuffle => {
                if let Some(order) = &self.shuffle_order {
                    if order.is_empty() {
                        return None;
                    }
                    let current_pos = order.iter().position(|&i| i == self.index).unwrap_or(0);
                    Some(order[(current_pos + 1) % order.len()])
                } else {
                    Some((self.index + 1) % self.items.len())
                }
            }
        }
    }

    pub fn prev_index(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        match self.mode {
            PlayMode::Normal => {
                if self.index > 0 {
                    Some(self.index - 1)
                } else {
                    Some(0)
                }
            }
            PlayMode::RepeatAll => {
                if self.index > 0 {
                    Some(self.index - 1)
                } else {
                    Some(self.items.len() - 1)
                }
            }
            PlayMode::RepeatOne => Some(self.index),
            PlayMode::Shuffle => {
                if let Some(order) = &self.shuffle_order {
                    if order.is_empty() {
                        return None;
                    }
                    let current_pos = order.iter().position(|&i| i == self.index).unwrap_or(0);
                    if current_pos > 0 {
                        Some(order[current_pos - 1])
                    } else {
                        Some(order[order.len() - 1])
                    }
                } else {
                    if self.index > 0 {
                        Some(self.index - 1)
                    } else {
                        Some(self.items.len() - 1)
                    }
                }
            }
        }
    }

    pub fn advance(&mut self, direction: i32) -> Option<usize> {
        let target = if direction > 0 {
            self.next_index()
        } else {
            self.prev_index()
        };
        if let Some(idx) = target {
            self.index = idx;
        }
        target
    }
}

pub fn generate_shuffle_order(len: usize) -> Vec<usize> {
    use rand::seq::SliceRandom;
    let mut order: Vec<usize> = (0..len).collect();
    let mut rng = rand::rng();
    order.shuffle(&mut rng);
    order
}

pub struct QueueState {
    pub queue: Mutex<PlaybackQueue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedPlaybackState {
    pub items: Vec<QueueItem>,
    pub index: usize,
    pub mode: PlayMode,
    pub position_ms: u64,
}

pub fn save_state_to_disk(app_dir: &std::path::Path, q: &PlaybackQueue, position_ms: u64) {
    let state = PersistedPlaybackState {
        items: q.items.clone(),
        index: q.index,
        mode: q.mode,
        position_ms,
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let file_path = app_dir.join("playback_state.json");
        let _ = std::fs::write(file_path, json);
    }
}

pub fn load_state_from_disk(app_dir: &std::path::Path) -> Option<PersistedPlaybackState> {
    let file_path = app_dir.join("playback_state.json");
    let content = std::fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_items(count: usize) -> Vec<QueueItem> {
        (0..count)
            .map(|i| QueueItem {
                track_id: i as i64,
                media_file_id: (i + 100) as i64,
                play_path: None,
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                artwork_id: None,
                duration_ms: Some(180_000),
            })
            .collect()
    }

    #[test]
    fn test_normal_mode_boundary() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 0, PlayMode::Normal);

        assert_eq!(q.next_index(), Some(1));
        q.advance(1);
        assert_eq!(q.index, 1);

        assert_eq!(q.next_index(), Some(2));
        q.advance(1);
        assert_eq!(q.index, 2);

        // At end of queue in Normal mode, next_index is None
        assert_eq!(q.next_index(), None);

        // Prev at index 0 stays at 0
        q.advance(-1);
        assert_eq!(q.index, 1);
        q.advance(-1);
        assert_eq!(q.index, 0);
        assert_eq!(q.prev_index(), Some(0));
    }

    #[test]
    fn test_repeat_all_mode() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 2, PlayMode::RepeatAll);

        // 2 -> wraps around to 0
        assert_eq!(q.next_index(), Some(0));
        q.advance(1);
        assert_eq!(q.index, 0);

        // 0 -> wraps back to 2
        assert_eq!(q.prev_index(), Some(2));
        q.advance(-1);
        assert_eq!(q.index, 2);
    }

    #[test]
    fn test_repeat_one_mode() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 1, PlayMode::RepeatOne);

        assert_eq!(q.next_index(), Some(1));
        assert_eq!(q.prev_index(), Some(1));
    }

    #[test]
    fn test_shuffle_mode_order() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(5), 2, PlayMode::Shuffle);

        // Current track (2) must be the first element in shuffle order
        let order = q.shuffle_order.clone().unwrap();
        assert_eq!(order.len(), 5);
        assert_eq!(order[0], 2);

        // Next index must be order[1]
        let next = q.next_index().unwrap();
        assert_eq!(next, order[1]);

        q.advance(1);
        assert_eq!(q.index, order[1]);

        // Prev index must be order[0] (which is 2)
        assert_eq!(q.prev_index().unwrap(), order[0]);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let temp_dir = std::env::temp_dir().join("lumo_test_persistence");
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(4), 1, PlayMode::RepeatAll);

        save_state_to_disk(&temp_dir, &q, 45_000);
        let loaded = load_state_from_disk(&temp_dir).expect("Failed to load saved state");

        assert_eq!(loaded.index, 1);
        assert_eq!(loaded.mode, PlayMode::RepeatAll);
        assert_eq!(loaded.position_ms, 45_000);
        assert_eq!(loaded.items.len(), 4);
        assert_eq!(loaded.items[1].title, "Track 1");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
