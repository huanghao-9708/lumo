import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';
import { resetScrollPosition } from '../../composables/useScrollRestore';
import { usePlayerStore } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';
import MinimalEntityList from './MinimalEntityList.vue';
import AlbumGrid from './AlbumGrid.vue';
import ArtistGrid from './ArtistGrid.vue';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

describe('专辑/艺术家迷你返回的深页窗口', () => {
  let pinia: Pinia;
  let wrapper: VueWrapper | undefined;
  let total: number;
  let resizeCallbacks: (() => void)[];
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;
  const command = (kind: string) => kind === 'album' ? 'library_get_albums' : 'library_get_artists';

  beforeEach(() => {
    pinia = createPinia(); setActivePinia(pinia); localStorage.clear(); resetScrollPosition();
    total = 30000; resizeCallbacks = []; frames.clear();
    vi.stubGlobal('ResizeObserver', class {
      constructor(callback: () => void) { resizeCallbacks.push(callback); }
      observe() {} disconnect() {}
    });
    vi.stubGlobal('IntersectionObserver', class { observe() {} disconnect() {} });
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { frames.set(++frameId, callback); return frameId; });
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      const el = this as HTMLElement;
      const container = el.closest('.overflow-y-auto') as HTMLElement | null;
      return { top: el.classList.contains('relative') && el.style.height ? 60 - (container?.scrollTop ?? 0) : 0 } as DOMRect;
    });
    invokeMock.mockReset().mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'desktop_get_preferences') return {
        preferences: { schemaVersion: 1, experienceMode: 'normal', windowForm: 'full' }, fileExisted: true,
      };
      if (cmd === 'library_get_album_count') return total;
      if (cmd === 'library_get_albums' || cmd === 'library_get_artists') {
        const items = Array.from({ length: Math.max(0, Math.min(args.limit, total - args.offset)) }, (_, i) => ({
          id: args.offset + i + 1, title: `专辑${args.offset + i + 1}`, name: `艺术家${args.offset + i + 1}`, track_count: 10,
        }));
        return cmd === 'library_get_albums' ? items : { artists: items, total };
      }
      if (cmd === 'library_get_counts') return { tracks: 0 };
      return [];
    });
  });
  afterEach(() => { wrapper?.unmount(); wrapper = undefined; disposePinia(pinia); vi.unstubAllGlobals(); vi.restoreAllMocks(); });

  async function settle() {
    await flushPromises();
    for (let i = 0; i < 4 && frames.size; i++) {
      const work = [...frames.values()]; frames.clear(); work.forEach(callback => callback(0));
      await nextTick(); await flushPromises();
    }
  }

  function render(kind: 'album' | 'artist', minimal: boolean) {
    const player = usePlayerStore();
    wrapper = minimal ? mount(defineComponent({ setup: () => () => h(MinimalEntityList, {
      kind, scrollKey: `${kind}:query`,
      items: (kind === 'album' ? player.albums : player.artists).map(item => ({ id: item.id, primary: 'title' in item ? item.title : item.name })),
      hasMore: kind === 'album' ? player.hasMoreAlbums : player.hasMoreArtists,
      loading: kind === 'album' ? player.isLoadingAlbums : player.isLoadingArtists,
      loadError: kind === 'album' ? player.isErrorAlbums : player.isErrorArtists,
      loadMore: kind === 'album' ? player.fetchAlbums : player.fetchArtists,
    }) })) : mount(kind === 'album' ? AlbumGrid : ArtistGrid);
    return wrapper;
  }

  async function viewport(minimal: boolean) {
    await nextTick();
    const el = wrapper!.get('.overflow-y-auto').element as HTMLElement;
    Object.defineProperty(el, 'clientHeight', { configurable: true, value: 300 });
    Object.defineProperty(el, 'clientWidth', { configurable: true, value: 864 });
    Object.defineProperty(el, 'scrollHeight', { configurable: true, get: () => {
      const node = el.querySelector(minimal ? 'div[style*="height"]' : 'div.relative[style*="height"]') as HTMLElement | null;
      return (parseFloat(node?.style.height ?? '0') || 0) + (minimal ? 60 : 120);
    } });
    resizeCallbacks.forEach(callback => callback()); await settle();
    return el;
  }

  it.each([
    ['album', true], ['artist', true], ['album', false], ['artist', false],
  ] as const)('%s / 文字列表=%s：深页返回只取一个窗口，上下跳转仍可达且数据不累计', async (kind, minimal) => {
    const mode = useDesktopModeStore(); await mode.init();
    if (minimal) await mode.setExperienceMode('minimal');
    const player = usePlayerStore(); player.activeLibraryTab = kind === 'album' ? '专辑' : '艺术家';
    const fetch = kind === 'album' ? player.fetchAlbums : player.fetchArtists;
    const items = () => kind === 'album' ? player.albums : player.artists;
    await fetch(true); for (let i = 0; i < 5; i++) await fetch();
    render(kind, minimal); const original = await viewport(minimal);
    const columns = minimal ? 1 : 4;
    const rowHeight = minimal ? 44 : kind === 'album' ? 262 : 258;
    const prefix = minimal ? 0 : 60;
    original.scrollTop = prefix + Math.floor(140 / columns) * rowHeight + rowHeight / 4;
    await wrapper!.get('.overflow-y-auto').trigger('scroll'); await settle();
    const firstId = items()[140].id;
    await mode.enterMini(); wrapper!.unmount(); wrapper = undefined;
    expect(items()).toHaveLength(0);
    await mode.exitMini();
    invokeMock.mockClear(); render(kind, minimal); const restored = await viewport(minimal);
    await player.applyBrowseRestore(); await settle();
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === command(kind)).map(([, args]) => [args.offset, args.limit])).toEqual([[110, 120]]);
    expect(items()).toHaveLength(120);
    expect(player.entityBrowseRestore?.index).toBe(140);
    expect(player.entityBrowseRestore?.rowFraction).toBeCloseTo(0.25);
    expect(restored.scrollTop).toBeCloseTo(prefix + Math.floor(140 / columns) * rowHeight + rowHeight / 4);
    expect(items().some(item => item.id === firstId)).toBe(true);
    for (const index of [6000, 0, 15000, 100, 29995, 140]) {
      restored.scrollTop = prefix + Math.floor(index / columns) * rowHeight;
      await wrapper!.get('.overflow-y-auto').trigger('scroll'); await settle();
      expect(items().length).toBeLessThanOrEqual(120);
      expect(items().some(item => item.id === index + 1)).toBe(true);
      expect(wrapper!.findAll(minimal ? 'button[title]' : '.grid > div').length).toBeLessThan(40);
    }
  });

  it('旧位置超出缩短后的列表时直接落到末尾附近，不重放所有历史页', async () => {
    const mode = useDesktopModeStore(); await mode.init();
    const player = usePlayerStore(); player.activeLibraryTab = '专辑';
    await player.fetchAlbums(true, { offset: 1000, limit: 120 });
    player.recordEntityBrowseAnchor('album', 1060, 0);
    await mode.enterMini(); total = 50; await mode.exitMini(); invokeMock.mockClear();
    await player.applyBrowseRestore();
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === command('album')).map(([, args]) => args.offset)).toEqual([1030, 0]);
    expect(player.albums).toHaveLength(50);
    expect(player.entityBrowseRestore?.index).toBe(49);
  });

  it.each(['album', 'artist'] as const)('%s：原生窗口先缩小再提交迷你状态时，锚点仍使用完整窗口的行列', async kind => {
    const mode = useDesktopModeStore(); await mode.init(); const player = usePlayerStore();
    player.activeLibraryTab = kind === 'album' ? '专辑' : '艺术家';
    await (kind === 'album' ? player.fetchAlbums(true, { offset: 100, limit: 120 }) : player.fetchArtists(true, { offset: 100, limit: 120 }));
    render(kind, false); const original = await viewport(false);
    const rowHeight = kind === 'album' ? 262 : 258;
    original.scrollTop = 60 + 35 * rowHeight + rowHeight / 4;
    await wrapper!.get('.overflow-y-auto').trigger('scroll'); await settle();
    const geometry = { x: 0, y: 0, width: 1200, height: 720, maximized: false };
    mode.attachWindowDriver({
      changeForm: async (_from, to) => {
        if (to === 'mini') {
          Object.defineProperty(original, 'clientWidth', { configurable: true, value: 320 });
          Object.defineProperty(original, 'clientHeight', { configurable: true, value: 32 });
          resizeCallbacks.forEach(callback => callback()); await settle();
        }
        return { previousGeometry: geometry, nextGeometry: { ...geometry, width: 560, height: 96 } };
      },
      setAlwaysOnTop: async () => {},
    });
    await mode.enterMini(); wrapper!.unmount(); wrapper = undefined;
    await mode.exitMini(); await player.applyBrowseRestore();
    expect(player.entityBrowseRestore?.index).toBe(140);
    expect(player.entityBrowseRestore?.rowFraction).toBeCloseTo(0.25);
  });

  it('改变筛选会回到正常首批，迟到的旧窗口请求不能覆盖新条件', async () => {
    const mode = useDesktopModeStore(); await mode.init();
    const player = usePlayerStore();
    await player.fetchAlbums(true, { offset: 1000, limit: 120 });
    const originalMock = invokeMock.getMockImplementation()!;
    let finish!: (items: unknown[]) => void;
    invokeMock.mockImplementation((cmd: string, args: any) => cmd === 'library_get_albums' && args.offset > 0
      ? new Promise(resolve => { finish = resolve; }) : originalMock(cmd, args));
    const oldRead = player.ensureEntityBrowseWindow('album', 2000, 2010);
    player.searchQuery = '新筛选'; const newRead = player.fetchAlbums(true);
    finish([{ id: 99999, title: '旧窗口' }]); await Promise.all([oldRead, newRead]);
    expect(player.albumsWindowed).toBe(false);
    expect(player.albumsWindowStart).toBe(0);
    expect(player.albums).toHaveLength(30);
    expect(player.albums.some(item => item.id === 99999)).toBe(false);
    expect(invokeMock.mock.calls.some(([cmd, args]) => cmd === command('album') && args.searchKeyword === '新筛选' && args.offset === 0)).toBe(true);
  });

  it.each([['album', true], ['artist', true], ['album', false], ['artist', false]] as const)(
    '%s / 文字列表=%s：窗口读取失败保持可重试且不自动循环请求', async (kind, minimal) => {
      const mode = useDesktopModeStore(); await mode.init();
      const player = usePlayerStore();
      await (kind === 'album' ? player.fetchAlbums(true, { offset: 1000, limit: 120 }) : player.fetchArtists(true, { offset: 1000, limit: 120 }));
      const implementation = invokeMock.getMockImplementation()!;
      let failing = true;
      const logging = vi.spyOn(console, 'error').mockImplementation(() => {});
      invokeMock.mockImplementation((cmd: string, args: any) => cmd === command(kind) && failing
        ? Promise.reject(new Error('读取失败')) : implementation(cmd, args));
      try {
        render(kind, minimal); const el = await viewport(minimal); await settle();
        expect(wrapper!.text()).toContain('加载失败');
        const failedReads = invokeMock.mock.calls.filter(([cmd]) => cmd === command(kind)).length;
        for (let i = 0; i < 3; i++) { await wrapper!.get('.overflow-y-auto').trigger('scroll'); await settle(); }
        expect(invokeMock.mock.calls.filter(([cmd]) => cmd === command(kind))).toHaveLength(failedReads);
        failing = false;
        const retry = wrapper!.findAll('button').find(button => button.text() === '重试加载')!;
        await retry.trigger('click'); await settle();
        expect(wrapper!.text()).not.toContain('加载失败');
        expect(kind === 'album' ? player.albumsWindowStart : player.artistsWindowStart).toBe(0);
        el.scrollTop = 0; await wrapper!.get('.overflow-y-auto').trigger('scroll'); await settle();
        expect(kind === 'album' ? player.albums[0].id : player.artists[0].id).toBe(1);
      } finally { logging.mockRestore(); }
    },
  );
});
