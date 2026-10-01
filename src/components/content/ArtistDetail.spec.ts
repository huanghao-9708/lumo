import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, shallowMount, type VueWrapper } from '@vue/test-utils';
import MainContent from '../layout/MainContent.vue';
import ArtistDetail from './ArtistDetail.vue';
import { usePlayerStore } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

describe('艺术家独立详情与极简专辑', () => {
  let wrapper: VueWrapper;
  let pinia: Pinia;
  const albums = Array.from({ length: 16 }, (_, i) => ({
    id: 100 + i, title: `专辑${i + 1}`, artist_name: '测试艺术家', release_year: 2020,
    track_count: 10, cover_artwork_id: 1000 + i, cover_thumbnail_base64: 'data:image/png;base64,test',
  }));

  beforeEach(() => {
    localStorage.clear();
    pinia = createPinia();
    setActivePinia(pinia);
    invokeMock.mockReset().mockImplementation(async (cmd: string, args: { offset?: number; limit?: number }) => {
      if (cmd === 'library_get_artist_stats') return { track_count: 1, album_count: 16 };
      if (cmd === 'library_get_artist_tracks') return [{
        id: 1, title: '详情歌曲', artist_name: '测试艺术家', album_title: '专辑1', duration_ms: 180000, is_favorite: false,
      }];
      if (cmd === 'library_get_artist_album_count') return albums.length;
      if (cmd === 'library_get_artist_albums') return albums.slice(args.offset, (args.offset ?? 0) + (args.limit ?? 15));
      if (cmd === 'library_get_album_count') return 0;
      if (cmd === 'library_get_artists') return { artists: [], total: 0 };
      return [];
    });
  });
  afterEach(async () => {
    wrapper?.unmount();
    await flushPromises();
    disposePinia(pinia);
  });

  async function openDetail(minimal = false) {
    const store = usePlayerStore();
    store.artists = [{ id: 7, name: '测试艺术家', trackCount: 1, avatarColor: '', avatar_artwork_id: 123 }];
    store.artistsTotalCount = 9999;
    store.activeLibraryTab = '艺术家';
    store.searchQuery = '列表筛选';
    await flushPromises();
    if (minimal) await useDesktopModeStore().setExperienceMode('minimal');
    store.navigateToArtist(7);
    await flushPromises();
    wrapper = shallowMount(MainContent, { global: { stubs: { ArtistDetail: false } } });
    await flushPromises();
    return store;
  }

  async function click(text: string) {
    const button = wrapper.findAll('button').find(item => item.text() === text);
    expect(button, text).toBeDefined();
    await button!.trigger('click');
    await flushPromises();
  }

  it('详情独占内容区，搜索只作用于当前艺术家，返回后恢复列表页头', async () => {
    const store = await openDetail();
    expect(wrapper.findComponent(ArtistDetail).exists()).toBe(true);
    expect(wrapper.findAll('h1').map(item => item.text())).toEqual(['测试艺术家']);
    expect(wrapper.text()).not.toContain('9,999 位艺术家');
    expect(wrapper.find('input[placeholder="搜索歌曲、艺术家、专辑…"]').exists()).toBe(false);
    const search = wrapper.get<HTMLInputElement>('[aria-label="搜索艺术家歌曲"]');
    expect(search.element.value).toBe('');
    await search.setValue('不存在');
    await flushPromises();
    expect(wrapper.text()).toContain('没有匹配的歌曲');
    expect(store.searchQuery).toBe('列表筛选');
    store.goBack();
    await flushPromises();
    expect(wrapper.findComponent(ArtistDetail).exists()).toBe(false);
    expect(wrapper.find('h1').text()).toBe('艺术家');
    expect(wrapper.text()).toContain('9,999 位艺术家');
  });

  it('正常→极简→正常即时卸载/恢复图片，全部专辑分栏与页码不变', async () => {
    const store = await openDetail();
    await click('全部专辑');
    expect(wrapper.findAll('img')).toHaveLength(16); // avatar + first 15 albums
    const mode = useDesktopModeStore();
    await mode.setExperienceMode('minimal');
    await flushPromises();
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.find('[data-artist-minimal-albums]').exists()).toBe(true);
    expect(wrapper.find('[data-artist-minimal-albums]').findAll('button')).toHaveLength(15);
    expect(wrapper.find('[class*="bg-gradient"]').exists()).toBe(false);
    expect(store.currentArtistDetails?.subTab).toBe('albums');
    expect(store.currentArtistDetails?.albumsCurrentPage).toBe(1);
    await mode.setExperienceMode('normal');
    await flushPromises();
    expect(wrapper.find('[data-artist-minimal-albums]').exists()).toBe(false);
    expect(wrapper.findAll('img')).toHaveLength(16);
    expect(store.currentArtistDetails?.subTab).toBe('albums');
  });

  it('极简专辑仍按15条翻页，文字条目进入专辑，返回保持详情分栏和页码', async () => {
    const store = await openDetail(true);
    await click('全部专辑');
    await click('下一页');
    expect(wrapper.find('[data-artist-minimal-albums]').text()).toContain('专辑16');
    expect(wrapper.find('[data-artist-minimal-albums]').findAll('button')).toHaveLength(1);
    expect(store.currentArtistDetails?.albumsCurrentPage).toBe(2);
    expect(wrapper.find('img').exists()).toBe(false);
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'library_get_artist_albums').map(([, args]) => args.offset)).toEqual([0, 15]);
    await wrapper.get('[data-artist-minimal-albums] button').trigger('click');
    await flushPromises();
    expect(store.activeLibraryTab).toBe('专辑');
    expect(store.activeAlbumId).toBe(115);
    store.goBack();
    await flushPromises();
    expect(wrapper.findComponent(ArtistDetail).exists()).toBe(true);
    expect(store.currentArtistDetails?.subTab).toBe('albums');
    expect(store.currentArtistDetails?.albumsCurrentPage).toBe(2);
    expect(wrapper.get('[data-artist-minimal-albums]').text()).toContain('专辑16');
    await click('上一页');
    expect(wrapper.get('[data-artist-minimal-albums]').findAll('button')).toHaveLength(15);
  });
});
