<script setup lang="ts">
import { ref } from 'vue';
import { Play, Loader2, Heart } from 'lucide-vue-next';
import type { Track } from '../../stores/player';

/**
 * 移动端歌曲行（56px 触控行）。
 *
 * 列：#(36px) + 标题/艺术家(flex-1) + 时长(48px) + 收藏按钮(44px)
 *
 * 与桌面 Song Row（40px）的关键差异：
 *   - 56px 行高，触摸目标 ≥ 44px
 *   - 单击播放（非双击），无 Hover 交互
 *   - 收藏图标始终可见（已收藏 Accent fill，未收藏 muted outline）
 *   - 仅保留 # / 标题+艺术家 / 时长，隐藏专辑/格式/更多列
 *   - Playing 行：左侧 2px Accent 竖条 + Accent title + semibold
 *   - 长按（500ms）→ 弹出 ActionSheet
 */

const props = defineProps<{
  track: Track;
  index: number;
  isPlaying: boolean;
  isCurrent: boolean;
}>();

const emit = defineEmits<{
  (e: 'play', index: number): void;
  (e: 'toggleFav', trackId: number): void;
  (e: 'longPress', trackId: number): void;
}>();

/* ============ 单击 ============ */

function onRowClick() {
  // Skip click if we just did a long press (prevented by flag)
  if (didLongPress.value) {
    didLongPress.value = false;
    return;
  }
  emit('play', props.index);
}

/* ============ 收藏 ============ */

function onFavClick(e: Event) {
  e.stopPropagation();
  emit('toggleFav', props.track.id);
}

/* ============ 长按检测（500ms） ============ */

const LONG_PRESS_MS = 500;
let longPressTimer: ReturnType<typeof setTimeout> | null = null;
const didLongPress = ref(false);

function onTouchStart(_e: TouchEvent) {
  didLongPress.value = false;
  if (longPressTimer) clearTimeout(longPressTimer);
  longPressTimer = setTimeout(() => {
    didLongPress.value = true;
    emit('longPress', props.track.id);
  }, LONG_PRESS_MS);
}

function onTouchEnd() {
  if (longPressTimer) {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }
}

function onTouchMove() {
  // Cancel long press on scroll/move
  if (longPressTimer) {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }
}
</script>

<template>
  <div
    class="flex items-center cursor-pointer relative active:bg-list-hover transition-colors-smooth"
    :class="{
      'playing-row bg-list-selected': isCurrent,
    }"
    :style="{ height: 'var(--touch-row)', contentVisibility: 'auto', containIntrinsicSize: 'var(--touch-row)' }"
    @click="onRowClick"
    @touchstart="onTouchStart"
    @touchend="onTouchEnd"
    @touchmove="onTouchMove"
  >
    <!-- 序号 / 播放图标 -->
    <div class="text-center shrink-0" style="width: 36px;">
      <span v-if="isCurrent" class="text-brand-orange inline-flex items-center justify-center">
        <Loader2 v-if="isPlaying" class="w-[14px] h-[14px] animate-spin" aria-hidden="true" />
        <Play v-else class="w-[14px] h-[14px] fill-current" aria-hidden="true" />
      </span>
      <span
        v-else
        class="text-text-muted font-mono tabular-nums"
        style="font-size: var(--text-12);"
      >{{ String(index + 1).padStart(2, '0') }}</span>
    </div>

    <!-- 标题 + 艺术家 -->
    <div class="flex-1 min-w-0 pl-1">
      <span
        class="truncate block font-medium"
        :class="isCurrent ? 'text-brand-orange font-semibold' : 'text-text-primary'"
        style="font-size: var(--text-mobile-body); line-height: 1.3;"
      >{{ track.title }}</span>
      <span
        class="text-text-secondary truncate block"
        style="font-size: var(--text-13); line-height: 1.3;"
      >{{ track.artist }}</span>
    </div>

    <!-- 时长 -->
    <div
      class="text-right shrink-0 font-mono tabular-nums text-text-muted"
      style="width: 48px; font-size: var(--text-12);"
    >{{ track.duration }}</div>

    <!-- 收藏按钮（始终可见） -->
    <button
      class="shrink-0 flex items-center justify-center ml-1 transition-colors-smooth"
      :class="track.isFavorite ? 'text-brand-orange' : 'text-text-disabled'"
      style="width: 44px; height: 44px;"
      :aria-label="track.isFavorite ? '取消收藏' : '收藏'"
      @click="onFavClick"
    >
      <Heart
        class="w-[18px] h-[18px]"
        :class="track.isFavorite ? 'fill-current' : ''"
        aria-hidden="true"
      />
    </button>
  </div>
</template>
