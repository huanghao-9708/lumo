/**
 * 开发模式内存诊断采样器（内存治理方案 A.2）。
 *
 * 只在 DEV 构建生效：
 * - release 构建里 registerMemSampler 是空操作，采样零开销；
 * - 诊断轮在 DevTools 控制台用 `__LUMO_MEM__.sample()` 取快照，
 *   或 `__LUMO_MEM__.startLogging(5000)` 周期打印；
 * - 所有计数都是数字/枚举（缓存条数、字节、列表长度、连接数），
 *   不记录任何用户内容（无标题/路径/凭据）。
 *
 * 采集面（方案 §3 的优先调查对象）：
 * - 封面缓存：条数 / 计量字节 / 预算 / 命中与驱逐（artworkCache）
 * - 封面请求：活跃 fetch / 排队 / 去重等待（useArtworkSrc）
 * - 前端列表：tracks / queue / albums / artists / playlists 长度（player store 注册）
 * - 数据库：连接池占用 / 页缓存 / 扫描文件缓存（Rust library_debug_stats）
 */

export interface MemSample {
  ts: string;
  jsHeapUsedMB?: number;
  jsHeapTotalMB?: number;
  [key: string]: number | string | undefined;
}

type Sampler = () => Record<string, number | string>;

const samplers = new Map<string, Sampler>();

/** 注册一个命名采样器（release 构建为空操作） */
export function registerMemSampler(name: string, fn: Sampler): void {
  if (!import.meta.env.DEV) return;
  samplers.set(name, fn);
}

/** 汇总当前所有采样器 + JS 堆（performance.memory 仅 Chromium 提供） */
export function collectMemSample(): MemSample {
  const out: MemSample = { ts: new Date().toISOString() };
  const mem = (performance as any).memory;
  if (mem) {
    out.jsHeapUsedMB = Math.round((mem.usedJSHeapSize / 1048576) * 10) / 10;
    out.jsHeapTotalMB = Math.round((mem.totalJSHeapSize / 1048576) * 10) / 10;
  }
  for (const [name, fn] of samplers) {
    try {
      Object.assign(out, fn());
    } catch (e) {
      out[`error_${name}`] = String(e);
    }
  }
  return out;
}

/** 从 Rust 侧取连接池/页缓存/扫描计数（失败返回错误标记，不打断采样） */
async function sampleDb(): Promise<Record<string, number | string>> {
  try {
    const { libraryDebugStats } = await import('../api/library');
    const s = await libraryDebugStats();
    return {
      db_pool_in_use: s.pool_connections_in_use,
      db_pool_idle: s.pool_connections_idle,
      db_pool_max: s.pool_max_size,
      db_cache_kib: s.cache_size_pragma,
      db_scan_cache_entries: s.scan_file_cache_entries,
      db_tracks: s.tracks,
      db_media_files: s.media_files,
    };
  } catch (e) {
    return { db_error: String(e) };
  }
}

if (import.meta.env.DEV && typeof window !== 'undefined') {
  (window as any).__LUMO_MEM__ = {
    sample(): MemSample {
      return collectMemSample();
    },
    /** 异步版：附带 Rust 侧数据库状态 */
    async sampleFull(): Promise<MemSample> {
      return { ...collectMemSample(), ...(await sampleDb()) };
    },
    /** 周期打印采样（诊断轮）；返回停止函数 */
    startLogging(intervalMs = 5000): () => void {
      const timer = window.setInterval(() => {
        void this.sampleFull().then((s: MemSample) => console.info('[MEM]', s));
      }, intervalMs);
      return () => window.clearInterval(timer);
    },
    /** S2 对照：临时清空封面缓存（观察内存差值的受控实验） */
    clearArtworkCache(): void {
      void import('./artworkCache').then((m) => m.clearArtworkCache());
    },
  };
  console.info(
    '%c[LUMO] 内存诊断已启用：__LUMO_MEM__.sample() / sampleFull() / startLogging(ms)',
    'color:#E28A23',
  );
}
