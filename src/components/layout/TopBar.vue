<script setup lang="ts">
import { Sun, Moon, PanelRight, MoreHorizontal, Minus, Square, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useUiStore } from '../../stores/ui';

const appWindow = getCurrentWindow();
const uiStore = useUiStore();

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <div
    class="h-[60px] w-full bg-bg-canvas flex items-center justify-end px-4 flex-shrink-0 select-none"
    data-tauri-drag-region
  >

    <!-- 右侧工具组：夜间模式 / 右栏 / 更多 -->
    <div class="flex items-center gap-1 mr-3 pointer-events-auto">
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

      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
        title="更多"
      >
        <MoreHorizontal class="w-[18px] h-[18px]" />
      </button>
    </div>

    <!-- 竖向分割线 -->
    <div class="w-px h-4 bg-border-color mx-1"></div>

    <!-- 窗口控制按钮 -->
    <div class="flex items-center gap-1 pointer-events-auto ml-1">
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
</template>
