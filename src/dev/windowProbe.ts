import { getCurrentWindow, currentMonitor, availableMonitors, LogicalSize } from '@tauri-apps/api/window';

/**
 * M0 临时窗口 API 探针（desktop-modes 02 §3，探针不作为已发布功能）。
 *
 * 验证单窗口 1200×720 ↔ 560×96 转换、解除/恢复 min constraints、最大化恢复、
 * 置顶开关、DPI/显示器信息与权限边界。任一步失败都会尝试把窗口恢复到
 * 可操作状态（验收 3），且全程不触碰播放（不调任何 playback 接口）。
 *
 * 触发：启动前设置环境变量 LUMO_WINDOW_PROBE=1（Rust `dev_window_probe_signal`
 * 返回 true 时自动执行）；结果同时渲染到屏幕覆盖层并通过
 * `dev_window_probe_log` 写到 Rust stderr，便于无人值守采集。
 * M1 的 DM-07 实施后本文件与两个临时命令应移除。
 */

export interface ProbeStep {
  step: string;
  status: 'PASS' | 'FAIL' | 'DENIED' | 'SEAM' | 'INFO';
  detail: string;
}

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

let probeArmed = false;

/** 探针布防期间的诊断日志（clamp 决策追踪用）；未布防时零开销 no-op */
export function probeLog(line: string): void {
  if (!probeArmed) return;
  try {
    (window as any).__TAURI_INTERNALS__?.invoke('dev_window_probe_log', { line }).catch(() => {});
  } catch {
    // 探针诊断失败不影响业务
  }
}

async function getInvoke(): Promise<Invoke | null> {
  const w = window as any;
  if (!w.__TAURI_INTERNALS__) return null;
  return w.__TAURI_INTERNALS__.invoke as Invoke;
}

