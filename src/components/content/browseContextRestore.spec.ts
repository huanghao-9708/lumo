import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { usePlayerStore } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';
import { resetScrollPosition } from '../../composables/useScrollRestore';
import FolderView from './FolderView.vue';
import SmartPlaylistView from './SmartPlaylistView.vue';
import ArtistDetail from './ArtistDetail.vue';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

describe('迷你返回：目录、智能歌单和艺术家详情上下文', () => {
  let pinia: Pinia;
  let wrapper: VueWrapper | undefined;
  let sourceIds: number[];
  let albumCount: number;
  const track = (id: number) => ({ id, title: `歌曲${id}`, artist_name: '艺术家', album_title: '专辑', duration_ms: 180000 });
  const children = (sourceId: number, path?: string) => ({ source_root: `盘${sourceId}`, children: path ? [
    { path: `${path}\\Child`, name: `子目录${sourceId}`, audio_count: 1, has_subdirs: false },
  ] : [{ path: `盘${sourceId}\\Music`, name: `音乐目录${sourceId}`, audio_count: 1, has_subdirs: true }] });

  beforeEach(() => {
    pinia = createPinia(); setActivePinia(pinia); localStorage.clear(); resetScrollPosition();
    sourceIds = [1, 2]; albumCount = 50;
    vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
    invokeMock.mockReset().mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'desktop_get_preferences') return { preferences: { schemaVersion: 1, experienceMode: 'minimal', windowForm: 'full' }, fileExisted: true };
      if (cmd === 'source_list') return sourceIds.map(id => ({ id, name: `盘${id}`, kind: 'local', enabled: true, root_uri: `盘${id}`, config_json: '{}' }));
      if (cmd === 'library_get_folder_children') return children(args.sourceId, args.folderPath);
      if (cmd === 'library_get_folder_tracks') return { tracks: [track(args.sourceId)], total: 1 };
      if (cmd === 'library_get_smart_playlist') return [track(9)];
      if (cmd === 'library_get_artist_by_id') return { id: args.artistId, name: '艺术家', track_count: 1 };
      if (cmd === 'library_get_artist_stats') return { track_count: 1, album_count: albumCount };
      if (cmd === 'library_get_artist_tracks') return [track(7)];
      if (cmd === 'library_get_artist_album_count') return albumCount;
      if (cmd === 'library_get_artist_albums') return Array.from({ length: Math.max(0, Math.min(args.limit, albumCount - args.offset)) }, (_, i) => ({
        id: args.offset + i + 100, title: `专辑${args.offset + i + 1}`, track_count: 10,
      }));
      if (cmd === 'library_get_album_count') return 0;
      if (cmd === 'library_get_artists') return { artists: [], total: 0 };
      if (cmd === 'library_get_counts') return { tracks: 0 };
      return [];
    });
  });
  afterEach(async () => { wrapper?.unmount(); wrapper = undefined; await flushPromises(); disposePinia(pinia); vi.unstubAllGlobals(); });

  async function enterAndRemount(component: typeof FolderView | typeof SmartPlaylistView) {
    const mode = useDesktopModeStore();
    await mode.enterMini(); wrapper!.unmount(); wrapper = undefined;
    await mode.exitMini(); invokeMock.mockClear();
    wrapper = mount(component); await flushPromises();
  }

  it('文件夹返回保留第二个数据源、展开路径和选中目录，仅重读当前目录与展开树', async () => {
    await useDesktopModeStore().init(); const player = usePlayerStore();
    player.activeLibraryTab = '文件夹'; await player.fetchSources();
    wrapper = mount(FolderView); await flushPromises();
    await wrapper.get('select').setValue('2'); await flushPromises();
    expect(player.folderBrowseContext.sourceId).toBe(2);
    const node = wrapper.findAll('span').find(span => span.text() === '音乐目录2')!;
    await node.trigger('click'); await flushPromises();
    expect(player.folderBrowseContext.expandedPaths).toEqual({ '盘2\\Music': true });
    expect(player.selectedTreePath).toBe('盘2\\Music');
    await enterAndRemount(FolderView);
    expect(invokeMock.mock.calls.some(([cmd]) => cmd === 'library_get_folder_children')).toBe(false);
    expect(player.folderTracks).toHaveLength(0);
    await player.applyBrowseRestore(); await flushPromises();
    expect(wrapper.get<HTMLSelectElement>('select').element.value).toBe('2');
    expect(wrapper.text()).toContain('音乐目录2'); expect(wrapper.text()).toContain('子目录2');
    expect(player.folderTracks.map(item => item.id)).toEqual([2]);
    expect(player.selectedTreePath).toBe('盘2\\Music');
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'library_get_folder_tracks').map(([, args]) => args)).toEqual([
      { sourceId: 2, folderPath: '盘2\\Music', limit: 100, offset: 0 },
    ]);
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'library_get_folder_children').map(([, args]) => args.sourceId)).toEqual([2, 2]);
  });

  it('普通组件重挂也恢复目录展开，目录对象不常驻 store', async () => {
    const player = usePlayerStore(); await player.fetchSources();
    player.folderBrowseContext = { sourceId: 2, expandedPaths: { '盘2\\Music': true } };
    wrapper = mount(FolderView); await flushPromises();
    wrapper.unmount(); wrapper = mount(FolderView); await flushPromises();
    expect(wrapper.text()).toContain('子目录2');
    expect(player.folderTreeChildren).toHaveLength(0);
  });

  it('数据源被移除时选择可用来源并清空旧路径，不请求已删除来源', async () => {
    await useDesktopModeStore().init(); const player = usePlayerStore();
    await player.fetchSources(); player.activeLibraryTab = '文件夹';
    await player.fetchFolderTracks(2, '盘2\\Music', true);
    player.folderBrowseContext.expandedPaths = { '盘2\\Music': true };
    wrapper = mount(FolderView); await flushPromises(); await enterAndRemount(FolderView);
    sourceIds = [1]; await player.applyBrowseRestore(); await flushPromises();
    expect(player.folderBrowseContext).toEqual({ sourceId: 1, expandedPaths: {} });
    expect(player.selectedTreePath).toBeNull(); expect(player.folderTracks).toHaveLength(0);
    expect(invokeMock.mock.calls.some(([cmd, args]) => cmd === 'library_get_folder_tracks' || (cmd === 'library_get_folder_children' && args.sourceId === 2))).toBe(false);
  });

  it('切来源后的旧目录响应不覆盖新目录树', async () => {
    const player = usePlayerStore(); await player.fetchSources();
    const implementation = invokeMock.getMockImplementation()!;
    let finish!: (value: unknown) => void;
    invokeMock.mockImplementation((cmd: string, args: any) => cmd === 'library_get_folder_children' && args.sourceId === 1
      ? new Promise(resolve => { finish = resolve; }) : implementation(cmd, args));
    wrapper = mount(FolderView); await flushPromises();
    await wrapper.get('select').setValue('2'); await flushPromises();
    finish(children(1)); await flushPromises();
    expect(wrapper.text()).toContain('音乐目录2'); expect(wrapper.text()).not.toContain('音乐目录1');
  });

  it('切目录后迟到的旧歌曲响应和 finally 都不能清掉新请求的状态', async () => {
    const player = usePlayerStore(); const readers: ((value: unknown) => void)[] = [];
    invokeMock.mockImplementation((cmd: string) => cmd === 'library_get_folder_tracks' ? new Promise(resolve => readers.push(resolve)) : Promise.resolve([]));
    const oldRead = player.fetchFolderTracks(1, '旧目录', true);
    const newRead = player.fetchFolderTracks(1, '新目录', true);
    readers[0]({ tracks: [track(1)], total: 1 }); await oldRead;
    expect(player.isLoadingFolderTracks).toBe(true); expect(player.folderTracks).toHaveLength(0);
    readers[1]({ tracks: [track(2)], total: 1 }); await newRead;
    expect(player.selectedTreePath).toBe('新目录'); expect(player.folderTracks.map(item => item.id)).toEqual([2]);
    expect(player.isLoadingFolderTracks).toBe(false);
  });

  it('智能歌单在迷你期间释放数据，返回重读同一类型并恢复歌曲', async () => {
    await useDesktopModeStore().init(); const player = usePlayerStore();
    await player.loadSmartPlaylist('never_played'); wrapper = mount(SmartPlaylistView); await flushPromises();
    await enterAndRemount(SmartPlaylistView); expect(player.smartPlaylistTracks).toHaveLength(0);
    await player.applyBrowseRestore(); await flushPromises();
    expect(wrapper.get('h1').text()).toBe('未曾播放'); expect(wrapper.text()).toContain('歌曲9');
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'library_get_smart_playlist').map(([, args]) => args)).toEqual([{ kind: 'never_played', limit: 100 }]);
  });

  it('连续切换智能歌单时旧失败不会清除新数据或提前结束加载', async () => {
    const player = usePlayerStore(); const readers: { resolve: (value: unknown) => void; reject: (error: Error) => void }[] = [];
    invokeMock.mockImplementation((cmd: string) => cmd === 'library_get_smart_playlist' ? new Promise((resolve, reject) => readers.push({ resolve, reject })) : Promise.resolve([]));
    const oldRead = player.loadSmartPlaylist('most_played'); const newRead = player.loadSmartPlaylist('never_played');
    const logging = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      readers[0].reject(new Error('旧请求失败')); await oldRead;
      expect(player.isLoadingSmartPlaylist).toBe(true);
      readers[1].resolve([track(2)]); await newRead;
      expect(player.smartPlaylistTracks.map(item => item.id)).toEqual([2]);
      expect(player.activeSmartPlaylistKind).toBe('never_played');
    } finally { logging.mockRestore(); }
  });

  it.each([false, true])('艺术家返回恢复专辑分栏、筛选和第3页，列表缩短=%s', async shrunk => {
    const mode = useDesktopModeStore(); await mode.init(); const player = usePlayerStore();
    player.activeLibraryTab = '艺术家'; player.activeArtistId = 7; await flushPromises();
    player.setArtistDetailSubTab('albums'); await player.goToArtistAlbumsPage(3); player.artistDetailFilterQuery = '歌曲7';
    wrapper = mount(ArtistDetail, { props: { artistId: 7 } }); await flushPromises();
    await mode.enterMini(); wrapper.unmount(); wrapper = undefined;
    if (shrunk) albumCount = 16;
    expect(player.currentArtistDetails).toBeNull(); await mode.exitMini(); invokeMock.mockClear();
    wrapper = mount(ArtistDetail, { props: { artistId: 7 } });
    await player.applyBrowseRestore(); await flushPromises();
    expect(player.currentArtistDetails?.subTab).toBe('albums');
    expect(player.artistDetailFilterQuery).toBe('歌曲7');
    expect(player.currentArtistDetails?.albumsCurrentPage).toBe(shrunk ? 2 : 3);
    expect(wrapper.get('[data-artist-minimal-albums]').text()).toContain(shrunk ? '专辑16' : '专辑31');
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'library_get_artist_albums').map(([, args]) => args.offset)).toEqual(shrunk ? [30, 15] : [30]);
    player.activeArtistId = 8; await flushPromises(); expect(player.artistDetailFilterQuery).toBe('');
  });

  it.each(['album', 'artist', 'playlist'] as const)('原%s被删除后回退列表/队列，不留下空白详情', async kind => {
    const mode = useDesktopModeStore(); await mode.init(); const player = usePlayerStore();
    player.activeLibraryTab = kind === 'album' ? '专辑' : kind === 'artist' ? '艺术家' : '播放列表';
    if (kind === 'album') player.activeAlbumId = 7;
    if (kind === 'artist') player.activeArtistId = 7;
    if (kind === 'playlist') player.activePlaylistId = 7;
    await flushPromises(); await mode.enterMini(); await mode.exitMini();
    const implementation = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((cmd: string, args: any) => cmd === `library_get_${kind}_by_id` ? Promise.resolve(null) : implementation(cmd, args));
    await player.applyBrowseRestore(); await flushPromises();
    expect(player.activeAlbumId).toBeNull(); expect(player.activeArtistId).toBeNull(); expect(player.activePlaylistId).toBeNull();
    expect(player.pendingBrowseRestore).toBe(false);
  });
});
