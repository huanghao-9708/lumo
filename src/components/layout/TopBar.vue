<script setup lang="ts">
import { Sun, Moon, Palette, MoreHorizontal, Minus, Square, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <div class="h-[60px] w-full bg-bg-canvas flex items-center justify-end px-4 flex-shrink-0 select-none" data-tauri-drag-region>
    
    <!-- Visual icons for Theme / Settings matching the design -->
    <div class="flex items-center gap-2 mr-4 pointer-events-auto">
      <button class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="Light Mode">
        <Sun class="w-[18px] h-[18px]" />
      </button>
      <button class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="Theme">
        <Palette class="w-[18px] h-[18px]" />
      </button>
      <button class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="Dark Mode">
        <Moon class="w-[18px] h-[18px]" />
      </button>
      <button class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="More">
        <MoreHorizontal class="w-[18px] h-[18px]" />
      </button>
    </div>

    <!-- 竖向分割线 -->
    <div class="w-px h-4 bg-border-color mx-2"></div>

    <!-- 窗口控制按钮 -->
    <div class="flex items-center gap-1 pointer-events-auto ml-2">
      <button @click="minimize" class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="最小化">
        <Minus class="w-4 h-4" />
      </button>
      <button @click="toggleMaximize" class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-text-primary hover:bg-bg-hover" title="最大化">
        <Square class="w-3.5 h-3.5" />
      </button>
      <button @click="close" class="w-8 h-8 flex items-center justify-center rounded-md transition-colors text-text-secondary hover:text-white hover:bg-[#E81123]" title="关闭">
        <X class="w-4 h-4" />
      </button>
    </div>

  </div>
</template>
