import { currentMonitor, getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

/**
 * 桌面窗口尺寸持久化与显示器限幅（LDL v2 空间规范 §9）。
 *
 * 目标：
 * 1. 新装/无历史时打开为默认 1200×720（tauri.conf.json 中的静态配置）。
 * 2. 记住用户主动调整后的尺寸（最大化状态不记），下次启动恢复。
 * 3. 尺寸不得超过当前显示器可用工作区；工作区小于目标时按工作区收敛，
 *    优先保证整个窗口完整可见（含任务栏留白）。
 *
 * 仅在 Tauri WebView 环境执行；浏览器 dev（无 __TAURI_INTERNALS__）直接跳过。
 */

const STORAGE_KEY = 'lumo_window_size';

/** 默认窗口（CSS 逻辑像素），与 tauri.conf.json 保持一致 */
export const WINDOW_DEFAULT_WIDTH = 1200;
export const WINDOW_DEFAULT_HEIGHT = 720;
/** 建议最小尺寸（CSS 逻辑像素），与 tauri.conf.json minWidth/minHeight 保持一致 */
export const WINDOW_MIN_WIDTH = 1024;
export const WINDOW_MIN_HEIGHT = 640;

interface SavedWindowSize {
  w: number;
  h: number;
}

function loadSavedSize(): SavedWindowSize | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as SavedWindowSize;
    if (
      typeof parsed?.w === 'number' && parsed.w > 0 && Number.isFinite(parsed.w) &&
      typeof parsed?.h === 'number' && parsed.h > 0 && Number.isFinite(parsed.h)
    ) {
      return { w: Math.round(parsed.w), h: Math.round(parsed.h) };
    }
  } catch {
    // 损坏的持久化数据按无历史处理
  }
  return null;
}

function saveSize(w: number, h: number) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ w: Math.round(w), h: Math.round(h) }));
  } catch {
    // localStorage 不可用时静默放弃（不影响窗口功能）
  }
}

/** 把目标尺寸夹进 [最小尺寸, 工作区] 区间；工作区比最小值还小时以工作区为准 */
export function clampToWorkArea(
  target: { w: number; h: number },
  workArea: { w: number; h: number } | null,
): { w: number; h: number } {
  let w = Math.max(WINDOW_MIN_WIDTH, target.w);
  let h = Math.max(WINDOW_MIN_HEIGHT, target.h);
  if (workArea) {
    w = Math.min(w, Math.max(320, workArea.w));
    h = Math.min(h, Math.max(240, workArea.h));
    // 工作区不足以容纳最小尺寸时，让窗口完整可见优先于最小尺寸
    w = Math.min(w, workArea.w);
    h = Math.min(h, workArea.h);
  }
  return { w: Math.round(w), h: Math.round(h) };
}

/** 启动时恢复/限幅 + 监听后续变化。返回清理函数（App 卸载时调用）。 */
export async function setupWindowPersistence(): Promise<() => void> {
  if (typeof window === 'undefined' || !(window as any).__TAURI_INTERNALS__) {
    return () => {};
  }

  const win = getCurrentWindow();

  // 1. 启动限幅与恢复：以当前显示器工作区为上界
  try {
    const scaleFactor = await win.scaleFactor();
    const monitor = await currentMonitor();
    const workArea = monitor
      ? {
        w: Math.round(monitor.workArea.size.width / scaleFactor),
        h: Math.round(monitor.workArea.size.height / scaleFactor),
      }
      : null;

    const target = clampToWorkArea(loadSavedSize() ?? { w: WINDOW_DEFAULT_WIDTH, h: WINDOW_DEFAULT_HEIGHT }, workArea);
    const inner = await win.innerSize();
    const currentLogical = { w: inner.width / scaleFactor, h: inner.height / scaleFactor };
    // 与目标差异超过 1px 才调整，避免启动时无谓的抖动
    if (Math.abs(currentLogical.w - target.w) > 1 || Math.abs(currentLogical.h - target.h) > 1) {
      await win.setSize(new LogicalSize(target.w, target.h));
      await win.center();
    }
  } catch (e) {
    console.warn('[window] 启动限幅失败，使用 tauri.conf.json 静态配置', e);
  }

  // 2. 记住用户调整后的尺寸（防抖；最大化/最小化不记）
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  const unlistenResized = await win.onResized(async () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      try {
        if (await win.isMaximized()) return;
        const scaleFactor = await win.scaleFactor();
        const size = await win.innerSize();
        const w = Math.round(size.width / scaleFactor);
        const h = Math.round(size.height / scaleFactor);
        if (w > 0 && h > 0) saveSize(w, h);
      } catch {
        // 窗口正在关闭等场景下读取失败，忽略
      }
    }, 400);
  });

  // 3. 显示器/缩放变化时重新约束（换屏、DPI 变更都会触发 scale-changed 或 resize）
  const unlistenScale = await win.onScaleChanged(async () => {
    try {
      const scaleFactor = await win.scaleFactor();
      const monitor = await currentMonitor();
      const workArea = monitor
        ? {
          w: Math.round(monitor.workArea.size.width / scaleFactor),
          h: Math.round(monitor.workArea.size.height / scaleFactor),
        }
        : null;
      const size = await win.innerSize();
      const clamped = clampToWorkArea(
        { w: size.width / scaleFactor, h: size.height / scaleFactor },
        workArea,
      );
      if (
        Math.abs(size.width / scaleFactor - clamped.w) > 1 ||
        Math.abs(size.height / scaleFactor - clamped.h) > 1
      ) {
        await win.setSize(new LogicalSize(clamped.w, clamped.h));
      }
    } catch {
      // 忽略瞬时错误
    }
  });

  return () => {
    if (saveTimer) clearTimeout(saveTimer);
    unlistenResized();
    unlistenScale();
  };
}
