<script setup lang="ts">
import { AlertCircle, Info } from 'lucide-vue-next';
import { useUiStore } from '../../stores/ui';

/**
 * 全局轻量 Toast：错误/提示自动 4s 消退（状态在 uiStore.toast）。
 * Teleport 到 body，桌面/移动布局共用（App.vue 挂载一次）。
 */
const uiStore = useUiStore();
</script>

<template>
  <Teleport to="body">
    <Transition name="app-toast">
      <div
        v-if="uiStore.toast"
        :key="uiStore.toast.id"
        class="app-toast flex items-center gap-2 px-4 py-2.5 rounded-[10px] bg-text-primary text-bg-canvas text-[13px] shadow-lg max-w-[480px]"
        role="status"
      >
        <AlertCircle v-if="uiStore.toast.kind === 'error'" class="w-4 h-4 text-status-error shrink-0" />
        <Info v-else class="w-4 h-4 text-status-info shrink-0" />
        <span class="truncate">{{ uiStore.toast.message }}</span>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.app-toast {
  position: fixed;
  bottom: 130px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 150;
}

/* LDL 动效：200ms ease-out 上浮淡入，禁弹簧/缩放 */
.app-toast-enter-active,
.app-toast-leave-active {
  transition: opacity 0.2s ease-out, transform 0.2s ease-out;
}
.app-toast-enter-from,
.app-toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
}
</style>
