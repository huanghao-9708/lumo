import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import App from './App.vue';
import { useDesktopModeStore } from './stores/desktopMode';
import { usePlayerStore } from './stores/player';

const { invokeMock, handlers, show, workspaceMounted, workspaceUnmounted } = vi.hoisted(() => ({
  invokeMock: vi.fn(), handlers: new Map<string, (event: { payload: unknown }) => void>(),
  show: vi.fn(), workspaceMounted: vi.fn(), workspaceUnmounted: vi.fn(),
}));
vi.mock('./utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async (name: string, handler: (event: { payload: unknown }) => void) => { handlers.set(name, handler); return () => handlers.delete(name); } }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ show, close: vi.fn(), minimize: vi.fn() }) }));
vi.mock('./composables/useWindowPersistence', () => ({ setupWindowPersistence: async () => () => {} }));
vi.mock('./components/layout/MainContent.vue', async () => {
  const { onMounted, onUnmounted, defineComponent, h } = await import('vue');
  return { default: defineComponent({ setup() { onMounted(workspaceMounted); onUnmounted(workspaceUnmounted); return () => h('div', { 'data-workspace': '' }, '曲库'); } }) };
});
vi.mock('./components/layout/TopBar.vue', () => ({ default: { template: '<div />' } }));
vi.mock('./components/layout/SidebarLeft.vue', () => ({ default: { template: '<div />' } }));
vi.mock('./components/layout/SidebarRight.vue', () => ({ default: { template: '<div />' } }));
vi.mock('./components/layout/BottomPlayer.vue', () => ({ default: { template: '<div />' } }));
vi.mock('./components/layout/NowPlayingImmersive.vue', () => ({ default: { template: '<div />' } }));

const brief = { trackId: 7, mediaFileId: 107, title: '长歌名'.repeat(60), artist: '歌手', album: '专辑', artworkId: 123, durationMs: 180000 };
const summary = { stateVersion: 1, queueLength: 30000, currentIndex: 5, mode: 'normal', positionMs: 0, durationMs: 180000, isPlaying: false, currentTrack: brief };
let form = 'mini';
let wrapper: VueWrapper | undefined;
beforeEach(() => {
  setActivePinia(createPinia()); localStorage.clear(); handlers.clear();
  vi.stubGlobal('__TAURI_INTERNALS__', {});
  Object.defineProperty(window, 'innerWidth', { value: 560, writable: true, configurable: true });
  show.mockReset().mockResolvedValue(undefined);
  workspaceMounted.mockClear(); workspaceUnmounted.mockClear();
  form = 'mini';
  invokeMock.mockReset().mockImplementation(async (cmd: string) => {
    if (cmd === 'desktop_get_preferences') return { preferences: { schemaVersion: 1, experienceMode: 'minimal', windowForm: form, fullGeometry: null, miniGeometry: null, miniAlwaysOnTop: false }, fileExisted: true };
    if (cmd === 'playback_session_summary') return summary;
    if (cmd === 'playback_queue_state') return { items: [brief], index: 0, mode: 'normal', positionMs: 5000 };
    if (cmd === 'library_get_startup_bundle') return { counts: { tracks: 1 }, playlists: [], albums: [], artists: [], play_queue: [], album_total: 0, artist_total: 0 };
    if (cmd === 'library_get_counts') return { tracks: 1 };
    if (cmd === 'library_get_insights') return { top_played_tracks: [], recent_played_tracks: [], recent_added_tracks: [], favorite_tracks: [], top_played_artists: [], top_played_albums: [], today_play_count: 0, last_played: null };
    return [];
  });
});
afterEach(() => { wrapper?.unmount(); wrapper = undefined; vi.unstubAllGlobals(); });

describe('mini lifecycle and cold startup', () => {
  it('完整冷启动不把旧startup bundle队列当成当前播放队列', async () => {
    form = 'full';
    Object.defineProperty(window, 'innerWidth', { value: 1200, writable: true, configurable: true });
    localStorage.setItem('lumo_current_index', '1'); localStorage.setItem('lumo_progress_ms', '61000');
    const implementation = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((cmd: string, ...args: unknown[]) => {
      if (cmd === 'library_get_startup_bundle') return Promise.resolve({ counts: { tracks: 1 }, playlists: [], albums: [], artists: [],
        play_queue: [{ id: 99, title: '旧曲目', duration_sec: 200 }], album_total: 0, artist_total: 0 });
      if (cmd === 'playback_queue_state') return Promise.resolve({ items: [{ ...brief, trackId: 1 }, brief], index: 1, mode: 'repeatAll', positionMs: 0 });
      return implementation(cmd, ...args);
    });
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    const player = usePlayerStore();
    expect(player.queue.map(track => track.id)).toEqual([1, 7]);
    expect(player.currentTrack?.id).toBe(7);
    expect(player.progressMs).toBe(61000);
    expect(player.durationMs).toBe(180000);
    expect(player.playMode).toBe('repeat');
    expect(player.isPlaying).toBe(false); expect(show).toHaveBeenCalledOnce();
    expect(invokeMock.mock.calls.map(([cmd]) => cmd)).not.toContain('playback_set_queue');
    expect(invokeMock.mock.calls.map(([cmd]) => cmd)).not.toContain('playback_play_index');
  });

  it('mini shows buffering while first decoding is pending and clears it on progress', async () => {
    const implementation = invokeMock.getMockImplementation()!;
    let finishPlay!: () => void;
    invokeMock.mockImplementation((cmd: string, ...args: unknown[]) => cmd === 'playback_play_index'
      ? new Promise<void>(resolve => { finishPlay = resolve; }) : implementation(cmd, ...args));
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    await wrapper.find('[aria-label="播放"]').trigger('click');
    expect(wrapper.find('[role="status"]').text()).toContain('正在缓冲');
    finishPlay(); await flushPromises();
    handlers.get('playback-progress')?.({ payload: { position: 1000 } });
    await flushPromises();
    expect(wrapper.find('[role="status"]').text()).toBe('正在播放');
  });

  it('returning from a paused mini cold start preserves its saved resume position', async () => {
    localStorage.setItem('lumo_current_index', '5');
    localStorage.setItem('lumo_progress_ms', '60000');
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    const player = usePlayerStore();
    expect(player.progressMs).toBe(60000);
    await player.syncSessionFromBackend();
    expect(player.progressMs).toBe(60000);
    await useDesktopModeStore().exitMini(); await flushPromises();
    await player.applyBrowseRestore();
    expect(player.currentTrack?.id).toBe(7);
    expect(player.progressMs).toBe(60000);
    expect(player.isPlaying).toBe(false);
    expect(invokeMock.mock.calls.map(([cmd]) => cmd)).not.toContain('playback_play_index');
  });

  it('mini cold start at 560px mounts only text controls and never loads a full library or queue', async () => {
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.find('[aria-label="迷你播放栏"]').exists()).toBe(true);
    expect(wrapper.find('img').exists()).toBe(false);
    expect(workspaceMounted).not.toHaveBeenCalled();
    expect(show).toHaveBeenCalledOnce();
    expect(usePlayerStore().queue).toHaveLength(0);
    expect(usePlayerStore().sessionQueueLength).toBe(30000);
    const commands = invokeMock.mock.calls.map(([cmd]) => cmd);
    expect(commands).not.toContain('library_get_startup_bundle');
    expect(commands).not.toContain('playback_queue_state');
    expect(commands).not.toContain('playback_play');

    await wrapper.find('[aria-label="播放"]').trigger('click'); await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('playback_play_index', { index: 5 });
    await wrapper.find('[aria-label="下一首"]').trigger('click'); await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('playback_advance', { direction: 1 });
    const seek = wrapper.find<HTMLInputElement>('[aria-label="播放进度"]');
    await seek.setValue('60000'); await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('playback_seek', { positionMs: 60000 });
    expect(localStorage.getItem('lumo_progress_ms')).toBe('60000');
  });

  it('return preserves experience, workspace really unmounts on reentry, and controls keep their keyboard behavior', async () => {
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    const volume = wrapper.find<HTMLInputElement>('[aria-label="音量"]');
    volume.element.focus();
    const key = new KeyboardEvent('keydown', { code: 'ArrowRight', bubbles: true, cancelable: true });
    volume.element.dispatchEvent(key);
    expect(key.defaultPrevented).toBe(false);
    await wrapper.find('[aria-label="返回完整界面"]').trigger('click'); await flushPromises();
    expect(useDesktopModeStore().experienceMode).toBe('minimal');
    expect(workspaceMounted).toHaveBeenCalledOnce();
    await usePlayerStore().applyBrowseRestore();
    await useDesktopModeStore().enterMini(); await flushPromises();
    expect(workspaceUnmounted).toHaveBeenCalledOnce();
    expect(wrapper.find('[data-workspace]').exists()).toBe(false);
    expect(usePlayerStore().sessionWiredCount).toBe(1);
    for (let i = 0; i < 20; i++) {
      await useDesktopModeStore().exitMini(); await flushPromises();
      await usePlayerStore().applyBrowseRestore();
      await useDesktopModeStore().enterMini(); await flushPromises();
    }
    expect(workspaceMounted).toHaveBeenCalledTimes(21);
    expect(workspaceUnmounted).toHaveBeenCalledTimes(21);
    expect(usePlayerStore().sessionWiredCount).toBe(1);
  });

  it('an editing dialog prevents entry without dropping its draft', async () => {
    const mode = useDesktopModeStore(); await mode.init(); await mode.exitMini();
    const player = usePlayerStore(); player.isCreatePlaylistModalOpen = true;
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    await mode.exitMini(); await flushPromises();
    expect(wrapper.find('input').exists()).toBe(true);
    expect(await mode.enterMini()).toBe(false);
    expect(mode.windowForm).toBe('full');
  });

  it('empty mini queue stays idle and an error remains visible after the toast disappears', async () => {
    localStorage.setItem('lumo_current_index', '0');
    localStorage.setItem('lumo_progress_ms', '90000');
    const implementation = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((cmd: string, ...args: unknown[]) => cmd === 'playback_session_summary'
      ? Promise.resolve({ ...summary, queueLength: 0, currentIndex: 0, currentTrack: null, durationMs: null })
      : implementation(cmd, ...args));
    wrapper = mount(App, { attachTo: document.body }); await flushPromises();
    expect(wrapper.find('[aria-label="播放"]').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('队列为空');
    expect(wrapper.text()).toContain('0:00 / 0:00');
    expect(usePlayerStore().progressMs).toBe(0);
    expect(localStorage.getItem('lumo_progress_ms')).toBe('0');
    expect(invokeMock.mock.calls.map(([cmd]) => cmd)).not.toContain('source_scan');
    handlers.get('playback-error')?.({ payload: { index: 0, message: '文件不存在，请返回曲库选择音乐' } });
    await flushPromises();
    expect(wrapper.find('[role="status"]').text()).toContain('文件不存在');
  });
});
