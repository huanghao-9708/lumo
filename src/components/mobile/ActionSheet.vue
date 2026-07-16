<script setup lang="ts">
import { watch, ref, onBeforeUnmount } from 'vue';

/**
 * 移动端底部操作菜单（Action Sheet）。
 *
 * 从底部滑入，带有半透明遮罩。点击遮罩或取消按钮关闭。
 * 用于长按歌曲行、专辑等触发的上下文菜单。
 *
 * Props:
 *   visible   - 是否显示
 *   actions   - 操作项列表（label + onClick + danger? + disabled?）
 *
 * 动画：250ms ease-out，底部 translateY(100%) → 0
 */

export interface ActionItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

const props = defineProps<{
  visible: boolean;
  actions: ActionItem[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const isLeaving = ref(false);

watch(() => props.visible, (v) => {
  if (!v) isLeaving.value = false;
});

function onAction(item: ActionItem) {
  if (item.disabled) return;
  item.onClick();
  close();
}

let closeTimer: ReturnType<typeof setTimeout> | null = null;

function close() {
  isLeaving.value = true;
  if (closeTimer) clearTimeout(closeTimer);
  closeTimer = setTimeout(() => {
    isLeaving.value = false;
    closeTimer = null;
    emit('close');
  }, 250);
}

function onBackdropClick() {
  close();
}

onBeforeUnmount(() => {
  if (closeTimer) clearTimeout(closeTimer);
  isLeaving.value = false;
});
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[200] flex flex-col justify-end">
      <!-- 遮罩 -->
      <Transition name="sheet-fade">
        <div
          v-if="visible && !isLeaving"
          class="absolute inset-0 bg-black/30"
          @click="onBackdropClick"
        ></div>
      </Transition>

      <!-- 菜单面板 -->
      <Transition name="sheet-slide">
        <div
          v-if="visible && !isLeaving"
          class="relative bg-bg-canvas rounded-t-[16px] overflow-hidden flex flex-col"
          style="padding-bottom: env(safe-area-inset-bottom);"
        >
          <!-- 操作项 -->
          <div class="py-2">
            <button
              v-for="(item, i) in actions"
              :key="i"
              class="w-full h-12 flex items-center justify-center text-[15px] font-medium transition-colors-smooth active:bg-list-hover disabled:opacity-40 disabled:cursor-not-allowed"
              :class="item.danger ? 'text-status-error' : 'text-text-primary'"
              :disabled="item.disabled"
              @click="onAction(item)"
            >
              {{ item.label }}
            </button>
          </div>

          <!-- 取消 -->
          <div class="h-px bg-border-color mx-4"></div>
          <button
            class="w-full h-12 flex items-center justify-center text-[15px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth"
            @click="close"
          >
            取消
          </button>
        </div>
      </Transition>
    </div>
  </Teleport>
</template>

<style>
/* Action Sheet 动画（LDL：250ms ease-out） */
.sheet-fade-enter-active,
.sheet-fade-leave-active {
  transition: opacity 0.25s ease-out;
}
.sheet-fade-enter-from,
.sheet-fade-leave-to {
  opacity: 0;
}

.sheet-slide-enter-active,
.sheet-slide-leave-active {
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.sheet-slide-enter-from,
.sheet-slide-leave-to {
  transform: translateY(100%);
}
</style>