export async function runWindowProbe(emit: (line: string) => void): Promise<ProbeStep[]> {
  const steps: ProbeStep[] = [];
  const record = (step: string, status: ProbeStep['status'], detail: string) => {
    steps.push({ step, status, detail });
    emit(`[${status}] ${step} — ${detail}`);
  };

  const invoke = await getInvoke();
  if (!invoke) {
    record('probe', 'FAIL', '非 Tauri 环境，无法探测');
    return steps;
  }
  const win = getCurrentWindow();
  const FULL = new LogicalSize(1200, 720);
  const MINI = new LogicalSize(560, 96);

  const logicalSize = async () => {
    const [scale, inner] = [await win.scaleFactor(), await win.innerSize()];
    return { w: inner.width / scale, h: inner.height / scale };
  };
  const near = (a: { w: number; h: number }, b: { w: number; h: number }, tol = 2) =>
    Math.abs(a.w - b.w) <= tol && Math.abs(a.h - b.h) <= tol;

  const initialSize = await logicalSize();
  const initialMaximized = await win.isMaximized();

  try {
    // 1. 初始状态与显示器信息（DPI 结论的机器部分；物理换屏仍需人工）
    const scale = await win.scaleFactor();
    const monitor = await currentMonitor();
    const monitorInfo = monitor
      ? `工作区 ${Math.round(monitor.workArea.size.width / scale)}×${Math.round(monitor.workArea.size.height / scale)}@${scale}`
      : 'currentMonitor 返回 null';
    record('初始状态', 'INFO', `尺寸 ${Math.round(initialSize.w)}×${Math.round(initialSize.h)}，最大化=${initialMaximized}，${monitorInfo}`);
    try {
      const monitors = await availableMonitors();
      record('多显示器枚举', 'INFO', `${monitors.length} 台：${monitors.map((m) => `${Math.round(m.size.width / scale)}×${Math.round(m.size.height / scale)}@${m.scaleFactor}`).join(' / ')}`);
    } catch (e) {
      record('多显示器枚举', 'DENIED', `availableMonitors 权限缺失：${e}`);
    }

    // 1.5 隔离实验：在任何约束/尺寸操作之前先做一次最大化→还原，
        // 判定「maximize 失效」是自身问题还是被前置 juggling 污染
    await win.maximize();
    {
      const s: string[] = [];
      let waited = 0;
      for (const at of [50, 250, 500]) {
        await new Promise((r) => setTimeout(r, at - waited));
        waited = at;
        const m = await win.isMaximized();
        const sz = await logicalSize();
        s.push(`+${at}ms:max=${m},${Math.round(sz.w)}×${Math.round(sz.h)}`);
      }
      const firstMax = await win.isMaximized();
      await win.unmaximize();
      await new Promise((r) => setTimeout(r, 500));
      const back = await logicalSize();
      record(
        '隔离：纯净最大化→还原',
        firstMax && near(back, { w: 1200, h: 720 }, 4) ? 'PASS' : 'FAIL',
        `${s.join(' ')}；还原后 ${Math.round(back.w)}×${Math.round(back.h)}`,
      );
    }

    // 2. 解除 min constraints，验证迷你尺寸不被原限幅拉回（验收 2）
    await win.setSizeConstraints(null);
    await win.setSize(MINI);
    await new Promise((r) => setTimeout(r, 300));
    const miniSize = await logicalSize();
    if (near(miniSize, { w: MINI.width, h: MINI.height })) {
      record('解除约束→560×96', 'PASS', `实测 ${Math.round(miniSize.w)}×${Math.round(miniSize.h)}，minWidth=1024 解除后未被拉回`);
    } else {
      record('解除约束→560×96', 'FAIL', `实测 ${Math.round(miniSize.w)}×${Math.round(miniSize.h)}，窗口被外部因素改写`);
    }

    // 3. 迷你宽度与 <768 响应式断点的接缝（DM-06/07 需平台判定分离）
    const innerWidth = window.innerWidth;
    record(
      '迷你宽度 vs <768 断点',
      innerWidth < 768 ? 'SEAM' : 'INFO',
      `innerWidth=${innerWidth}${innerWidth < 768 ? '，当前实现会落入 MobileLayout 断点；DM-06 必须让桌面迷你优先于宽度断点' : '，未落入移动断点（非预期，迷你应为 560 宽）'}`,
    );

    // 4. 在迷你尺寸下恢复 min constraints，记录是否被拉大（DM-07 转换顺序依据）
    await win.setSizeConstraints({ minWidth: 1024, minHeight: 640 });
    await new Promise((r) => setTimeout(r, 300));
    const afterConstrain = await logicalSize();
    if (near(afterConstrain, { w: MINI.width, h: MINI.height })) {
      record('迷你下恢复约束', 'PASS', `实测 ${Math.round(afterConstrain.w)}×${Math.round(afterConstrain.h)}，约束未即时改写窗口`);
    } else {
      record('迷你下恢复约束', 'SEAM', `实测 ${Math.round(afterConstrain.w)}×${Math.round(afterConstrain.h)}：恢复 1024×640 约束会即时拉大迷你窗口，DM-07 转换顺序必须「先恢复尺寸再恢复约束」或按形态切换约束`);
    }

    // 5. 回到完整尺寸 + 居中
    await win.setSize(FULL);
    await win.center();
    await new Promise((r) => setTimeout(r, 200));
    const fullSize = await logicalSize();
    record('恢复 1200×720', near(fullSize, { w: 1200, h: 720 }) ? 'PASS' : 'FAIL', `实测 ${Math.round(fullSize.w)}×${Math.round(fullSize.h)}`);

    // 6. 最大化 → 还原（同步验证 M0 前的 maximize 修复）；多点采样定位标志位翻转时机
    await win.maximize();
    const samples: string[] = [];
    let waited = 0;
    for (const at of [50, 250, 500, 1000]) {
      await new Promise((r) => setTimeout(r, at - waited));
      waited = at;
      const m = await win.isMaximized();
      const s = await logicalSize();
      samples.push(`+${at}ms:max=${m},${Math.round(s.w)}×${Math.round(s.h)}`);
    }
    const maxState = await win.isMaximized();
    await win.unmaximize();
    await new Promise((r) => setTimeout(r, 500));
    const restored = await logicalSize();
    const restoredOk = (await win.isMaximized()) === false && near(restored, { w: 1200, h: 720 }, 4);
    record(
      '最大化→还原',
      maxState && restoredOk ? 'PASS' : 'FAIL',
      `${samples.join(' ')}；unmaximize 后 ${Math.round(restored.w)}×${Math.round(restored.h)}`,
    );

    // 7. 置顶开关（DM-06 迷你置顶依赖；权限按实际调用补）
    try {
      await win.setAlwaysOnTop(true);
      const on = await (win as any).isAlwaysOnTop();
      await win.setAlwaysOnTop(false);
      const off = await (win as any).isAlwaysOnTop();
      record('置顶开→关', on && !off ? 'PASS' : 'FAIL', `isAlwaysOnTop 开=${on} 关=${off}`);
    } catch (e) {
      record('置顶开→关', 'DENIED', `需要 core:window:allow-set-always-on-top / allow-is-always-on-top：${e}`);
    }
  } catch (e) {
    record('探测过程', 'FAIL', `${e}`);
  } finally {
    // 验收 3：无论哪步失败，恢复可操作窗口；不触碰音频
    try {
      await win.setSizeConstraints({ minWidth: 1024, minHeight: 640 });
      if (await win.isMaximized()) await win.unmaximize();
      await win.setSize(FULL);
      await win.center();
      const finalSize = await logicalSize();
      record('恢复现场', near(finalSize, { w: 1200, h: 720 }, 4) ? 'PASS' : 'FAIL', `约束=1024×640，尺寸 ${Math.round(finalSize.w)}×${Math.round(finalSize.h)}，置顶=关，播放未触碰`);
    } catch (e) {
      record('恢复现场', 'FAIL', `${e}`);
    }
  }
  return steps;
}

