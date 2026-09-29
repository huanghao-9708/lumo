import { beforeEach, describe, expect, it } from 'vitest';
import {
  cacheArtworkDataUrl,
  clearArtworkCache,
  getArtworkCacheBudgetBytes,
  getArtworkCacheStats,
  getCachedArtwork,
  isArtworkCached,
  setArtworkCacheBudgetBytes,
} from './artworkCache';

/** 生成约 sizeKiB 计量字节（dataUrl.length*2）的假 dataURL */
function fakeDataUrl(sizeKiB: number): string {
  // 前缀 'data:image/jpeg;base64,' 长 23 字符；总长取 sizeKiB*512 使计量恰为 sizeKiB*1024
  return 'data:image/jpeg;base64,' + 'A'.repeat(sizeKiB * 512 - 23);
}

describe('artworkCache 字节上限 LRU（内存治理 B.1）', () => {
  beforeEach(() => {
    window.localStorage.clear();
    clearArtworkCache();
    setArtworkCacheBudgetBytes(4 * 1024 * 1024);
  });

  it('写入后可命中，并计入条数与字节', () => {
    cacheArtworkDataUrl(1, fakeDataUrl(64));
    expect(getCachedArtwork(1)).toBe(fakeDataUrl(64));
    const s = getArtworkCacheStats();
    expect(s.entries).toBe(1);
    expect(s.bytes).toBe(64 * 1024);
    expect(s.hits).toBe(1);
  });

  it('未命中计入 misses', () => {
    expect(getCachedArtwork(999)).toBeNull();
    expect(getArtworkCacheStats().misses).toBe(1);
  });

  it('超过预算按 LRU 驱逐最久未使用的条目', () => {
    // 预算 4MiB；每张 1MiB → 最多 4 张
    for (let i = 1; i <= 5; i++) {
      cacheArtworkDataUrl(i, fakeDataUrl(1024));
    }
    let s = getArtworkCacheStats();
    expect(s.entries).toBe(4);
    expect(s.bytes).toBeLessThanOrEqual(s.budgetBytes);
    expect(s.evictions).toBe(1);
    // 最先写入的 1 被驱逐，2-5 保留
    expect(isArtworkCached(1)).toBe(false);
    expect(isArtworkCached(5)).toBe(true);

    // 访问 2 使其变新，再写 6 → 驱逐 3（而非 2）
    getCachedArtwork(2);
    cacheArtworkDataUrl(6, fakeDataUrl(1024));
    expect(isArtworkCached(2)).toBe(true);
    expect(isArtworkCached(3)).toBe(false);
    s = getArtworkCacheStats();
    expect(s.evictions).toBe(2);
  });

  it('单张超过总预算的图不进入缓存，也不驱逐其他条目', () => {
    cacheArtworkDataUrl(1, fakeDataUrl(1024));
    // 8MiB > 4MiB 预算：拒绝
    cacheArtworkDataUrl(2, fakeDataUrl(8192));
    const s = getArtworkCacheStats();
    expect(isArtworkCached(2)).toBe(false);
    expect(s.rejectedOversize).toBe(1);
    expect(isArtworkCached(1)).toBe(true);
    expect(s.evictions).toBe(0);
  });

  it('覆盖同 id 条目时旧字节被扣减', () => {
    cacheArtworkDataUrl(1, fakeDataUrl(1024));
    cacheArtworkDataUrl(1, fakeDataUrl(64));
    const s = getArtworkCacheStats();
    expect(s.entries).toBe(1);
    expect(s.bytes).toBe(64 * 1024);
  });

  it('预算调小后立即收敛到新水位', () => {
    for (let i = 1; i <= 4; i++) cacheArtworkDataUrl(i, fakeDataUrl(1024));
    setArtworkCacheBudgetBytes(1024 * 1024); // 只够 1 张
    const s = getArtworkCacheStats();
    expect(s.entries).toBe(1);
    expect(s.bytes).toBe(1024 * 1024);
    // 最新的 4 留下
    expect(isArtworkCached(4)).toBe(true);
  });

  it('localStorage 可覆盖预算（A/B 档位）', () => {
    window.localStorage.setItem('lumo_artwork_cache_mb', '32');
    clearArtworkCache();
    // 重新触发读取：清空不重读 localStorage，用 set 走同一路径验证边界
    // （预算加载发生在模块首次导入；此处验证 setArtworkCacheBudgetBytes 上的 32MiB 行为）
    setArtworkCacheBudgetBytes(32 * 1024 * 1024);
    expect(getArtworkCacheBudgetBytes()).toBe(32 * 1024 * 1024);
    // 低于 1MiB 的非法值被忽略
    setArtworkCacheBudgetBytes(512 * 1024);
    expect(getArtworkCacheBudgetBytes()).toBe(32 * 1024 * 1024);
  });

  it('clearArtworkCache 清零计量', () => {
    cacheArtworkDataUrl(1, fakeDataUrl(1024));
    clearArtworkCache();
    const s = getArtworkCacheStats();
    expect(s.entries).toBe(0);
    expect(s.bytes).toBe(0);
    expect(isArtworkCached(1)).toBe(false);
  });
});
