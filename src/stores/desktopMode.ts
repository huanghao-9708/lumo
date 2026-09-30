import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "../utils/tauriInvoke";
import { useUiStore } from "./ui";

/**
 * 桌面两维模式控制器（desktop-modes 01 §4，DM-01）。
 *
 * 两个独立维度：
 *   - 体验模式 experienceMode：normal（正常）/ minimal（极简），决定视觉与可选任务策略
 *   - 窗口形态 windowForm：full（完整窗口）/ mini（迷你播放栏），决定挂载界面与窗口几何
 *
 * 权威在后端 `desktop_ui_preferences.json`（Rust 版本化校验 + 原子写入）；
 * 本 store 是运行镜像。转换串行化：转换期间拒绝新转换（防抖由调用方 UI 处理），
 * 每次转换持有递增 transitionId，过期响应一律丢弃。写入失败保留本次切换（内存），
 * 提示「本次有效，未保存」，不循环重试。
 *
 * M0 探针结论（M0_执行记录.md）：窗口几何的读取/应用与最大化状态跟踪在 DM-07
 * 接入；本 store 现阶段只负责状态机与持久化，geometry 字段为 DM-07 预留。
 */

export type ExperienceMode = 'normal' | 'minimal';
export type WindowForm = 'full' | 'mini';
export type TransitionPhase = 'idle' | 'entering-mini' | 'leaving-mini' | 'changing-experience';

/** 逻辑像素几何（与后端 WindowGeometry 对应） */
export interface WindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
}

export interface DesktopPreferences {
  schemaVersion: number;
  experienceMode: ExperienceMode;
  windowForm: WindowForm;
  fullGeometry: WindowGeometry | null;
  miniGeometry: WindowGeometry | null;
  miniAlwaysOnTop: boolean;
}

/** 后端 desktop_get_preferences 返回：fileExisted=false 表示首次运行（需迁移旧尺寸） */
interface DesktopPreferencesSnapshot {
  preferences: DesktopPreferences;
  fileExisted: boolean;
}

/** 旧窗口尺寸 localStorage 键（useWindowPersistence 现行权威值，仅首次迁移读取） */
const LEGACY_SIZE_KEY = 'lumo_window_size';
const PREFS_SCHEMA_VERSION = 1;

function isExperienceMode(v: unknown): v is ExperienceMode {
  return v === 'normal' || v === 'minimal';
}
function isWindowForm(v: unknown): v is WindowForm {
  return v === 'full' || v === 'mini';
}

/** 防御后端之外的任何来源（迁移值/测试注入）的几何数据 */
function sanitizeGeometry(v: unknown): WindowGeometry | null {
  if (!v || typeof v !== 'object') return null;
  const g = v as Partial<WindowGeometry>;
  if (
    typeof g.x !== 'number' || !Number.isFinite(g.x) ||
    typeof g.y !== 'number' || !Number.isFinite(g.y) ||
    typeof g.width !== 'number' || !Number.isFinite(g.width) || g.width <= 0 ||
    typeof g.height !== 'number' || !Number.isFinite(g.height) || g.height <= 0
  ) {
    return null;
  }
  return {
    x: Math.round(g.x), y: Math.round(g.y),
    width: Math.round(g.width), height: Math.round(g.height),
    maximized: g.maximized === true,
  };
}

