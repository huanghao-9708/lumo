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

/** 查询某首歌是否已缓存到本地（前端离线置灰判断用） */
export function playbackIsCached(mediaFileId: number): Promise<boolean> {
  return invoke('playback_is_cached', { mediaFileId });
}

/** 清空音频缓存，返回释放的字节数 */
export function playbackClearAudioCache(): Promise<number> {
  return invoke('playback_clear_audio_cache');
}

/** 获取音频缓存总大小（字节），设置页显示用 */
export function playbackGetAudioCacheSize(): Promise<number> {
  return invoke('playback_get_audio_cache_size');
}
