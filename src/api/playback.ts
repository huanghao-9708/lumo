import { invoke } from '../utils/tauriInvoke';

export function playbackPlay(mediaFileId?: number, forceLocal?: boolean): Promise<number | null> {
  return invoke('playback_play', { mediaFileId, forceLocal });
}

export function playbackEnqueueNext(mediaFileId: number, forceLocal?: boolean): Promise<void> {
  return invoke('playback_enqueue_next', { mediaFileId, forceLocal });
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

/** 读取当前音频能量（RMS，0.0–1.0），驱动沉浸式播放页的封面可视化。
 *  非播放态返回 0；后端逐窗(≈21ms)统计，前端以约 30Hz 采样即可。 */
export function playbackGetLevel(): Promise<number> {
  return invoke('playback_get_level');
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
