use crate::services::backoff::AdvanceRetry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// 自动切歌失败的退避参数：第 1 次失败等 1 秒，翻倍递增，封顶 15 秒。
pub const ADVANCE_RETRY_BASE: Duration = Duration::from_secs(1);
pub const ADVANCE_RETRY_MAX: Duration = Duration::from_secs(15);

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
    /// 自动切歌的失败重试状态（CR-002）。放在队列里而不是观察者循环的局部变量里，
    /// 是因为「用户主动切歌 / 换队列」发生在命令侧，必须能就地作废旧的退避。
    pub retry: AdvanceRetry,
}

impl Default for PlaybackQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
            mode: PlayMode::Normal,
            shuffle_order: None,
            retry: AdvanceRetry::new(ADVANCE_RETRY_BASE, ADVANCE_RETRY_MAX),
        }
    }

    pub fn set_queue(&mut self, items: Vec<QueueItem>, index: usize, mode: PlayMode) {
        self.items = items;
        self.index = if self.items.is_empty() {
            0
        } else {
            index.min(self.items.len() - 1)
        };
        // 换了队列就等于换了语境：上一份队列攒下的退避惩罚不该带过来
        self.retry.clear();
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
                } else if self.index > 0 {
                    Some(self.index - 1)
                } else {
                    Some(self.items.len() - 1)
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
            // 用户自己按了上一首/下一首：正在进行的自动重试目标已被取代，连退避一起作废
            self.retry.clear();
        }
        target
    }

    /// 播放器确实接下这一首之后才写当前位置（CR-002）。
    ///
    /// 反过来（先写位置再交给播放器）会让一次失败看起来像「这首歌已经播过了」：
    /// 观察者于是从失败曲目的下一首开始数，重试就变成了跳过。
    pub fn commit_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.index = index;
        }
    }

    /// 自动切歌这一跳该播哪一首：优先原样重试刚失败的那一首，它播成功了才继续向前推进。
    pub fn retry_or_next_index(&mut self) -> Option<usize> {
        match self.retry.pending() {
            Some(index) if index < self.items.len() => Some(index),
            // 队列在失败后变短了：待重试的目标已经不存在，作废并回到正常推进
            Some(_) => {
                self.retry.clear();
                self.next_index()
            }
            None => self.next_index(),
        }
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

    /// CR-002 的验收场景：「第二首失败、第三首可用」时第二首不能被跳过。
    /// 观察者循环本身要 AppHandle 与真实播放器，测不到；这里钉住的是它依赖的全部决策。
    #[test]
    fn failed_track_is_retried_before_advancing() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 0, PlayMode::Normal);
        let now = std::time::Instant::now();

        // A 播完，第一次落到 B
        assert_eq!(q.retry_or_next_index(), Some(1));
        q.retry.record_failure(1, "1:101:network", now);
        // 退避期间再跳两跳：目标始终是 B，不会悄悄滑到 C
        assert_eq!(q.retry_or_next_index(), Some(1));
        assert_eq!(q.retry_or_next_index(), Some(1));
        assert_eq!(q.index, 0, "B 没播起来之前，当前位置还应该是 A");

        // B 播成功：位置写定、退避作废，这时才轮到 C
        q.commit_index(1);
        q.retry.clear();
        assert_eq!(q.index, 1);
        assert_eq!(q.retry_or_next_index(), Some(2));
    }

    #[test]
    fn commit_index_only_moves_within_the_queue() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(2), 0, PlayMode::Normal);
        q.commit_index(1);
        assert_eq!(q.index, 1);
        q.commit_index(9);
        assert_eq!(q.index, 1, "越界位置不得把队列指到不存在的曲目上");
    }

    /// 用户操作必须立刻打断旧的自动重试，否则「按下一首」会被上一次故障的目标劫持。
    #[test]
    fn user_skip_voids_the_pending_retry() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 0, PlayMode::Normal);
        let now = std::time::Instant::now();
        q.retry.record_failure(1, "1:101:network", now);
        assert_eq!(q.retry_or_next_index(), Some(1));

        q.advance(1);
        assert_eq!(q.retry.pending(), None);
        assert!(!q.retry.waiting(now), "用户已经换歌，旧的退避不该继续生效");
        assert_eq!(q.index, 1);
    }

    #[test]
    fn new_queue_voids_the_pending_retry() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(3), 0, PlayMode::Normal);
        q.retry
            .record_failure(2, "2:102:network", std::time::Instant::now());
        q.set_queue(mock_items(2), 0, PlayMode::Normal);
        assert_eq!(q.retry.pending(), None);
        assert_eq!(q.retry.failures(), 0);
    }

    /// 失败之后队列被改短（那首歌被删掉了）：越界的待重试目标要退回正常推进，不能 panic。
    #[test]
    fn stale_pending_target_falls_back_to_normal_advance() {
        let mut q = PlaybackQueue::new();
        q.set_queue(mock_items(4), 0, PlayMode::Normal);
        q.retry
            .record_failure(3, "3:103:network", std::time::Instant::now());
        assert_eq!(q.retry_or_next_index(), Some(3));
        q.items.truncate(2);
        assert_eq!(q.retry_or_next_index(), Some(1));
        assert_eq!(q.retry.pending(), None);
    }
}
