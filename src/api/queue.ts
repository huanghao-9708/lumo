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
