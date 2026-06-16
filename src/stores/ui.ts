import { defineStore } from "pinia";
import { ref, watch } from "vue";

export const useUiStore = defineStore("ui", () => {
  // 深色模式状态
  const isDarkMode = ref(localStorage.getItem('lumo_dark_mode') === 'true');
  // 主题状态
  let savedTheme = localStorage.getItem('lumo_active_theme');
  if (!savedTheme || savedTheme === 'theme-default') {
    savedTheme = 'theme-simple'; // Migrate old default
  }
  const activeTheme = ref(savedTheme);

  function toggleDarkMode() {
    isDarkMode.value = !isDarkMode.value;
  }

  function setActiveTheme(theme: string) {
    activeTheme.value = theme;
  }

  // 监听并同步深色模式状态到 HTML 根节点和 localStorage
  watch(isDarkMode, (newVal) => {
    localStorage.setItem('lumo_dark_mode', String(newVal));
    if (newVal) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, { immediate: true });

  // 监听并同步主题类名到 HTML 根节点和 localStorage
  watch(activeTheme, (newVal, oldVal) => {
    localStorage.setItem('lumo_active_theme', newVal);
    if (oldVal && oldVal !== 'theme-default') {
      document.documentElement.classList.remove(oldVal);
    }
    if (newVal !== 'theme-default') {
      document.documentElement.classList.add(newVal);
    }
  }, { immediate: true });

  return {
    isDarkMode,
    activeTheme,
    toggleDarkMode,
    setActiveTheme
  };
});