/** 覆盖层渲染：无依赖，dev 门控下由 App.vue 挂载 */
export function renderProbeOverlay(steps: ProbeStep[]): void {
  const id = 'lumo-window-probe-overlay';
  document.getElementById(id)?.remove();
  const box = document.createElement('div');
  box.id = id;
  box.style.cssText = 'position:fixed;left:12px;bottom:12px;z-index:99999;max-height:70vh;overflow:auto;background:rgba(20,20,20,.95);color:#ddd;font:12px/1.5 Consolas,monospace;padding:12px 16px;border-radius:8px;max-width:720px;pointer-events:auto;white-space:pre-wrap';
  const color = (s: ProbeStep['status']) =>
    s === 'PASS' ? '#7ad97a' : s === 'FAIL' ? '#ff7a7a' : s === 'DENIED' ? '#e0c36a' : s === 'SEAM' ? '#6ab7ff' : '#aaa';
  box.innerHTML = steps
    .map((s) => `<div style="color:${color(s.status)}">[${s.status}] ${s.step} — ${s.detail}</div>`)
    .join('');
  const close = document.createElement('button');
  close.textContent = '关闭探针';
  close.style.cssText = 'margin-top:8px;padding:2px 10px;cursor:pointer';
  close.onclick = () => box.remove();
  box.appendChild(close);
  document.body.appendChild(box);
}

/** 环境变量触发的自动执行入口；返回 null 表示未布防 */
export async function maybeAutoRunProbe(): Promise<void> {
  const invoke = await getInvoke();
  if (!invoke) return;
  try {
    const armed = await invoke('dev_window_probe_signal');
    if (!armed) return;
  } catch {
    return;
  }
  probeArmed = true;
  probeLog('[probe] armed, 3s 后开始');
  await new Promise((r) => setTimeout(r, 3000));
  const lines: string[] = [];
  try {
    const steps = await runWindowProbe((line) => {
      lines.push(line);
      // eslint-disable-next-line no-console
      console.log('[WINDOW_PROBE]', line);
      invoke('dev_window_probe_log', { line }).catch(() => {});
    });
    renderProbeOverlay(steps);
  } catch (e) {
    const line = `[FAIL] 探针未捕获异常 — ${e}`;
    // eslint-disable-next-line no-console
    console.log('[WINDOW_PROBE]', line);
    invoke('dev_window_probe_log', { line }).catch(() => {});
  }
}
