import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, disposePinia, setActivePinia, type Pinia } from 'pinia';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import HomeView from './HomeView.vue';
import { useAiStore } from '../../stores/ai';
import { usePlayerStore } from '../../stores/player';
import type { AiSettingsDTO } from '../../api/ai';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('../../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

describe('首页AI电台入口服从设置开关', () => {
  let wrapper: VueWrapper;
  let pinia: Pinia;
  let savedSettings: AiSettingsDTO;
  beforeEach(() => {
    localStorage.clear();
    pinia = createPinia();
    setActivePinia(pinia);
    savedSettings = { enabled: false, base_url: '', model: '', temperature: 0.8, has_key: false };
    invokeMock.mockReset().mockImplementation(async (cmd: string, args: { enabled: boolean }) => {
      if (cmd === 'ai_get_settings') return { ...savedSettings };
      if (cmd === 'ai_save_settings') {
        savedSettings = { ...savedSettings, enabled: args.enabled };
        return { ...savedSettings };
      }
      if (cmd === 'library_get_stats') return null;
      if (cmd === 'library_get_insights') return {
        top_played_tracks: [], recent_played_tracks: [], recent_added_tracks: [], favorite_tracks: [],
        top_played_artists: [], top_played_albums: [], today_play_count: 0, last_played: null,
      };
      return [];
    });
  });
  afterEach(async () => {
    wrapper?.unmount();
    await flushPromises();
    disposePinia(pinia);
  });

  const entry = () => wrapper.findAll('button').find(button => button.text().includes('AI 电台'));

  it('冷启动设置读取期间及已关闭时隐藏入口，统计卡仍正常显示', async () => {
    let finish!: (settings: AiSettingsDTO) => void;
    const implementation = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((cmd: string, args: { enabled: boolean }) => cmd === 'ai_get_settings'
      ? new Promise<AiSettingsDTO>(resolve => { finish = resolve; }) : implementation(cmd, args));
    wrapper = mount(HomeView);
    await flushPromises();
    expect(entry()).toBeUndefined();
    expect(wrapper.text()).toContain('累计听歌时长');
    finish(savedSettings);
    await flushPromises();
    expect(entry()).toBeUndefined();
  });

  it('保存关闭后立即移除入口，重新启用恢复且仍能进入电台', async () => {
    savedSettings.enabled = true;
    wrapper = mount(HomeView);
    await flushPromises();
    expect(entry()).toBeDefined();
    const ai = useAiStore();
    expect(await ai.saveSettings({ enabled: false })).toBe(true);
    await flushPromises();
    expect(entry()).toBeUndefined();
    expect(await ai.saveSettings({ enabled: true })).toBe(true);
    await flushPromises();
    await entry()!.trigger('click');
    expect(usePlayerStore().activeLibraryTab).toBe('AI 电台');
  });

  it('已加载的已关闭设置直接使用，不先展示入口或重复读取', async () => {
    await useAiStore().fetchSettings();
    invokeMock.mockClear();
    wrapper = mount(HomeView);
    await flushPromises();
    expect(entry()).toBeUndefined();
    expect(invokeMock.mock.calls.some(([cmd]) => cmd === 'ai_get_settings')).toBe(false);
  });
});
