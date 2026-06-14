/**
 * 封面图片内存缓存（LRU）。
 *
 * 为什么需要它（在后端已经加了 HTTP 缓存的前提下还做一层）：
 *
 * 1. WebView2 对自定义 scheme（`http://lumo.localhost/`）的 HTTP 缓存行为不完全
 *    可靠，且虚拟列表在滚动时会反复创建/销毁 `<img>` 元素，每次 patch 都可能
 *    让浏览器重新走一遍请求（即便返回 304 也有 IPC + 协议线程往返开销）。
 *
 * 2. 把第一次加载到的封面转成 dataURL 缓存在内存里，后续命中直接作为 `src`
 *    赋值，浏览器完全不会再发网络请求——这是消除"滚动时图片导致卡顿"的
 *    最后一道保险。
 *
 * 内存代价：dataURL 会常驻主进程内存（base64 比原文件大约 1.33 倍）。
 * 因此限制缓存条数并按 LRU 淘汰。默认上限 5000 张，按平均封面 50KB×1.33 估算
 * 约 330MB——对桌面应用可接受，且能覆盖几千张专辑的滚动历史，
 * 避免回滚时重新请求后端。
 */

const MAX_ENTRIES = 5000;

interface CacheEntry {
  dataUrl: string;
  // 访问时间戳，用于 LRU 排序。用 monotonically increasing 的计数器而非真实时钟，
  // 避免系统时间回拨导致的判定异常。
  lastUsed: number;
}

const cache = new Map<number, CacheEntry>();
let accessCounter = 0;

/** 当前是否已开启缓存。可通过 `setArtworkCacheEnabled(false)` 临时关闭（例如清缓存时） */
let enabled = true;

/** 命中缓存时返回 dataURL，否则返回 null。访问即"使用"，会更新 LRU 顺序。 */
export function getCachedArtwork(artworkId: number | null | undefined): string | null {
  if (!enabled || artworkId == null) return null;
  const entry = cache.get(artworkId);
  if (!entry) return null;
  entry.lastUsed = ++accessCounter;
  return entry.dataUrl;
}

/**
 * 把一个 artwork 对应的原始 blob 数据写入缓存。
 * 内部会转成 dataURL 存储。
 */
export async function cacheArtworkBlob(artworkId: number, blob: Blob): Promise<string> {
  const dataUrl = await blobToDataURL(blob);
  if (enabled) {
    cache.set(artworkId, { dataUrl, lastUsed: ++accessCounter });
    evictIfNeeded();
  }
  return dataUrl;
}

/** 直接以 dataURL 形式写入缓存（已知 dataURL 时用，省一次转换） */
export function cacheArtworkDataUrl(artworkId: number, dataUrl: string): void {
  if (!enabled) return;
  cache.set(artworkId, { dataUrl, lastUsed: ++accessCounter });
  evictIfNeeded();
}

/** 清空所有缓存（例如用户在设置里点了"清空封面缓存"） */
export function clearArtworkCache(): void {
  cache.clear();
}

/** 临时启用/禁用缓存 */
export function setArtworkCacheEnabled(value: boolean): void {
  enabled = value;
  if (!value) cache.clear();
}

/**
 * 判断某个 artworkId 是否已在缓存中（不更新 LRU 顺序）。
 * 预加载器用它来跳过已缓存的项，避免重复 fetch。
 */
export function isArtworkCached(artworkId: number | null | undefined): boolean {
  if (!enabled || artworkId == null) return false;
  return cache.has(artworkId);
}

/**
 * 预热接口：批量拉取一组封面并写入缓存。
 *
 * 设计要点：
 * - 已在缓存里的直接跳过，不发请求
 * - 用固定并发数（默认 6）限制同时发出的 fetch，避免一次性几十个请求
 *   把 lumo:// 协议线程打爆（每个请求都要查 SQLite + 读磁盘）
 * - 任何单项失败都不影响其他项，调用方无需 try/catch
 * - 返回 Promise，调用方可选择 await 也可忽略（fire-and-forget 预热）
 *
 * @param ids 要预热的 artworkId 列表（null/undefined 会被过滤）
 * @param fetcher 给定 id 返回 blob URL 的函数（默认用 getArtworkUrl）
 * @param concurrency 同时并发的请求数
 */
export async function prefetchArtworks(
  ids: Array<number | null | undefined>,
  fetcher: (id: number) => string,
  concurrency = 6,
): Promise<void> {
  if (!enabled) return;
  // 过滤掉无效值和已缓存项
  const todo: number[] = [];
  const seen = new Set<number>();
  for (const id of ids) {
    if (id == null) continue;
    if (seen.has(id)) continue;
    seen.add(id);
    if (!cache.has(id)) todo.push(id);
  }
  if (todo.length === 0) return;

  // 简单的有界并发执行器
  let cursor = 0;
  async function worker() {
    while (cursor < todo.length) {
      const id = todo[cursor++];
      try {
        const resp = await fetch(fetcher(id));
        if (!resp.ok) continue;
        const blob = await resp.blob();
        await cacheArtworkBlob(id, blob);
      } catch {
        // 单项失败静默忽略；下次访问时 useArtworkSrc 会自然回退到原始 URL
      }
    }
  }

  const workers: Promise<void>[] = [];
  for (let i = 0; i < Math.min(concurrency, todo.length); i++) {
    workers.push(worker());
  }
  await Promise.all(workers);
}

/** 超过上限时按 LRU 淘汰最久未访问的条目 */
function evictIfNeeded(): void {
  while (cache.size > MAX_ENTRIES) {
    // 找到 lastUsed 最小的条目淘汰
    let oldestKey: number | null = null;
    let oldestUsed = Infinity;
    for (const [key, entry] of cache) {
      if (entry.lastUsed < oldestUsed) {
        oldestUsed = entry.lastUsed;
        oldestKey = key;
      }
    }
    if (oldestKey === null) break;
    cache.delete(oldestKey);
  }
}

function blobToDataURL(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}
