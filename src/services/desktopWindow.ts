import { availableMonitors, currentMonitor, getCurrentWindow, LogicalPosition, LogicalSize, type Monitor } from '@tauri-apps/api/window';
import { useDesktopModeStore, type DesktopPreferences, type WindowForm, type WindowGeometry } from '../stores/desktopMode';

export const WINDOW_DEFAULT_WIDTH = 1200;
export const WINDOW_DEFAULT_HEIGHT = 720;
export const WINDOW_MIN_WIDTH = 1024;
export const WINDOW_MIN_HEIGHT = 640;
export const MINI_DEFAULT_WIDTH = 560;
export const MINI_MIN_WIDTH = 480;
export const MINI_HEIGHT = 96;

type WorkArea = { x: number; y: number; w: number; h: number };
const hasNativeWindow = () => typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

export function clampToWorkArea(
  target: { w: number; h: number }, area: { w: number; h: number } | null, form: WindowForm = 'full',
): { w: number; h: number } {
  const width = Math.max(form === 'mini' ? MINI_MIN_WIDTH : WINDOW_MIN_WIDTH, target.w);
  const height = form === 'mini' ? MINI_HEIGHT : Math.max(WINDOW_MIN_HEIGHT, target.h);
  return { w: Math.round(area ? Math.min(width, area.w) : width), h: Math.round(area ? Math.min(height, area.h) : height) };
}

function monitorArea(m: Monitor): WorkArea {
  return {
    x: m.workArea.position.x / m.scaleFactor, y: m.workArea.position.y / m.scaleFactor,
    w: m.workArea.size.width / m.scaleFactor, h: m.workArea.size.height / m.scaleFactor,
  };
}

/** Keep every control on screen after DPI changes or unplugging a monitor. */
export function fitGeometry(g: WindowGeometry, area: WorkArea | null, form: WindowForm): WindowGeometry {
  const size = clampToWorkArea({ w: g.width, h: g.height }, area, form);
  return {
    ...g, width: size.w, height: size.h,
    x: Math.round(area ? Math.max(area.x, Math.min(g.x, area.x + area.w - size.w)) : g.x),
    y: Math.round(area ? Math.max(area.y, Math.min(g.y, area.y + area.h - size.h)) : g.y),
    maximized: form === 'full' && g.maximized,
  };
}

let maximizeAction: (() => Promise<void>) | null = null;
export async function toggleWindowMaximize(): Promise<void> {
  await maximizeAction?.();
}

