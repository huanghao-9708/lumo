<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';

/**
 * 桌面端通用弹窗底座（LDL：遮罩 bg-black/30 + 居中卡 rounded-[12px]）。
 * Teleport 到 body，Esc / 点击遮罩关闭；header / footer 插槽固定，
 * 默认插槽区域独立滚动（列表型弹窗内容多时不撑爆卡片）。
 */
const props = withDefaults(defineProps<{
  /** 弹窗卡片宽度（任意 CSS 宽度值） */
  width?: string;
  /** 点击遮罩是否关闭 */
  closeOnOverlay?: boolean;
  /** 按 Esc 是否关闭 */
  closeOnEsc?: boolean;
}>(), {
  width: '520px',
  closeOnOverlay: true,
  closeOnEsc: true,
});

const emit = defineEmits<{ close: [] }>();

function onOverlayClick(e: MouseEvent) {
  if (props.closeOnOverlay && e.target === e.currentTarget) emit('close');
}

function onKeydown(e: KeyboardEvent) {
  if (props.closeOnEsc && e.key === 'Escape') {
    e.stopPropagation();
    emit('close');
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition name="app-modal">
      <div
        class="fixed inset-0 z-[100] bg-black/30 flex items-center justify-center"
        role="dialog"
        aria-modal="true"
        @click="onOverlayClick"
      >
        <div
          class="bg-bg-canvas rounded-[12px] shadow-lg overflow-hidden flex flex-col max-h-[82vh] max-w-[calc(100vw-48px)]"
          :style="{ width: props.width }"
        >
          <slot name="header" />
          <div class="overflow-y-auto min-h-0">
            <slot />
          </div>
          <slot name="footer" />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<!-- LDL 动效：仅 200ms ease-out 淡入淡出，禁缩放/弹簧 -->
<style scoped>
.app-modal-enter-active,
.app-modal-leave-active {
  transition: opacity 0.2s ease-out;
}
.app-modal-enter-from,
.app-modal-leave-to {
  opacity: 0;
}
</style>
