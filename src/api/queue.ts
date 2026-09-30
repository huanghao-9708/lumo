import { invoke } from '../utils/tauriInvoke';

export interface QueueItemDTO {
  trackId: number;
  mediaFileId: number;
  title: string;
  artist: string;
  album: string;
  artworkId: number | null;
  durationMs: number | null;
}

export type BackendPlayMode = 'normal' | 'repeatAll' | 'repeatOne' | 'shuffle';

export interface PlaybackQueueStateDTO {
  items: QueueItemDTO[];
  index: number;
  mode: BackendPlayMode;
  positionMs: number;
}

/** 轻量会话摘要（DM-02）：刻意不含 items，迷你栏与轻量恢复消费 */
export interface SessionTrackBriefDTO {
  trackId: number;
  mediaFileId: number;
  title: string;
  artist: string;
  album: string;
  artworkId: number | null;
  durationMs: number | null;
}

export interface PlaybackSessionSummaryDTO {
  stateVersion: number;
  queueLength: number;
  currentIndex: number;
  mode: BackendPlayMode;
  positionMs: number;
  durationMs: number | null;
  isPlaying: boolean;
  currentTrack: SessionTrackBriefDTO | null;
}

export function playbackSessionSummary(): Promise<PlaybackSessionSummaryDTO> {
  return invoke('playback_session_summary');
}

export function playbackSetQueue(items: QueueItemDTO[], index: number, mode: BackendPlayMode): Promise<void> {
  return invoke('playback_set_queue', { items, index, mode });
}

export function playbackQueueState(): Promise<PlaybackQueueStateDTO> {
  return invoke('playback_queue_state');
}

export function playbackAdvance(direction: number): Promise<void> {
  return invoke('playback_advance', { direction });
}

export function playbackSetMode(mode: BackendPlayMode): Promise<void> {
  return invoke('playback_set_mode', { mode });
}

export function playbackPlayIndex(index: number): Promise<void> {
  return invoke('playback_play_index', { index });
}
