import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises } from '@vue/test-utils';
import { useDesktopModeStore } from '../stores/desktopMode';
import { fitGeometry, setupWindowPersistence, toggleWindowMaximize } from './desktopWindow';

const { invokeMock, native, callbacks } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  native: {
    innerSize: vi.fn(), outerPosition: vi.fn(), scaleFactor: vi.fn(), isMaximized: vi.fn(), isMinimized: vi.fn(),
    setSize: vi.fn(), setPosition: vi.fn(), setSizeConstraints: vi.fn(), setAlwaysOnTop: vi.fn(),
    unmaximize: vi.fn(), maximize: vi.fn(), center: vi.fn(),
    onResized: vi.fn(), onMoved: vi.fn(), onScaleChanged: vi.fn(),
  }, callbacks: new Map<string, () => void>(),
}));
vi.mock('../utils/tauriInvoke', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => native,
  LogicalSize: class { constructor(public width: number, public height: number) {} },
  LogicalPosition: class { constructor(public x: number, public y: number) {} },
  availableMonitors: async () => [{ scaleFactor: 1.5, workArea: { position: { x: 0, y: 0 }, size: { width: 2560, height: 1440 } } }],
  currentMonitor: async () => null,
}));

let dispose: (() => void) | undefined;
let size = { width: 1800, height: 1080 };
let position = { x: 150, y: 150 };

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal('__TAURI_INTERNALS__', {});
  setActivePinia(createPinia());
  localStorage.clear();
  callbacks.clear();
  size = { width: 1800, height: 1080 };
  position = { x: 150, y: 150 };
  Object.values(native).forEach(fn => fn.mockReset().mockResolvedValue(undefined));
  native.innerSize.mockImplementation(async () => size);
  native.outerPosition.mockImplementation(async () => position);
  native.scaleFactor.mockResolvedValue(1.5);
  native.isMaximized.mockResolvedValue(false); // Simulate M0 tao false after maximize.
  native.isMinimized.mockResolvedValue(false);
  native.maximize.mockImplementation(async () => { size = { width: 2560, height: 1440 }; });
  native.setSize.mockImplementation(async (next: { width: number; height: number }) => {
    size = { width: next.width * 1.5, height: next.height * 1.5 };
    callbacks.get('resize')?.();
  });
  native.setPosition.mockImplementation(async (next: { x: number; y: number }) => {
    position = { x: next.x * 1.5, y: next.y * 1.5 };
    callbacks.get('move')?.();
  });
  for (const [key, fn] of [['resize', native.onResized], ['move', native.onMoved], ['scale', native.onScaleChanged]] as const) {
    fn.mockImplementation(async (callback: () => void) => { callbacks.set(key, callback); return () => callbacks.delete(key); });
  }
  invokeMock.mockReset().mockImplementation(async (cmd: string) => cmd === 'desktop_get_preferences' ? {
    preferences: { schemaVersion: 1, experienceMode: 'normal', windowForm: 'full', fullGeometry: { x: 100, y: 100, width: 1200, height: 720, maximized: false }, miniGeometry: null, miniAlwaysOnTop: true }, fileExisted: true,
  } : undefined);
});
afterEach(() => { dispose?.(); dispose = undefined; vi.useRealTimers(); vi.unstubAllGlobals(); });

describe('desktop window transactions', () => {
  it.each([true, false])('OS restore clears the owned maximized fallback, delayed save=%s', async delayedSave => {
    const mode = useDesktopModeStore(); await mode.init();
    dispose = await setupWindowPersistence();
    await toggleWindowMaximize(); await mode.enterMini(); await mode.exitMini();
    expect(native.maximize).toHaveBeenCalledTimes(2);
    // Native OS Restore bypasses toggleWindowMaximize; tao still reports false.
    size = { width: 1800, height: 1080 };
    position = { x: 150, y: 150 };
    callbacks.get('resize')?.();
    if (delayedSave) { await vi.advanceTimersByTimeAsync(500); await flushPromises(); }
    await mode.enterMini();
    expect(mode.fullGeometry).toMatchObject({ width: 1200, height: 720, maximized: false });
    await mode.exitMini();
    expect(size).toEqual({ width: 1800, height: 1080 });
    expect(native.maximize).toHaveBeenCalledTimes(2);
  });

  it('maximized full → mini → full restores intent despite stale native flag and cancels old resize saves', async () => {
    const mode = useDesktopModeStore();
    await mode.init();
    dispose = await setupWindowPersistence();
    callbacks.get('resize')?.(); // Old delayed full save must not run during mini transition.
    await toggleWindowMaximize();
    expect(await mode.enterMini()).toBe(true);
    expect(size).toEqual({ width: 840, height: 144 });
    expect(mode.fullGeometry?.maximized).toBe(true);
    expect(native.setAlwaysOnTop).toHaveBeenLastCalledWith(true);
    await vi.advanceTimersByTimeAsync(1000);
    expect(mode.fullGeometry?.width).toBe(1200);
    expect(await mode.exitMini()).toBe(true);
    expect(mode.windowForm).toBe('full');
    expect(native.maximize).toHaveBeenCalledTimes(2);
    expect(native.setAlwaysOnTop).toHaveBeenLastCalledWith(false);
  });

  it('API failure rolls back geometry without saving a successful mini preference', async () => {
    const mode = useDesktopModeStore();
    await mode.init();
    dispose = await setupWindowPersistence();
    native.setPosition.mockRejectedValueOnce(new Error('position denied'));
    invokeMock.mockClear();
    expect(await mode.enterMini()).toBe(false);
    expect(mode.windowForm).toBe('full');
    expect(size).toEqual({ width: 1800, height: 1080 });
    expect(mode.phase).toBe('idle');
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'desktop_update_preferences')).toHaveLength(0);
  });

  it('mini move saves only mini geometry and cleanup cancels callbacks and timers', async () => {
    const mode = useDesktopModeStore();
    await mode.init();
    dispose = await setupWindowPersistence();
    await mode.enterMini();
    const full = { ...mode.fullGeometry! };
    position = { x: 450, y: 300 };
    callbacks.get('move')?.();
    await vi.advanceTimersByTimeAsync(500);
    await flushPromises();
    expect(mode.miniGeometry?.x).toBe(300);
    expect(mode.fullGeometry).toEqual(full);
    callbacks.get('resize')?.();
    const count = invokeMock.mock.calls.length;
    dispose(); dispose = undefined;
    await vi.advanceTimersByTimeAsync(1000);
    expect(invokeMock.mock.calls.length).toBe(count);
    expect(callbacks.size).toBe(0);
  });

  it.each([1, 1.25, 1.5, 2])('fits mini controls inside a removed-monitor fallback at scale %s', scale => {
    const g = fitGeometry({ x: -4000, y: 8000, width: 100, height: 10, maximized: true },
      { x: 0, y: 0, w: 1920 / scale, h: 1080 / scale }, 'mini');
    expect(g.width).toBe(480);
    expect(g.height).toBe(96);
    expect(g.x).toBe(0);
    expect(g.y + g.height).toBeLessThanOrEqual(1080 / scale + 1);
    expect(g.maximized).toBe(false);
  });
});
