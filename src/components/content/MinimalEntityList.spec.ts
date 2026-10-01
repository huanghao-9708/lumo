import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import MinimalEntityList from './MinimalEntityList.vue';
import { resetScrollPosition } from '../../composables/useScrollRestore';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
import { usePlayerStore } from '../../stores/player';

describe('极简专辑/艺术家分页', () => {
  let pinia: Pinia;
  let wrapper: VueWrapper;
  let resizeCallbacks: (() => void)[];
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    localStorage.clear();
    resetScrollPosition();
    invokeMock.mockReset();
    resizeCallbacks = [];
    frames.clear();
    vi.stubGlobal('ResizeObserver', class {
      constructor(callback: () => void) { resizeCallbacks.push(callback); }
      observe() {}
      disconnect() {}
    });
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.set(++frameId, callback);
      return frameId;
    });
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
  });
  afterEach(() => {
    wrapper?.unmount();
    disposePinia(pinia);
    vi.unstubAllGlobals();
  });

  async function setViewport(height: number, getHeight: () => number) {
    await nextTick();
    const el = wrapper.get('[data-experience-minimal-list]').element as HTMLElement;
    Object.defineProperty(el, 'clientHeight', { configurable: true, value: height });
    Object.defineProperty(el, 'scrollHeight', { configurable: true, get: getHeight });
    resizeCallbacks.forEach(callback => callback());
    await flushPromises();
    return el;
  }

  it.each(['album', 'artist'] as const)('%s：滚动加载真实 store 的第2/3页，末页停止且仍只渲染视口附近', async kind => {
    const command = kind === 'album' ? 'library_get_albums' : 'library_get_artists';
    const offsets: number[] = [];
    invokeMock.mockImplementation(async (cmd: string, args: { offset: number; limit: number }) => {
      if (cmd === 'library_get_album_count') return 75;
      if (cmd === command) {
        offsets.push(args.offset);
        const page = Array.from({ length: Math.min(args.limit, 75 - args.offset) }, (_, i) => ({
          id: args.offset + i + 1, title: `专辑${args.offset + i + 1}`, name: `艺术家${args.offset + i + 1}`, track_count: 10,
        }));
        return kind === 'album' ? page : { artists: page, total: 75 };
      }
      return [];
    });
    const store = usePlayerStore();
    const loadMore = kind === 'album' ? store.fetchAlbums : store.fetchArtists;
    const getItems = () => kind === 'album' ? store.albums : store.artists;
    await loadMore(true);
    wrapper = mount(defineComponent({ setup: () => () => h(MinimalEntityList, {
      kind, scrollKey: kind,
      items: getItems().map(item => ({ id: item.id, primary: 'title' in item ? item.title : item.name })),
      hasMore: kind === 'album' ? store.hasMoreAlbums : store.hasMoreArtists,
      loading: kind === 'album' ? store.isLoadingAlbums : store.isLoadingArtists,
      loadError: kind === 'album' ? store.isErrorAlbums : store.isErrorArtists,
      loadMore,
    }) }));
    const el = await setViewport(300, () => getItems().length * 44 + 60);
    expect(offsets).toEqual([0]);
    for (const expectedCount of [60, 75]) {
      el.scrollTop = el.scrollHeight - el.clientHeight;
      await wrapper.get('[data-experience-minimal-list]').trigger('scroll');
      await flushPromises();
      expect(getItems()).toHaveLength(expectedCount);
    }
    expect(offsets).toEqual([0, 30, 60]);
    el.scrollTop = el.scrollHeight - el.clientHeight;
    await wrapper.get('[data-experience-minimal-list]').trigger('scroll');
    await flushPromises();
    expect(offsets).toEqual([0, 30, 60]);
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.findAll('button').length).toBeLessThan(40);
    expect(wrapper.text()).not.toContain('加载更多');
  });

  it('连续滚动只有一个在途请求，失败不自动重试，点击重试后完成', async () => {
    let reject!: (error: Error) => void;
    const hasMore = ref(true);
    const loadMore = vi.fn()
      .mockImplementationOnce(() => new Promise<void>((_resolve, rejectPage) => { reject = rejectPage; }))
      .mockImplementationOnce(async () => { hasMore.value = false; });
    wrapper = mount(defineComponent({ setup: () => () => h(MinimalEntityList, {
      kind: 'artist', scrollKey: 'artist', items: [{ id: 1, primary: '艺术家' }], hasMore: hasMore.value, loadMore,
    }) }));
    await setViewport(300, () => 500);
    const list = wrapper.get('[data-experience-minimal-list]');
    (list.element as HTMLElement).scrollTop = 200;
    await list.trigger('scroll');
    await list.trigger('scroll');
    expect(loadMore).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('加载中');
    reject(new Error('读取失败'));
    await flushPromises();
    await list.trigger('scroll');
    expect(loadMore).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('加载失败');
    await wrapper.findAll('button').find(button => button.text() === '重试加载')!.trigger('click');
    await flushPromises();
    expect(loadMore).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).not.toContain('加载失败');
  });

  it('首批不足一屏时补充下一批，收藏列表没有分页请求', async () => {
    const items = ref([{ id: 1, primary: '专辑1' }]);
    const loadMore = vi.fn(async () => {
      items.value = Array.from({ length: 30 }, (_, id) => ({ id, primary: `专辑${id}` }));
    });
    wrapper = mount(defineComponent({ setup: () => () => h(MinimalEntityList, {
      kind: 'album', scrollKey: 'album', items: items.value, hasMore: true, loadMore,
    }) }));
    await setViewport(600, () => items.value.length * 44 + 60);
    expect(loadMore).toHaveBeenCalledTimes(1);
    wrapper.unmount();
    wrapper = mount(MinimalEntityList, { props: { kind: 'album', scrollKey: 'favorites', items: items.value } });
    await setViewport(1500, () => items.value.length * 44);
    await wrapper.get('[data-experience-minimal-list]').trigger('scroll');
    expect(loadMore).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).not.toContain('加载更多');
  });

  it('卸载后迟到页完成不会再触发下一页', async () => {
    let resolve!: () => void;
    const items = ref([{ id: 1, primary: '专辑1' }]);
    const loadMore = vi.fn(() => new Promise<void>(done => {
      resolve = () => { items.value.push({ id: 2, primary: '专辑2' }); done(); };
    }));
    wrapper = mount(defineComponent({ setup: () => () => h(MinimalEntityList, {
      kind: 'album', scrollKey: 'album', items: [...items.value], hasMore: true, loadMore,
    }) }));
    await setViewport(600, () => items.value.length * 44);
    expect(loadMore).toHaveBeenCalledTimes(1);
    wrapper.unmount();
    resolve();
    await flushPromises();
    resizeCallbacks.forEach(callback => callback());
    expect(loadMore).toHaveBeenCalledTimes(1);
  });
});
