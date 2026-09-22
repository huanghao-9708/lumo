<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { usePlayerStore } from '../../stores/player';

/**
 * 播放速率选择（0.5 / 0.8 / 1 / 1.2 / 1.5）。
 *
 * 桌面底部播放栏与沉浸式播放页共用同一份实现（两处只是配色不同，用 variant 区分）。
 * 点击弹出小面板，选完立即生效并持久化到 localStorage。
 */
withDefaults(defineProps<{
  /** 'light' = 沉浸式深色底（白字），'default' = 常规浅色界面 */
  variant?: 'default' | 'light';
}>(), { variant: 'default' });

const playerStore = usePlayerStore();
const open = ref(false);
const rootRef = ref<HTMLElement | null>(null);

/** 非原速时高亮，让用户一眼看出"现在不是 1x" */
const isNonDefault = computed(() => playerStore.playbackRate !== 1);

function choose(rate: number) {
  playerStore.setPlaybackRate(rate);
  open.value = false;
}

function onDocumentMouseDown(e: MouseEvent) {
  if (!open.value) return;
  if (rootRef.value && !rootRef.value.contains(e.target as Node)) open.value = false;
}

onMounted(() => document.addEventListener('mousedown', onDocumentMouseDown));
onBeforeUnmount(() => document.removeEventListener('mousedown', onDocumentMouseDown));
</script>

<template>
  <div ref="rootRef" class="relative flex-shrink-0">
    <button
      class="h-[20px] px-1.5 rounded-[6px] font-mono text-[11px] font-medium tabular-nums transition-colors-smooth"
      :class="[
        isNonDefault
          ? 'text-brand-orange'
          : (variant === 'light' ? 'text-white/70 hover:text-white' : 'text-text-muted hover:text-text-primary'),
        variant === 'light' ? 'hover:bg-white/10' : 'hover:bg-list-hover',
      ]"
      :title="`播放速度 ${playerStore.playbackRate}x（点击切换）`"
      @click="open = !open"
    >{{ playerStore.playbackRate }}x</button>

    <div
      v-if="open"
      class="absolute bottom-full right-0 mb-2 w-[84px] py-1 rounded-[8px] border shadow-lg z-50"
      :class="variant === 'light'
        ? 'bg-black/85 border-white/15 backdrop-blur-sm'
        : 'bg-bg-canvas border-border-solid'"
    >
      <button
        v-for="rate in playerStore.PLAYBACK_RATES"
        :key="rate"
        class="block w-full text-left px-3 py-1.5 text-[12px] font-mono tabular-nums transition-colors-smooth"
        :class="rate === playerStore.playbackRate
          ? 'text-brand-orange'
          : (variant === 'light' ? 'text-white/85 hover:bg-white/10' : 'text-text-primary hover:bg-list-hover')"
        @click="choose(rate)"
      >{{ rate }}x</button>
    </div>
  </div>
</template>
