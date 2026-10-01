import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { reactive, type Component } from 'vue';
import type { Track } from '../../../stores/player';
import MainContent from '../../layout/MainContent.vue';
import RecentlyPlayed from '../../content/RecentlyPlayed.vue';
import FavoritesView from '../../content/FavoritesView.vue';
import AlbumDetail from '../../content/AlbumDetail.vue';
import ArtistDetail from '../../content/ArtistDetail.vue';
import PlaylistDetail from '../../content/PlaylistDetail.vue';
import FolderView from '../../content/FolderView.vue';
import SmartPlaylistView from '../../content/SmartPlaylistView.vue';

const { state, invokeMock } = vi.hoisted(() => ({ state: { player: null as any }, invokeMock: vi.fn() }));
vi.mock('../../../stores/player', async importOriginal => ({
  ...await importOriginal<object>(), usePlayerStore: () => state.player,
}));
vi.mock('../../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

const pages: { name: string; component: Component; tab: string; props?: object }[] = [
  { name: '全部歌曲', component: MainContent, tab: '全部歌曲' },
  { name: '播放队列', component: MainContent, tab: '播放列表' },
  { name: '最近播放', component: RecentlyPlayed, tab: '最近播放' },
  { name: '喜欢的音乐', component: FavoritesView, tab: '喜欢的音乐' },
  { name: '专辑详情', component: AlbumDetail, tab: '专辑', props: { albumId: 7 } },
  { name: '艺术家详情', component: ArtistDetail, tab: '艺术家', props: { artistId: 7 } },
  { name: '歌单详情', component: PlaylistDetail, tab: '播放列表' },
  { name: '文件夹', component: FolderView, tab: '文件夹' },
  { name: '智能歌单', component: SmartPlaylistView, tab: '智能歌单' },
];

describe('歌曲列表统一多选交互', () => {
  let pinia: Pinia;
  let wrapper: VueWrapper;
  beforeEach(() => {
    localStorage.clear();
    invokeMock.mockReset().mockResolvedValue([]);
    pinia = createPinia(); setActivePinia(pinia);
    const tracks: Track[] = [1, 2].map(id => ({
      id, title: `歌曲${id}`, artistId: 7, artist: '歌手', albumId: 7, album: '专辑',
      duration: '03:00', durationSec: 180, format: 'FLAC', coverColor: '', isFavorite: false,
      fileSize: null, sourceKind: 'local',
    }));
    state.player = reactive({
      tracks, queue: [...tracks], folderTracks: [...tracks], smartPlaylistTracks: [...tracks],
      currentTrack: null, isPlaying: false, activeLibraryTab: '', activeAlbumId: null,
      activeArtistId: null, activePlaylistId: null, activeSmartPlaylistKind: 'most_played',
      selectedTreePath: 'Music', localSources: [], albums: [], artists: [],
      folderBrowseContext: { sourceId: null, expandedPaths: {} }, artistDetailFilterQuery: '',
      favoriteAlbums: [], favoriteArtists: [], globalSearchQuery: '', searchQuery: '', playabilityEpoch: 0,
      pendingBrowseRestore: false, isLoadingTracks: false, isErrorTracks: false,
      isLoadingSmartPlaylist: false, isLoadingFolderTracks: false, tracksTotalCount: 2,
      libraryCounts: { favorite_tracks: 2 },
      currentAlbumDetails: { id: 7, title: '专辑', year: 2020, tracks },
      currentArtistDetails: {
        id: 7, name: '歌手', tracks, albums: [], subTab: 'tracks', stats: { track_count: 2, album_count: 0 },
        hasMoreTracks: false, isLoadingTracks: false, isLoadingAlbums: false,
      },
      currentPlaylistDetails: { id: 7, name: '歌单', tracks, isLoadingTracks: false },
      playlists: [{ id: 88, name: '目标歌单', count: 0 }],
      ensurePlayability: vi.fn(), getPlayability: () => undefined, isTrackUnplayable: () => false,
      fetchFavoriteAlbums: vi.fn(), fetchFavoriteArtists: vi.fn(), fetchSources: vi.fn(),
      playTrack: vi.fn(), playQueue: vi.fn(), playAll: vi.fn(), toggleFavorite: vi.fn(),
      navigateToArtist: vi.fn(), navigateToAlbum: vi.fn(),
      batchSetFavorite: vi.fn(async () => {}),
      batchAddToPlaylist: vi.fn(async (_playlistId: number, ids: number[]) => ({ added: ids.length, skipped: 0 })),
      setArtistDetailSubTab: (tab: string) => { state.player.currentArtistDetails.subTab = tab; },
    });
  });
  afterEach(async () => {
    wrapper?.unmount(); await flushPromises(); disposePinia(pinia);
  });

  async function open(page: typeof pages[number]) {
    state.player.activeLibraryTab = page.tab;
    wrapper = mount(page.component, {
      props: page.props,
      global: { stubs: { PlaylistPickerModal: true, ConfirmDialog: true, EqualizerIndicator: true } },
    });
    await flushPromises();
    expect(wrapper.findAll('[title="多选歌曲"]')).toHaveLength(1);
    expect(wrapper.find('[title="列表视图"]').exists()).toBe(false);
    expect(wrapper.find('[title="网格视图"]').exists()).toBe(false);
    await wrapper.get('[title="多选歌曲"]').trigger('click');
  }

  async function click(text: string) {
    const button = wrapper.findAll('button').find(item => item.text() === text);
    expect(button, text).toBeDefined();
    await button!.trigger('click'); await flushPromises();
  }

  it.each(pages)('$name：统一入口、单次选中反馈、全选和批量操作可用且不播放', async page => {
    await open(page);
    const toolbar = wrapper.get('[aria-label="歌曲批量操作"]');
    expect(toolbar.element.closest('.overflow-y-auto')).toBeNull();
    expect(toolbar.text()).toContain('已选 0 首');
    expect(wrapper.get('[aria-pressed="true"]').attributes('title')).toBe('退出多选');
    expect(wrapper.findAll('input[type="checkbox"]')).toHaveLength(2);
    await wrapper.get('[data-track-id="1"]').trigger('click');
    expect(wrapper.get('[data-track-id="1"]').attributes('data-selected')).toBe('true');
    expect(wrapper.get('[data-track-id="1"]').classes()).toContain('bg-list-selected');
    expect(wrapper.get<HTMLInputElement>('[data-track-id="1"] input').element.checked).toBe(true);
    expect(toolbar.text()).toContain('已选 1 首');
    // A native checkbox changes selection once; its click must not bubble to the row.
    await wrapper.get<HTMLInputElement>('[data-track-id="2"] input').setValue(true);
    expect(toolbar.text()).toContain('已选 2 首');
    await wrapper.get('[data-track-id="1"]').trigger('dblclick');
    expect(state.player.playAll).not.toHaveBeenCalled();
    expect(state.player.playQueue).not.toHaveBeenCalled();
    expect(state.player.playTrack).not.toHaveBeenCalled();
    expect(state.player.toggleFavorite).not.toHaveBeenCalled();
    await click('取消全选');
    expect(toolbar.text()).toContain('已选 0 首');
    expect(wrapper.findAll('[data-selected="true"]')).toHaveLength(0);
    await click('全选');
    expect(toolbar.text()).toContain('已选 2 首');
    if (page.name === '喜欢的音乐') {
      await click('添加到歌单');
      const target = wrapper.findAll('button').find(item => item.text().startsWith('目标歌单'))!;
      await target.trigger('click'); await flushPromises();
      expect(state.player.batchAddToPlaylist).toHaveBeenCalledWith(88, [1, 2]);
    } else {
      await click('添加到喜欢的音乐');
      expect(state.player.batchSetFavorite).toHaveBeenCalledWith([1, 2], true);
    }
    expect(wrapper.find('[aria-label="歌曲批量操作"]').exists()).toBe(false);
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false);
  });

  it('队列含重复歌曲时按唯一曲目计数，全选状态仍正确', async () => {
    state.player.queue.push({ ...state.player.queue[0] });
    await open(pages[1]);
    await click('全选');
    expect(wrapper.get('[aria-label="歌曲批量操作"]').text()).toContain('已选 2 首');
    expect(wrapper.findAll('[data-selected="true"]')).toHaveLength(3);
    await click('取消全选');
    expect(wrapper.findAll('[data-selected="true"]')).toHaveLength(0);
  });

  it('筛选和切换列表清空旧选择，不把上个列表的歌曲送入批量操作', async () => {
    await open(pages[0]);
    await wrapper.get('[data-track-id="1"]').trigger('click');
    state.player.searchQuery = '新筛选';
    await flushPromises();
    expect(wrapper.find('[aria-label="歌曲批量操作"]').exists()).toBe(false);
    wrapper.unmount();
    await open(pages[6]);
    await wrapper.get('[data-track-id="1"]').trigger('click');
    state.player.activePlaylistId = 99;
    await flushPromises();
    expect(wrapper.find('[aria-label="歌曲批量操作"]').exists()).toBe(false);
    expect(state.player.batchSetFavorite).not.toHaveBeenCalled();
  });

  it('结果集全选请求迟到时不能覆盖已退出并重新开启的选择', async () => {
    let finish!: (ids: number[]) => void;
    invokeMock.mockImplementation((cmd: string) => cmd === 'library_get_track_ids'
      ? new Promise<number[]>(resolve => { finish = resolve; }) : Promise.resolve([]));
    state.player.searchQuery = '筛选';
    await open(pages[0]);
    await click('全选');
    state.player.searchQuery = '其他筛选';
    await flushPromises();
    await wrapper.get('[title="多选歌曲"]').trigger('click');
    await wrapper.get('[data-track-id="2"]').trigger('click');
    finish([999]);
    await flushPromises();
    expect(wrapper.get('[aria-label="歌曲批量操作"]').text()).toContain('已选 1 首');
    expect(wrapper.get('[data-track-id="2"]').attributes('data-selected')).toBe('true');
    await click('添加到喜欢的音乐');
    expect(state.player.batchSetFavorite).toHaveBeenCalledWith([2], true);
  });
});
