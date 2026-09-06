<script setup lang="ts">
import { Loader2 } from 'lucide-vue-next';
import AppModal from './AppModal.vue';

/**
 * 通用确认弹窗（基于 AppModal）。danger=true 时确认按钮为红色。
 * busy 期间禁止 Esc / 遮罩关闭，防止异步操作中误关。
 */
withDefaults(defineProps<{
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
  busy?: boolean;
}>(), {
  confirmText: '确认',
  cancelText: '取消',
  danger: false,
  busy: false,
});

const emit = defineEmits<{ confirm: []; cancel: [] }>();
</script>

<template>
  <AppModal width="380px" :close-on-overlay="!busy" :close-on-esc="!busy">
    <div class="px-6 pt-6 pb-2">
      <h2 class="text-[16px] font-bold text-text-primary leading-tight">{{ title }}</h2>
      <p class="text-[13px] text-text-secondary mt-2 leading-relaxed">{{ message }}</p>
    </div>
    <div class="px-6 pb-6 pt-3 flex items-center justify-end gap-2">
      <button
        class="h-[34px] px-4 text-[13px] text-text-secondary hover:text-text-primary transition-colors-smooth"
        @click="emit('cancel')"
      >{{ cancelText }}</button>
      <button
        class="h-[34px] px-5 rounded-full text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity disabled:opacity-40"
        :class="danger ? 'bg-status-error text-white' : 'bg-text-primary text-bg-canvas'"
        :disabled="busy"
        @click="emit('confirm')"
      >
        <Loader2 v-if="busy" class="w-3.5 h-3.5 animate-spin" />
        {{ confirmText }}
      </button>
    </div>
  </AppModal>
</template>
