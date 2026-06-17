<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue';
import { useUiStore } from './stores/ui';
import { usePlayerStore } from './stores/player';
import { Minus, Square, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { themes } from './themes';

const uiStore = useUiStore();
const playerStore = usePlayerStore();

// 计算当前主题的动态 Layout
const ActiveLayout = computed(() => {
  const theme = themes[uiStore.activeTheme];
  if (theme && theme.components.Layout) {
    return theme.components.Layout;
  }
  // Fallback to simple layout
  return themes['theme-simple'].components.Layout;
});

// 键盘快捷键监听
const handleGlobalKeyDown = (e: KeyboardEvent) => {
  // 判断当前焦点是否在输入框
  const activeEl = document.activeElement;
  if (activeEl && (
    activeEl.tagName === 'INPUT' || 
    activeEl.tagName === 'TEXTAREA' || 
    activeEl.getAttribute('contenteditable') === 'true'
  )) {
    return;
  }

  // 1. 空格键播放/暂停
  if (e.code === 'Space') {
    e.preventDefault();
    playerStore.togglePlay();
  }

  // 2. Ctrl + 左右箭头：切歌
  if (e.ctrlKey && e.code === 'ArrowRight') {
    e.preventDefault();
    playerStore.nextTrack();
  } else if (e.ctrlKey && e.code === 'ArrowLeft') {
    e.preventDefault();
    playerStore.prevTrack();
  }

  // 3. 左右箭头：快退 / 快进 5 秒
  else if (e.code === 'ArrowRight') {
    e.preventDefault();
    const newPos = Math.min(playerStore.durationMs, playerStore.progressMs + 5000);
    playerStore.seek(newPos);
  } else if (e.code === 'ArrowLeft') {
    e.preventDefault();
    const newPos = Math.max(0, playerStore.progressMs - 5000);
    playerStore.seek(newPos);
  }

  // 4. 上下箭头：增减音量
  else if (e.code === 'ArrowUp') {
    e.preventDefault();
    const newVol = Math.min(100, playerStore.volume + 5);
    playerStore.setVolume(newVol);
  } else if (e.code === 'ArrowDown') {
    e.preventDefault();
    const newVol = Math.max(0, playerStore.volume - 5);
    playerStore.setVolume(newVol);
  }
};

onMounted(() => {
  playerStore.restoreSession();
  window.addEventListener('keydown', handleGlobalKeyDown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleGlobalKeyDown);
});

const appWindow = getCurrentWindow();
const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <!-- 动态装配的主题核心布局 -->
  <component :is="ActiveLayout" />

  <!-- 全局窗口控制按钮（主题切换 / 夜间模式 / UI 切换已统一移入各主题侧边栏的设置按钮旁） -->
  <div class="fixed top-0 right-0 h-10 flex items-center px-4 gap-3 z-50 select-none pointer-events-none text-text-muted">
    <!-- 竖向分割线 -->
    <div class="w-px h-4 transition-colors duration-300 bg-border-color"></div>

    <!-- 窗口控制按钮 -->
    <div class="flex items-center gap-1.5">
      <button @click="minimize" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 hover:text-text-main hover:bg-bg-active"
        title="最小化">
        <Minus class="w-3.5 h-3.5" />
      </button>
      <button @click="toggleMaximize" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 hover:text-text-main hover:bg-bg-active"
        title="最大化">
        <Square class="w-3 h-3" />
      </button>
      <button @click="close" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 hover:text-red-600 hover:bg-red-50 -900/30 -400"
        title="关闭">
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</template>

<style>
/* 全局重置或覆盖样式 */
</style>