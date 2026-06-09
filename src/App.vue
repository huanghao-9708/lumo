<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue';
import { useUiStore } from './stores/ui';

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
</script>

<template>
  <!-- 动态挂载选中的播放器 UI 插件 -->
  <component :is="activeUiComponent" />
</template>

<style>
/* 全局重置或覆盖样式 */
</style>