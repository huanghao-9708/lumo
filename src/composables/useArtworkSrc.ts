import { computed, type Ref } from 'vue';
import { clearArtworkCache } from '../utils/artworkCache';
import { getArtworkUrl } from '../utils';

/**
 * 单一封面通道：小图请求缩略图 URL，由浏览器缓存和后端尺寸变体处理复用。
 * 不再为同一个 <img> 额外 fetch/转 Base64，避免两份图片解码和异步任务保留组件。
 */
export function useArtworkSrc(
  artworkIdGetter: () => number | null | undefined,
  size: 'thumb' | 'full' = 'thumb',
): Ref<string> {
  return computed(() => {
    const id = artworkIdGetter();
    return id == null ? '' : getArtworkUrl(id, size);
  });
}

/** 与设置中的封面缓存清理保持兼容。 */
export function resetArtworkFrontCache() {
  clearArtworkCache();
}

/** 保留旧诊断接口；当前无 JS 预取队列，浏览器协议请求不计入这些数字。 */
export function getArtworkFetchStats() {
  return { activeFetches: 0, queued: 0, pendingIds: 0 };
}

if (import.meta.env.DEV) {
  void import('../utils/memDiagnostics').then(({ registerMemSampler }) => {
    registerMemSampler('artwork', () => ({
      artwork_source_mode: 'protocol-url',
      artwork_fetch_active: 0,
      artwork_fetch_queued: 0,
    }));
  });
}
