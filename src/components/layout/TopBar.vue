<script setup lang="ts">
import { ChevronLeft, ChevronRight, Search, Sun, Moon, PanelRight, MoreHorizontal, Minus, Square, X, Settings as SettingsIcon } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { useUiStore } from '../../stores/ui';
import { usePlayerStore } from '../../stores/player';

const appWindow = getCurrentWindow();
const uiStore = useUiStore();
const playerStore = usePlayerStore();

const isMoreOpen = ref(false);
const moreRef = ref<HTMLDivElement | null>(null);

function toggleMore() {
  isMoreOpen.value = !isMoreOpen.value;
}

function openSettings() {
  isMoreOpen.value = false;
  playerStore.activeLibraryTab = '设置';
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activePlaylistId = null;
}

function onDocClick(e: MouseEvent) {
  if (moreRef.value && !moreRef.value.contains(e.target as Node)) {
    isMoreOpen.value = false;
  }
}

onMounted(() => document.addEventListener('click', onDocClick));
onBeforeUnmount(() => document.removeEventListener('click', onDocClick));

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <div
    class="h-[60px] w-full bg-bg-canvas flex items-center justify-between px-4 flex-shrink-0 select-none"
    data-tauri-drag-region
  >

    <!-- 左侧：全局前后导航 -->
    <div class="flex items-center gap-1 pointer-events-auto">
      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
        :class="playerStore.canGoBack ? 'text-text-secondary hover:text-text-primary hover:bg-bg-hover' : 'text-text-disabled'"
        :disabled="!playerStore.canGoBack"
        title="后退"
        @click="playerStore.goBack()"
      >
        <ChevronLeft class="w-[18px] h-[18px]" />
      </button>
      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
        :class="playerStore.canGoForward ? 'text-text-secondary hover:text-text-primary hover:bg-bg-hover' : 'text-text-disabled'"
        :disabled="!playerStore.canGoForward"
        title="前进"
        @click="playerStore.goForward()"
      >
        <ChevronRight class="w-[18px] h-[18px]" />
      </button>
    </div>

    <!-- 中间：全局搜索占位 -->
    <div class="flex-1 flex justify-center px-4 pointer-events-auto">
      <div class="relative w-full max-w-[420px]">
        <Search class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        <input
          v-model="playerStore.globalSearchQuery"
          type="text"
          placeholder="搜索全局…"
          class="w-full h-[34px] pl-8 pr-3 text-[13px] bg-bg-hover border border-transparent rounded-[8px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:bg-bg-canvas focus:border-border-color"
        />
      </div>
    </div>

    <!-- 右侧：工具组 + 窗口控制 -->
    <div class="flex items-center gap-1 pointer-events-auto">
      <div class="flex items-center gap-1 mr-3">
        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
          :class="uiStore.isDarkMode ? 'text-brand-orange bg-bg-active' : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
          :title="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
          @click="uiStore.toggleDarkMode()"
        >
          <Moon v-if="!uiStore.isDarkMode" class="w-[18px] h-[18px]" />
          <Sun v-else class="w-[18px] h-[18px]" />
        </button>

        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
          :class="uiStore.isRightSidebarVisible ? 'text-text-secondary bg-bg-hover' : 'text-text-muted hover:text-text-primary hover:bg-bg-hover'"
          title="显示/隐藏信息面板"
          @click="uiStore.toggleRightSidebar()"
        >
          <PanelRight class="w-[18px] h-[18px]" />
        </button>

        <div ref="moreRef" class="relative">
          <button
            class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
            :class="isMoreOpen ? 'bg-bg-hover text-text-primary' : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
            title="更多"
            @click="toggleMore"
          >
            <MoreHorizontal class="w-[18px] h-[18px]" />
          </button>
          <div
            v-if="isMoreOpen"
            class="absolute right-0 top-full mt-1 w-[160px] bg-bg-canvas border border-border-color rounded-[8px] shadow-lg overflow-hidden z-[50]"
          >
            <button
              class="w-full flex items-center gap-2 px-3 py-[7px] text-[13px] text-text-primary hover:bg-list-hover transition-colors-smooth"
              @click="openSettings"
            >
              <SettingsIcon class="w-[16px] h-[16px] text-text-muted" />
              设置
            </button>
          </div>
        </div>
      </div>

      <div class="w-px h-4 bg-border-color mx-1"></div>

      <div class="flex items-center gap-1">
        <button
          @click="minimize"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
          title="最小化"
        >
          <Minus class="w-4 h-4" />
        </button>
        <button
          @click="toggleMaximize"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
          title="最大化"
        >
          <Square class="w-3.5 h-3.5" />
        </button>
        <button
          @click="close"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-white hover:bg-[#E81123] transition-colors-smooth"
          title="关闭"
        >
          <X class="w-4 h-4" />
        </button>
      </div>
    </div>

  </div>
</template>
