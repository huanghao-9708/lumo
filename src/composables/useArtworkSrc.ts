import { ref, watch, type Ref } from 'vue';
import {
  getCachedArtwork,
  cacheArtworkBlob,
  clearArtworkCache,
} from '../utils/artworkCache';
import { getArtworkUrl, getCurrentPlatform } from '../utils';

// ================= 新增：全局并发控制队列 =================
const MAX_CONCURRENT_FETCHES = 3; // 限制后台预加载并发数，留足通道给 IPC
let activeFetches = 0;
const fetchQueue: Array<() => void> = [];
const pendingPrefetchIds = new Set<number>();

function enqueuePrefetch(id: number, task: () => Promise<void>) {
  if (pendingPrefetchIds.has(id)) return;
  pendingPrefetchIds.add(id);

  const execute = async () => {
    try {
      await task();
    } finally {
      pendingPrefetchIds.delete(id);
      activeFetches--;
      dequeuePrefetch();
    }
  };

  fetchQueue.push(execute);
  dequeuePrefetch();
}

function dequeuePrefetch() {
  while (activeFetches < MAX_CONCURRENT_FETCHES && fetchQueue.length > 0) {
    const task = fetchQueue.shift();
    if (task) {
      activeFetches++;
      task();
    }
  }
}
// ==========================================================

/**
 * 响应式的封面图片 src。
 *
 * 用法（在模板里）：
 * ```vue
 * <script setup>
 *   const coverSrc = useArtworkSrc(() => album.cover_artwork_id);
 * </script>
 * <template>
 *   <img v-if="coverSrc" :src="coverSrc" />
 * </template>
 * ```
 *
 * 工作机制：
 * 1. 先查内存 dataURL 缓存，命中直接用（不发任何请求）
 * 2. 未命中时先用原始 `http://lumo.localhost/...` 作为占位（让浏览器去请求一次）
 * 3. 同时用 fetch 异步拉取并转 blob → dataURL 入缓存
 * 4. 下次再访问同一 id，走第 1 步，零请求
 *
 * 配合后端的 ETag + Cache-Control，封面的网络/磁盘开销被压到最低。
 */

export function useArtworkSrc(artworkIdGetter: () => number | null | undefined): Ref<string> {
  const src = ref('');

  async function refresh() {
    const id = artworkIdGetter();
    if (id == null) {
      src.value = '';
      return;
    }

    // 1. 内存缓存命中
    const cached = getCachedArtwork(id);
    if (cached) {
      src.value = cached;
      return;
    }

    // 2. 未命中：先用原始 URL 占位，浏览器会去请求（此时后端的 Cache-Control 生效）
    const url = getArtworkUrl(id);
    src.value = url;

    // 3. 异步预取并写入缓存，下次直接走 dataURL
    //    用 fetch 拉 blob，避免 <img> 已在加载时被重复触发。
    //    只在 Windows（WebView2）下做预取——其他平台的 WKWebView 对自定义 scheme
    //    的 HTTP 缓存已经较好，不需要这层。
    if (getCurrentPlatform() === 'windows') {
      enqueuePrefetch(id, () => prefetchAndCache(id, url).catch(() => {
        // 预取失败不致命，下次访问会重试
      }));
    }
  }

  async function prefetchAndCache(id: number, url: string) {
    const resp = await fetch(url);
    if (!resp.ok) return;
    const blob = await resp.blob();
    // 仅当当前 ref 仍然指向同一个 id 时才更新 src，避免竞态导致旧图覆盖新图
    if (artworkIdGetter() === id) {
      const dataUrl = await cacheArtworkBlob(id, blob);
      src.value = dataUrl;
    } else {
      // 已切到别的封面，但仍把这张缓存起来供下次用
      await cacheArtworkBlob(id, blob);
    }
  }

  watch(artworkIdGetter, refresh, { immediate: true });

  return src;
}

/** 清空前端内存缓存（与后端清缓存联动时调用） */
export function resetArtworkFrontCache() {
  clearArtworkCache();
}
