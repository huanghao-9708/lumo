<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue';
import { useUiStore } from './stores/ui';
import { Minus, Square, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';

const uiStore = useUiStore();

// 动态载入当前激活的 UI 插件入口
const activeUiComponent = computed(() => {
  switch (uiStore.activePlugin) {
    case 'ui-simple':
      return defineAsyncComponent(() => import('./plugins/ui-simple/index.vue'));
    case 'ui-default':
    default:
      return defineAsyncComponent(() => import('./plugins/ui-default/index.vue'));
  }
});

const appWindow = getCurrentWindow();
const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <!-- 动态挂载选中的播放器 UI 插件 -->
  <component :is="activeUiComponent" />

  <!-- 全局窗口控制和 UI 切换按钮 -->
  <div class="fixed top-0 right-0 h-10 flex items-center px-4 gap-3 z-50 select-none pointer-events-none">
    <!-- UI 切换 Pill 按钮 -->
    <div class="pointer-events-auto flex items-center p-0.5 rounded-full border text-[11px] font-medium transition-all duration-300 shadow-sm"
      :class="uiStore.activePlugin === 'ui-simple' 
        ? 'bg-[#eae8e1] border-[#dcdad1] text-[#888]' 
        : 'bg-gray-100 border-gray-200 text-gray-400'">
      <button 
        @click="uiStore.setActivePlugin('ui-default')"
        class="px-2.5 py-0.5 rounded-full transition-all duration-300 tracking-wider text-[11px]"
        :class="uiStore.activePlugin === 'ui-default'
          ? 'bg-white text-brand-orange shadow-sm font-semibold'
          : 'hover:text-gray-700'"
      >
        默认
      </button>
      <button 
        @click="uiStore.setActivePlugin('ui-simple')"
        class="px-2.5 py-0.5 rounded-full transition-all duration-300 tracking-wider text-[11px]"
        :class="uiStore.activePlugin === 'ui-simple'
          ? 'bg-[#1a1a1a] text-[#fdfcf9] shadow-sm font-semibold'
          : 'hover:text-[#333]'"
      >
        极简
      </button>
    </div>

    <!-- 竖向分割线 -->
    <div class="w-px h-4 transition-colors duration-300"
      :class="uiStore.activePlugin === 'ui-simple' ? 'bg-[#dcdad1]' : 'bg-gray-200'"></div>

    <!-- 窗口控制按钮 -->
    <div class="flex items-center gap-1.5">
      <button @click="minimize" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200"
        :class="uiStore.activePlugin === 'ui-simple' 
          ? 'text-[#888] hover:text-black hover:bg-[#eae8e1]' 
          : 'text-gray-400 hover:text-gray-700 hover:bg-gray-100'"
        title="最小化">
        <Minus class="w-3.5 h-3.5" />
      </button>
      <button @click="toggleMaximize" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200"
        :class="uiStore.activePlugin === 'ui-simple' 
          ? 'text-[#888] hover:text-black hover:bg-[#eae8e1]' 
          : 'text-gray-400 hover:text-gray-700 hover:bg-gray-100'"
        title="最大化">
        <Square class="w-3 h-3" />
      </button>
      <button @click="close" class="pointer-events-auto w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200"
        :class="uiStore.activePlugin === 'ui-simple' 
          ? 'text-[#888] hover:text-red-600 hover:bg-red-50' 
          : 'text-gray-400 hover:text-red-500 hover:bg-red-50'"
        title="关闭">
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</template>

<style>
/* 全局重置或覆盖样式 */
</style>