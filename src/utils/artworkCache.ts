/**
 * 封面图片内存缓存（字节上限 LRU）。
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
 * 内存代价（2026-09 内存治理，方案 §4-B）：
 * dataURL 常驻 JS 堆。旧实现按"条数"上限（5000 张）没有字节上限，注释按
 * 50KB/张自估约 330MB——原图尺寸变大后条数上限形同虚设。现改为**按编码后
 * 字节计量的 LRU**：
 * - 默认预算 16 MiB（可用 localStorage `lumo_artwork_cache_mb` 覆盖为 16/32
 *   等，供 S2 场景 A/B 对比）；
 * - 单张超过总预算的图不进入缓存（异常大图直接走原始 URL，由 WebView2 的
 *   HTTP 缓存兜底），也不会把已缓存的其他封面挤出去；
 * - 计量取 `dataUrl.length * 2`：base64 是 ASCII，V8 单字节串按 1B/char 存，
 *   但 dataURL 在赋值/传递中可能被复制多份，按 2B/char 取保守上界（宁可
 *   早驱逐，不少记字节）；
 * - 命中/未命中/驱逐/拒绝计数经 `getArtworkCacheStats()` 暴露，仅供开发模式
 *   诊断（memDiagnostics）读取，不记录任何用户内容。
 */

/** 默认缓存预算（字节）：16 MiB。A/B 档位 16/32 MiB。 */
const DEFAULT_BUDGET_BYTES = 16 * 1024 * 1024;

/** localStorage 键：把预算覆盖成指定 MiB 数（A/B 实验用，1~512） */
const BUDGET_OVERRIDE_KEY = 'lumo_artwork_cache_mb';

interface CacheEntry {
  dataUrl: string;
  /** 计量字节（保守上界，见文件头说明） */
  bytes: number;
}

/** Map 按插入序 = LRU 序：命中时 delete+set 移到尾部，淘汰从头部取 */
const cache = new Map<number | string, CacheEntry>();
let totalBytes = 0;
let budgetBytes = loadBudget();

function loadBudget(): number {
  try {
    const raw = window.localStorage.getItem(BUDGET_OVERRIDE_KEY);
    if (raw) {
      const mb = Number(raw);
      if (Number.isFinite(mb) && mb >= 1 && mb <= 512) {
        return Math.round(mb * 1024 * 1024);
      }
    }
  } catch {
    // localStorage 不可用时用默认值
  }
  return DEFAULT_BUDGET_BYTES;
}

/** 诊断计数器（开发模式经 memDiagnostics 读取） */
const stats = { hits: 0, misses: 0, evictions: 0, rejectedOversize: 0 };

/** 当前是否已开启缓存。可通过 `setArtworkCacheEnabled(false)` 临时关闭（例如清缓存时） */
let enabled = true;

/** 命中缓存时返回 dataURL，否则返回 null。访问即"使用"，会更新 LRU 顺序。 */
export function getCachedArtwork(artworkId: number | string | null | undefined): string | null {
  if (!enabled || artworkId == null) return null;
  const entry = cache.get(artworkId);
  if (!entry) {
    stats.misses++;
    return null;
  }
  // 重新插入 = 移到 LRU 尾部（Map 保持插入序）
  cache.delete(artworkId);
  cache.set(artworkId, entry);
  stats.hits++;
  return entry.dataUrl;
}

/**
 * 把一个 artwork 对应的原始 blob 数据写入缓存。
 * 内部会转成 dataURL 存储。
 */
export async function cacheArtworkBlob(artworkId: number | string, blob: Blob): Promise<string> {
  const dataUrl = await blobToDataURL(blob);
  if (enabled) {
    insert(artworkId, dataUrl);
  }
  return dataUrl;
}

/** 直接以 dataURL 形式写入缓存（已知 dataURL 时用，省一次转换） */
export function cacheArtworkDataUrl(artworkId: number | string, dataUrl: string): void {
  if (!enabled) return;
  insert(artworkId, dataUrl);
}

/** 写入 + 按字节预算淘汰；返回是否真正进入缓存 */
function insert(artworkId: number | string, dataUrl: string): boolean {
  const bytes = dataUrl.length * 2;
  // 单张超过总预算：不缓存（异常大图走 URL 兜底），也不驱逐其他条目
  if (bytes > budgetBytes) {
    stats.rejectedOversize++;
    return false;
  }
  // 覆盖旧条目时先扣减旧计量
  const old = cache.get(artworkId);
  if (old) {
    totalBytes -= old.bytes;
    cache.delete(artworkId);
  }
  cache.set(artworkId, { dataUrl, bytes });
  totalBytes += bytes;
  evictIfNeeded();
  return true;
}

/** 清空所有缓存（例如用户在设置里点了"清空封面缓存"）；诊断计数同步归零 */
export function clearArtworkCache(): void {
  cache.clear();
  totalBytes = 0;
  stats.hits = 0;
  stats.misses = 0;
  stats.evictions = 0;
  stats.rejectedOversize = 0;
}

/** 临时启用/禁用缓存 */
export function setArtworkCacheEnabled(value: boolean): void {
  enabled = value;
  if (!value) clearArtworkCache();
}

/**
 * 判断某个 artworkId 是否已在缓存中（不更新 LRU 顺序）。
 * 预加载器用它来跳过已缓存的项，避免重复 fetch。
 */
export function isArtworkCached(artworkId: number | null | undefined): boolean {
  if (!enabled || artworkId == null) return false;
  return cache.has(artworkId);
}

/** 调整预算（字节，≥1 MiB 才生效）；调小后立即按 LRU 收敛到新水位 */
export function setArtworkCacheBudgetBytes(bytes: number): void {
  if (!Number.isFinite(bytes) || bytes < 1024 * 1024) return;
  budgetBytes = Math.round(bytes);
  evictIfNeeded();
}

/** 当前生效的预算（字节） */
export function getArtworkCacheBudgetBytes(): number {
  return budgetBytes;
}

/**
 * 缓存诊断快照：条数 / 计量字节 / 预算 / 命中等计数。
 * 仅开发模式诊断用，不包含任何用户内容。
 */
export function getArtworkCacheStats(): {
  entries: number;
  bytes: number;
  budgetBytes: number;
  hits: number;
  misses: number;
  evictions: number;
  rejectedOversize: number;
} {
  return {
    entries: cache.size,
    bytes: totalBytes,
    budgetBytes,
    hits: stats.hits,
    misses: stats.misses,
    evictions: stats.evictions,
    rejectedOversize: stats.rejectedOversize,
  };
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
        // 联网行为: local —— lumo:// 自定义协议，走本机 IPC 不出网
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

/** 超过字节预算时从 LRU 头部（最久未使用）淘汰 */
function evictIfNeeded(): void {
  while (totalBytes > budgetBytes && cache.size > 0) {
    const oldestKey = cache.keys().next().value;
    if (oldestKey === undefined) break;
    const entry = cache.get(oldestKey);
    cache.delete(oldestKey);
    if (entry) totalBytes -= entry.bytes;
    stats.evictions++;
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