/** One owner for constraints, transitions and both saved geometries. Playback is never touched. */
export async function setupWindowPersistence(): Promise<() => void> {
  if (!hasNativeWindow()) return () => {};
  const mode = useDesktopModeStore();
  const win = getCurrentWindow();
  let disposed = false;
  let locked = true;
  let epoch = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let fullMaximized = mode.fullGeometry?.maximized ?? false;
  let lastFull = mode.fullGeometry;
  let constraintsKey = '';
  const unlisteners: Array<() => void> = [];
  let geometryTask: Promise<void> | null = null;

  function freeze() {
    locked = true;
    epoch++;
    if (timer) clearTimeout(timer);
    timer = null;
  }

  async function readGeometry(form: WindowForm): Promise<WindowGeometry> {
    const [size, position, scale, maximized] = await Promise.all([
      win.innerSize(), win.outerPosition(), win.scaleFactor(), win.isMaximized(),
    ]);
    if (form === 'full' && (fullMaximized || maximized) && lastFull) {
      return { ...lastFull, maximized: true };
    }
    return {
      x: Math.round(position.x / scale), y: Math.round(position.y / scale),
      width: Math.round(size.width / scale), height: Math.round(size.height / scale),
      maximized: form === 'full' && (fullMaximized || maximized),
    };
  }

  async function areaFor(g: WindowGeometry | null): Promise<WorkArea | null> {
    const monitors = await availableMonitors();
    const match = g && monitors.find(m => {
      const a = monitorArea(m);
      return g.x >= a.x && g.x < a.x + a.w && g.y >= a.y && g.y < a.y + a.h;
    });
    const monitor = match || await currentMonitor() || monitors[0];
    return monitor ? monitorArea(monitor) : null;
  }

  async function setConstraints(form: WindowForm, area: WorkArea | null) {
    const constraints = {
      minWidth: Math.min(form === 'mini' ? MINI_MIN_WIDTH : WINDOW_MIN_WIDTH, area?.w ?? Infinity),
      minHeight: Math.min(form === 'mini' ? MINI_HEIGHT : WINDOW_MIN_HEIGHT, area?.h ?? Infinity),
      maxHeight: form === 'mini' ? Math.min(MINI_HEIGHT, area?.h ?? Infinity) : undefined,
    };
    const key = JSON.stringify(constraints);
    if (key === constraintsKey) return;
    await win.setSizeConstraints(constraints);
    constraintsKey = key;
  }

  async function applyGeometry(form: WindowForm, saved: WindowGeometry | null, onTop: boolean): Promise<WindowGeometry> {
    const area = await areaFor(saved);
    const target = fitGeometry(saved ?? {
      x: area ? area.x + (area.w - (form === 'mini' ? MINI_DEFAULT_WIDTH : WINDOW_DEFAULT_WIDTH)) / 2 : 0,
      y: area ? area.y + (area.h - (form === 'mini' ? MINI_HEIGHT : WINDOW_DEFAULT_HEIGHT)) / 2 : 0,
      width: form === 'mini' ? MINI_DEFAULT_WIDTH : WINDOW_DEFAULT_WIDTH,
      height: form === 'mini' ? MINI_HEIGHT : WINDOW_DEFAULT_HEIGHT, maximized: false,
    }, area, form);
    // M0 tao seam: never trust isMaximized just after changing window styles.
    await win.unmaximize();
    await setConstraints(form, area);
    await win.setSize(new LogicalSize(target.width, target.height));
    await win.setPosition(new LogicalPosition(target.x, target.y));
    await win.setAlwaysOnTop(form === 'mini' && onTop);
    if (form === 'full') {
      lastFull = target;
      fullMaximized = target.maximized;
      if (target.maximized) await win.maximize();
    }
    return target;
  }

  const driver = {
    async changeForm(from: WindowForm, to: WindowForm, prefs: DesktopPreferences) {
      freeze();
      await geometryTask;
      locked = true;
      let previousGeometry: WindowGeometry | null = null;
      try {
        previousGeometry = await readGeometry(from);
        const target = to === 'mini' ? prefs.miniGeometry : prefs.fullGeometry;
        const nextGeometry = await applyGeometry(to, target, prefs.miniAlwaysOnTop);
        return { previousGeometry, nextGeometry };
      } catch (error) {
        const previous = previousGeometry ?? (from === 'full' ? prefs.fullGeometry : prefs.miniGeometry);
        try { await applyGeometry(from, previous, prefs.miniAlwaysOnTop); }
        catch (rollbackError) {
          console.error('[window] 窗口回退受限', rollbackError);
          const size = from === 'mini' ? { width: MINI_DEFAULT_WIDTH, height: MINI_HEIGHT } : { width: WINDOW_DEFAULT_WIDTH, height: WINDOW_DEFAULT_HEIGHT };
          for (const recover of [
            () => win.unmaximize(), () => win.setSizeConstraints(null),
            () => win.setSize(new LogicalSize(size.width, size.height)),
            () => win.center(), () => win.setAlwaysOnTop(from === 'mini' && prefs.miniAlwaysOnTop),
          ]) { try { await recover(); } catch { /* Continue other recovery operations. */ } }
          constraintsKey = '';
        }
        throw error;
      } finally { locked = false; }
    },
    setAlwaysOnTop: (on: boolean) => win.setAlwaysOnTop(on),
  };

  async function recordAndClamp() {
    if (disposed || locked || mode.phase !== 'idle') return;
    const myEpoch = epoch;
    const form = mode.windowForm;
    try {
      if (await win.isMinimized()) return;
      const geometry = await readGeometry(form);
      if (disposed || locked || myEpoch !== epoch || mode.phase !== 'idle' || form !== mode.windowForm) return;
      if (!geometry.maximized) {
        const area = await areaFor(geometry);
        if (disposed || locked || myEpoch !== epoch || mode.phase !== 'idle') return;
        const fitted = fitGeometry(geometry, area, form);
        locked = true;
        try {
          await setConstraints(form, area);
          if (fitted.width !== geometry.width || fitted.height !== geometry.height) {
            await win.setSize(new LogicalSize(fitted.width, fitted.height));
          }
          if (fitted.x !== geometry.x || fitted.y !== geometry.y) {
            await win.setPosition(new LogicalPosition(fitted.x, fitted.y));
          }
          Object.assign(geometry, fitted);
        } finally { locked = false; }
      }
      if (myEpoch !== epoch || mode.phase !== 'idle') return;
      if (form === 'full') {
        if (!geometry.maximized) lastFull = geometry;
        await mode.updateFullGeometry(geometry);
      } else await mode.updateMiniGeometry(geometry);
    } catch (error) { console.warn('[window] 窗口位置保存失败', error); }
  }
  function schedule() {
    if (disposed || locked || mode.phase !== 'idle') return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      geometryTask = recordAndClamp().finally(() => { geometryTask = null; });
    }, 400);
  }

  try {
    mode.attachWindowDriver(driver);
    await applyGeometry(mode.windowForm, mode.windowForm === 'mini' ? mode.miniGeometry : mode.fullGeometry, mode.miniAlwaysOnTop);
    unlisteners.push(await win.onResized(schedule));
    unlisteners.push(await win.onMoved(schedule));
    unlisteners.push(await win.onScaleChanged(schedule));
    mode.attachWindowDriver(driver);
    maximizeAction = async () => {
      if (locked || mode.phase !== 'idle' || mode.windowForm !== 'full') return;
      freeze();
      await geometryTask;
      locked = true;
      try {
        if (fullMaximized || await win.isMaximized()) {
          await win.unmaximize();
          fullMaximized = false;
          await applyGeometry('full', lastFull ? { ...lastFull, maximized: false } : null, false);
        } else {
          lastFull = await readGeometry('full');
          await win.maximize();
          fullMaximized = true;
        }
      } catch (e) { console.warn('[window] 最大化切换失败', e); }
      finally { locked = false; schedule(); }
    };
  } catch (error) {
    unlisteners.forEach(unlisten => unlisten());
    throw error;
  } finally { locked = false; }

  return () => {
    disposed = true;
    freeze();
    unlisteners.forEach(unlisten => unlisten());
    mode.attachWindowDriver(null);
    maximizeAction = null;
  };
}
