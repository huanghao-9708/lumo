import { invoke } from '../utils/tauriInvoke';

export function playbackPlay(mediaFileId?: number): Promise<number | null> {
  return invoke('playback_play', { mediaFileId });
}

export function playbackEnqueueNext(mediaFileId: number): Promise<void> {
  return invoke('playback_enqueue_next', { mediaFileId });
}

export function playbackGetQueueLen(): Promise<number> {
  return invoke('playback_get_queue_len');
}

export function playbackPause(): Promise<void> {
  return invoke('playback_pause');
}

export function playbackResume(): Promise<void> {
  return invoke('playback_resume');
}

export function playbackStop(): Promise<void> {
  return invoke('playback_stop');
}

export function playbackSetVolume(volume: number): Promise<void> {
  return invoke('playback_set_volume', { volume });
}

export function playbackGetPos(): Promise<number> {
  return invoke('playback_get_pos');
}

export function playbackSeek(positionMs: number): Promise<void> {
  return invoke('playback_seek', { positionMs });
}

export function playbackIsFinished(): Promise<boolean> {
  return invoke('playback_is_finished');
}
