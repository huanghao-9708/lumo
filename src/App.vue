<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useUiStore } from './stores/ui';
import { usePlayerStore } from './stores/player';
import { Minus, Square, X, Sun, Moon, Palette, ChevronDown } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import AppLayout from './layout/index.vue';

const isUiDropdownOpen = ref(false);
const closeDropdown = () => setTimeout(() => isUiDropdownOpen.value = false, 200);

const uiStore = useUiStore();
const playerStore = usePlayerStore();

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
  <!-- 固定核心布局 -->
  <AppLayout />

  <!-- 全局窗口控制和 UI 切换按钮 -->
  <div class="fixed top-0 right-0 h-10 flex items-center px-4 gap-3 z-50 select-none pointer-events-none text-text-muted">
    
    <!-- Theme Toggle -->
    <button @click="uiStore.toggleDarkMode()" 
            class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 hover:text-text-main hover:bg-bg-active"
            title="切换主题">
      <Sun v-if="uiStore.isDarkMode" class="w-3.5 h-3.5" />
      <Moon v-else class="w-3.5 h-3.5" />
    </button>

    <!-- UI Selection Dropdown -->
    <div class="pointer-events-auto relative">
      <button @click="isUiDropdownOpen = !isUiDropdownOpen"
              @blur="closeDropdown()"
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg transition-all duration-200 text-[12px] hover:text-text-main hover:bg-bg-active">
        <Palette class="w-3.5 h-3.5" />
        <span class="font-medium tracking-wide">UI</span>
        <ChevronDown class="w-3 h-3 transition-transform duration-200" :class="isUiDropdownOpen ? 'rotate-180' : ''" />
      </button>

      <!-- Dropdown Menu -->
      <transition 
        enter-active-class="transition duration-200 ease-out"
        enter-from-class="transform scale-95 opacity-0"
        enter-to-class="transform scale-100 opacity-100"
        leave-active-class="transition duration-150 ease-in"
        leave-from-class="transform scale-100 opacity-100"
        leave-to-class="transform scale-95 opacity-0">
        <div v-if="isUiDropdownOpen" 
             class="absolute right-0 mt-1.5 w-32 rounded-xl shadow-lg border overflow-hidden backdrop-blur-md bg-bg-base/95 border-border-color">
          <div class="p-1 flex flex-col gap-0.5">
            <button @click="uiStore.setActiveTheme('theme-default')"
                    class="w-full text-left px-3 py-1.5 rounded-lg text-[12px] font-medium transition-colors"
                    :class="uiStore.activeTheme === 'theme-default'
                      ? 'bg-bg-active text-accent'
                      : 'text-text-muted hover:bg-bg-panel'">
              默认模板
            </button>
            <button @click="uiStore.setActiveTheme('theme-simple')"
                    class="w-full text-left px-3 py-1.5 rounded-lg text-[12px] font-medium transition-colors"
                    :class="uiStore.activeTheme === 'theme-simple'
                      ? 'bg-bg-active text-accent'
                      : 'text-text-muted hover:bg-bg-panel'">
              极简模板
            </button>
          </div>
        </div>
      </transition>
    </div>

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