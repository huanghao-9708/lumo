import { defineStore } from "pinia";
import { ref, watch } from "vue";

/**
 * 移动端底部 Tab Bar 的 4 个一级导航。
 *
 *   library   → 曲库（对应桌面 Sidebar Library 组：全部歌曲/专辑/艺术家/…）
 *   search    → 搜索（对应桌面 Content Toolbar 搜索 + GlobalSearch 视图）
 *   favorites → 收藏（对应桌面 Sidebar Favorites 组：喜欢的音乐/收藏专辑/收藏歌手）
 *   settings  → 设置（对应桌面 TopBar 更多菜单中的设置页）
 */
export type MobileTab = 'library' | 'search' | 'favorites' | 'settings';

/** Toast 类型：error 红色警示 / info 中性提示 */
export type ToastKind = 'error' | 'info';

export interface ToastMessage {
  id: number;
  message: string;
  kind: ToastKind;
}



/**
 * UI 全局状态：夜间模式、右栏可见性、沉浸式播放页。
 *
 * 夜间模式通过切换 <html> 上的 data-theme 属性，由 style.css 里
 * `[data-theme="dark"]` 选择器覆盖 LDL token 实现，无需在此处硬编码颜色。
 * 状态持久化到 localStorage。
 */
export const useUiStore = defineStore("ui", () => {
  // ===== 夜间模式 =====
  const DARK_KEY = "lumo_dark_mode";
  const FOLLOW_SYSTEM_KEY = "lumo_follow_system";

  function readFollowSystem(): boolean {
    return localStorage.getItem(FOLLOW_SYSTEM_KEY) === "1";
  }

  /** 是否跟随系统暗色模式 */
  const followSystem = ref(readFollowSystem());

  function readDarkPref(): boolean {
    if (followSystem.value) {
      return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
    }
    const saved = localStorage.getItem(DARK_KEY);
    if (saved !== null) return saved === "1";
    return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
  }

  const isDarkMode = ref(readDarkPref());

  /** 把 data-theme 同步到 <html>，CSS 变量据此切换 */
  function applyThemeToDom(dark: boolean) {
    const el = document.documentElement;
    if (dark) el.setAttribute("data-theme", "dark");
    else el.removeAttribute("data-theme");
  }

  // 初始应用一次
  applyThemeToDom(isDarkMode.value);

  watch(isDarkMode, (dark) => {
    applyThemeToDom(dark);
    // 仅在非跟随系统模式下持久化手动偏好
    if (!followSystem.value) {
      localStorage.setItem(DARK_KEY, dark ? "1" : "0");
    }
  });

  function toggleDarkMode() {
    isDarkMode.value = !isDarkMode.value;
  }

  function setDarkMode(dark: boolean) {
    isDarkMode.value = dark;
  }

  /** 设置是否跟随系统暗色模式 */
  function setFollowSystem(follow: boolean) {
    followSystem.value = follow;
    localStorage.setItem(FOLLOW_SYSTEM_KEY, follow ? "1" : "0");
    if (follow) {
      const prefersDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
      isDarkMode.value = prefersDark;
      localStorage.removeItem(DARK_KEY);
    }
  }

  // ===== 右侧 Inspector 面板可见性 =====
  const isRightSidebarVisible = ref(false);

  function toggleRightSidebar() {
    isRightSidebarVisible.value = !isRightSidebarVisible.value;
  }

  function setRightSidebarVisible(visible: boolean) {
    isRightSidebarVisible.value = visible;
  }

  // ===== 沉浸式播放页（全屏 Now Playing）可见性 =====
  // 由 BottomPlayer 封面点击 / ChevronDown 触发，从下往上抽屉式覆盖整窗。
  const isImmersiveView = ref(false);

  function openImmersiveView() {
    isImmersiveView.value = true;
  }
  function closeImmersiveView() {
    isImmersiveView.value = false;
  }
  function toggleImmersiveView() {
    isImmersiveView.value = !isImmersiveView.value;
  }

  // ===== 网络状态（断网/弱网降级控制） =====
  const isOnline = ref(navigator.onLine);

  function setOnline(v: boolean) {
    isOnline.value = v;
  }

  // ===== 轻量 Toast（错误/提示，4s 自动消退；全局唯一一条，新的顶掉旧的） =====
  const toast = ref<ToastMessage | null>(null);
  let toastTimer: number | null = null;

  function showToast(message: string, kind: ToastKind = 'error') {
    if (toastTimer !== null) clearTimeout(toastTimer);
    toast.value = { id: Date.now(), message, kind };
    toastTimer = window.setTimeout(() => {
      toast.value = null;
      toastTimer = null;
    }, 4000);
  }

  // ===== 移动端视图状态 =====
  // 移动端底部 Tab Bar 的 4 个一级导航（类型见模块顶层 MobileTab）。
  // Tab 切换时会同步设置 playerStore.activeLibraryTab，使现有视图逻辑无缝复用。
  const activeMobileTab = ref<MobileTab>('library');

  function setMobileTab(tab: MobileTab) {
    activeMobileTab.value = tab;
  }

  return {
    isDarkMode,
    toggleDarkMode,
    setDarkMode,
    followSystem,
    setFollowSystem,
    isRightSidebarVisible,
    toggleRightSidebar,
    setRightSidebarVisible,
    isImmersiveView,
    openImmersiveView,
    closeImmersiveView,
    toggleImmersiveView,
    isOnline,
    setOnline,
    toast,
    showToast,
    // 移动端
    activeMobileTab,
    setMobileTab,
  };
});