export const useDesktopModeStore = defineStore("desktopMode", () => {
  const uiStore = useUiStore();

  // ===== 运行镜像（权威在后端 JSON；启动时 init 载入） =====
  const loaded = ref(false);
  const experienceMode = ref<ExperienceMode>('normal');
  const windowForm = ref<WindowForm>('full');
  const fullGeometry = ref<WindowGeometry | null>(null);
  const miniGeometry = ref<WindowGeometry | null>(null);
  const miniAlwaysOnTop = ref(false);

  // ===== 转换事务状态 =====
  const phase = ref<TransitionPhase>('idle');
  let transitionSeq = 0;
  /** 上次写入失败的偏好仍生效（内存），提示条由 UI 依据此标记展示 */
  const prefsDirty = ref(false);

  /** 视觉需求派生（DM-03 资源策略的输入；联网仍需既有隐私授权） */
  const visualAllowed = computed(
    () => experienceMode.value === 'normal' && windowForm.value === 'full',
  );

  /**
   * 策略版本（DM-03 验收 2）：每次有效转换/偏好应用自增。
   * 异步消费者（图片、歌词、取色）在发出请求时记录版本，写回前比对，
   * 版本不一致即丢弃——保证「有效策略先于异步响应」。
   */
  const strategyVersion = ref(1);

  /** 把视觉策略下发给后端（回填批次门禁）。advisory：失败只影响可选任务时序 */
  function pushVisualPolicy() {
    invoke('desktop_set_visual_policy', { allowed: visualAllowed.value }).catch((e) => {
      console.warn('[desktopMode] 视觉策略下发失败（本次运行可选任务时序可能退化为默认）', e);
    });
  }

  function snapshot(): DesktopPreferences {
    return {
      schemaVersion: PREFS_SCHEMA_VERSION,
      experienceMode: experienceMode.value,
      windowForm: windowForm.value,
      fullGeometry: fullGeometry.value,
      miniGeometry: miniGeometry.value,
      miniAlwaysOnTop: miniAlwaysOnTop.value,
    };
  }

  function applyPrefs(prefs: DesktopPreferences) {
    // 后端已做字段级回退，这里再防御一次（前端是唯一消费方，宽松解析）
    experienceMode.value = isExperienceMode(prefs.experienceMode) ? prefs.experienceMode : 'normal';
    windowForm.value = isWindowForm(prefs.windowForm) ? prefs.windowForm : 'full';
    fullGeometry.value = sanitizeGeometry(prefs.fullGeometry);
    miniGeometry.value = sanitizeGeometry(prefs.miniGeometry);
    miniAlwaysOnTop.value = prefs.miniAlwaysOnTop === true;
  }

  /** 旧 lumo_window_size 首次迁移：仅后端无偏好文件时执行一次 */
  function readLegacySize(): WindowGeometry | null {
    try {
      const raw = localStorage.getItem(LEGACY_SIZE_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as { w?: unknown; h?: unknown };
      const w = typeof parsed?.w === 'number' ? parsed.w : NaN;
      const h = typeof parsed?.h === 'number' ? parsed.h : NaN;
      return sanitizeGeometry({ x: 0, y: 0, width: w, height: h, maximized: false });
    } catch {
      return null;
    }
  }

  /**
   * 启动载入。失败或损坏都留在默认 normal+full（后端已保证），不阻塞启动，
   * 也不触发任何可选视觉任务（初始策略先于可选任务）。
   */
  async function init(): Promise<void> {
    try {
      const snap = await invoke<DesktopPreferencesSnapshot>('desktop_get_preferences');
      applyPrefs(snap.preferences ?? ({} as DesktopPreferences));
      if (!snap.fileExisted) {
        // 首次运行：旧完整尺寸迁移（一次性，失败静默留在默认）
        const legacy = readLegacySize();
        if (legacy) {
          fullGeometry.value = legacy;
          try {
            await invoke('desktop_update_preferences', { preferences: snapshot() });
          } catch {
            // 迁移失败不影响运行；保留内存值，下次转换时随快照一并提交
            prefsDirty.value = true;
          }
        }
      }
    } catch (e) {
      console.warn('[desktopMode] 偏好读取失败，使用默认 normal+full', e);
    } finally {
      strategyVersion.value++;
      loaded.value = true;
      // 视觉策略确认：回填等可选任务此后按 visualAllowed 运行
      pushVisualPolicy();
    }
  }

  /**
   * 转换执行器：串行化 + 过期 token + 写失败不回滚内存状态。
   * 返回 false 表示被拒绝（转换进行中或目标已是当前值）。
   */
  async function runTransition(
    next: TransitionPhase,
    apply: () => void,
  ): Promise<boolean> {
    if (phase.value !== 'idle') return false;
    const myId = ++transitionSeq;
    phase.value = next;
    try {
      apply();
      // 策略版本先于任何异步消费者写回自增（DM-03 验收 2）
      strategyVersion.value++;
      await invoke('desktop_update_preferences', { preferences: snapshot() });
      if (myId !== transitionSeq) return true; // 已被更新的事务取代，结果丢弃
      prefsDirty.value = false;
      return true;
    } catch (e) {
      console.warn('[desktopMode] 偏好写入失败：本次切换仅在内存生效', e);
      // 验收 5：播放与本次切换继续；提示未保存；不循环重试
      prefsDirty.value = true;
      uiStore.showToast('桌面偏好写入失败：本次有效，未保存', 'error');
      return true;
    } finally {
      if (myId === transitionSeq) {
        phase.value = 'idle';
        // 内存策略已生效：无论持久化成败，按当前 visualAllowed 下发（极简仅暂停需求）
        pushVisualPolicy();
      }
    }
  }

  /** 切体验模式（normal/minimal），不改窗口形态与主题 */
  async function setExperienceMode(mode: ExperienceMode): Promise<boolean> {
    if (mode === experienceMode.value) return true;
    return runTransition('changing-experience', () => {
      experienceMode.value = mode;
    });
  }

  /** 进入迷你播放栏（DM-06 起才有可见效果；此处只推进状态机与持久化） */
  async function enterMini(): Promise<boolean> {
    if (windowForm.value === 'mini') return true;
    return runTransition('entering-mini', () => {
      windowForm.value = 'mini';
    });
  }

  /** 返回完整窗口，保留原体验模式 */
  async function exitMini(): Promise<boolean> {
    if (windowForm.value === 'full') return true;
    return runTransition('leaving-mini', () => {
      windowForm.value = 'full';
    });
  }

  /** 几何更新（DM-07 接入窗口事务时调用；转换期间拒绝，避免几何互相覆盖） */
  async function updateFullGeometry(g: WindowGeometry): Promise<boolean> {
    const clean = sanitizeGeometry(g);
    if (!clean || phase.value !== 'idle') return false;
    fullGeometry.value = clean;
    try {
      await invoke('desktop_update_preferences', { preferences: snapshot() });
      return true;
    } catch {
      prefsDirty.value = true;
      return false;
    }
  }

  async function updateMiniGeometry(g: WindowGeometry): Promise<boolean> {
    const clean = sanitizeGeometry(g);
    if (!clean || phase.value !== 'idle') return false;
    miniGeometry.value = clean;
    try {
      await invoke('desktop_update_preferences', { preferences: snapshot() });
      return true;
    } catch {
      prefsDirty.value = true;
      return false;
    }
  }

  async function setMiniAlwaysOnTop(on: boolean): Promise<boolean> {
    if (miniAlwaysOnTop.value === on) return true;
    return runTransition('changing-experience', () => {
      miniAlwaysOnTop.value = on;
    });
  }

  return {
    loaded,
    experienceMode,
    windowForm,
    fullGeometry,
    miniGeometry,
    miniAlwaysOnTop,
    phase,
    prefsDirty,
    visualAllowed,
    strategyVersion,
    init,
    setExperienceMode,
    enterMini,
    exitMini,
    updateFullGeometry,
    updateMiniGeometry,
    setMiniAlwaysOnTop,
  };
});

// M1 测试入口（计划 02 §4：无正式新入口也能在测试入口验证四组合策略）。
// DM-06 提供正式入口后移除。用法：
//   __LUMO_DESKTOP_MODE__.get() / setExperience('minimal') / enterMini() / exitMini()
if (import.meta.env.DEV && typeof window !== 'undefined') {
  (window as any).__LUMO_DESKTOP_MODE__ = {
    get: () => {
      const s = useDesktopModeStore();
      return {
        experienceMode: s.experienceMode,
        windowForm: s.windowForm,
        phase: s.phase,
        visualAllowed: s.visualAllowed,
        prefsDirty: s.prefsDirty,
      };
    },
    setExperience: (mode: ExperienceMode) => useDesktopModeStore().setExperienceMode(mode),
    enterMini: () => useDesktopModeStore().enterMini(),
    exitMini: () => useDesktopModeStore().exitMini(),
  };
  console.info(
    '%c[LUMO] 桌面模式测试入口已启用：__LUMO_DESKTOP_MODE__.get() / setExperience("minimal") / enterMini() / exitMini()',
    'color:#a6e',
  );
}
