import { defineStore } from "pinia";
import { ref, watch } from "vue";



/**
 * UI 全局状态：夜间模式、右栏可见性。
 *
 * 夜间模式通过切换 <html> 上的 data-theme 属性，由 style.css 里
 * `[data-theme="dark"]` 选择器覆盖 LDL token 实现，无需在此处硬编码颜色。
 * 状态持久化到 localStorage。
 */
export const useUiStore = defineStore("ui", () => {
  // ===== 夜间模式 =====
  const DARK_KEY = "lumo_dark_mode";

  function readDarkPref(): boolean {
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
    localStorage.setItem(DARK_KEY, dark ? "1" : "0");
  });

  function toggleDarkMode() {
    isDarkMode.value = !isDarkMode.value;
  }

  function setDarkMode(dark: boolean) {
    isDarkMode.value = dark;
  }

  // ===== 右侧 Inspector 面板可见性 =====
  const isRightSidebarVisible = ref(false);

  function toggleRightSidebar() {
    isRightSidebarVisible.value = !isRightSidebarVisible.value;
  }

  function setRightSidebarVisible(visible: boolean) {
    isRightSidebarVisible.value = visible;
  }

  return {
    isDarkMode,
    toggleDarkMode,
    setDarkMode,
    isRightSidebarVisible,
    toggleRightSidebar,
    setRightSidebarVisible,
  };
});
