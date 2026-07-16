<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1,
  ChevronDown, Disc3, Heart,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useCoverColor } from '../../composables/useCoverColor';
import LyricsView from '../shared/LyricsView.vue';

/**
 * 移动端全屏沉浸式播放页。
 *
 * 从 Mini Player 底部上滑展开，下滑收起。
 * 基于桌面 NowPlayingImmersive 适配为移动端单列布局。
 *
 * 布局（从上到下）：
 *   ┌───────────────────┐
 *   │  ⌄ 收起           │  Top safe-area + collapse button
 *   ├───────────────────┤
 *   │                   │
 *   │    ┌────────┐     │  大封面（居中，max-w-[280px]）
 *   │    │  封面   │     │
 *   │    └────────┘     │
 *   │                   │
 *   │  歌曲名称          │  20px Bold
 *   │  艺术家 · 专辑     │  14px Secondary
 *   │                   │
 *   │  ═════●══════     │  进度条（touch-draggable range）
 *   │  1:23     3:42    │  Mono time
 *   │                   │
 *   │  🔀 ⏮ ▶ ⏭ ♡     │  Transport controls
 *   │                   │
 *   │  ── 歌词 ──      │  Lyrics（scrollable）
 *   │  前一句            │
 *   │  > 当前行 <        │  Accent highlight
 *   │  下一句            │
 *   │                   │
 *   └───────────────────┘
 */

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 封面 + 取色背景 ============ */

const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);
const { primary, ready } = useCoverColor(() => coverSrc.value || null);

const FALLBACK_BG = '#2A2722';
const bgPrimary = computed(() => (ready.value && primary.value ? primary.value : FALLBACK_BG));

/* ============ 进度条 ============ */

const currentTimeText = computed(() => formatMs(playerStore.progressMs));
const totalTimeText = computed(() => formatMs(playerStore.durationMs));

function formatMs(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

/* ============ 进度条拖拽 ============ */

const progressRef = ref<HTMLInputElement | null>(null);

function onProgressInput(e: Event) {
  const input = e.target as HTMLInputElement;
  if (!playerStore.durationMs) return;
  playerStore.seek(Math.floor(Number(input.value)));
}

/* ============ 播放模式 ============ */

function cycleMode() {
  const modes = ['normal', 'repeat', 'repeat-one', 'shuffle'] as const;
  const idx = modes.indexOf(playerStore.playMode as typeof modes[number]);
  playerStore.playMode = modes[(idx + 1) % modes.length];
}

const modeIcon = computed(() => {
  switch (playerStore.playMode) {
    case 'repeat': return Repeat;
    case 'repeat-one': return Repeat1;
    case 'shuffle': return Shuffle;
    default: return Repeat;
  }
});

const modeActive = computed(() => playerStore.playMode !== 'normal');

/* ============ 收藏 ============ */

const trackIsFav = computed(() => playerStore.currentTrack?.isFavorite ?? false);

function toggleFav() {
  const t = playerStore.currentTrack;
  if (t) playerStore.toggleFavorite(t.id);
}

/* ============ 元数据 ============ */

function fileInfoText(): string {
  const t = playerStore.currentTrack;
  if (!t) return '';
  const parts: string[] = [];
  if (t.format) parts.push(t.format.toUpperCase());
  return parts.join(' · ');
}

/* ============ 手势下滑收起 ============ */

const touchStartY = ref(0);

function onSwipeStart(e: TouchEvent) {
  // Only track single-finger vertical swipes
  if (e.touches.length === 1) {
    touchStartY.value = e.touches[0].clientY;
  }
}

function onSwipeEnd(e: TouchEvent) {
  if (!e.changedTouches.length) return;
  const deltaY = e.changedTouches[0].clientY - touchStartY.value;
  // Swipe down > 80px → dismiss
  if (deltaY > 80) {
    exit();
  }
}

function exit() {
  uiStore.closeImmersiveView();
}

/* ============ 键盘 ============ */

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault();
    exit();
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-[200] flex flex-col overflow-hidden select-none"
    :style="{ background: bgPrimary }"
    @touchstart="onSwipeStart"
    @touchend="onSwipeEnd"
    @keydown="onKey"
  >
    <!-- 背景三层叠（与桌面 NowPlayingImmersive 一致） -->
    <!-- 1. 高斯模糊封面 -->
    <img
      v-if="coverSrc"
      :src="coverSrc"
      alt=""
      class="absolute inset-0 w-full h-full object-cover scale-150 blur-[80px] opacity-50 transition-opacity duration-500 pointer-events-none"
    />

    <!-- 2. 主色叠加 -->
    <div
      class="absolute inset-0 pointer-events-none transition-colors duration-500"
      :style="{ background: bgPrimary, opacity: 0.55, mixBlendMode: 'color' }"
    ></div>

    <!-- 3. 暗化层 -->
    <div class="absolute inset-0 bg-black/35 pointer-events-none"></div>

    <!-- ===== 顶部：收起按钮 ===== -->
    <div
      class="relative z-10 flex items-center flex-shrink-0 px-4"
      :style="{ height: '56px', paddingTop: 'env(safe-area-inset-top)' }"
    >
      <button
        class="w-10 h-10 flex items-center justify-center rounded-[8px] text-white/70 active:text-white active:bg-white/10 transition-colors-smooth"
        title="收起"
        aria-label="收起播放页"
        @click="exit"
      >
        <ChevronDown class="w-[22px] h-[22px]" aria-hidden="true" />
      </button>
    </div>

    <!-- ===== 空队列占位 ===== -->
    <div
      v-if="!playerStore.currentTrack"
      class="relative z-10 flex-1 flex flex-col items-center justify-center gap-3 text-white/60"
    >
      <Disc3 class="w-10 h-10 text-white/40" aria-hidden="true" />
      <span class="text-[15px]">未在播放</span>
    </div>

    <!-- ===== 内容：单列纵向布局 ===== -->
    <div v-else class="relative z-10 flex-1 overflow-y-auto flex flex-col items-center px-6">
      <!-- 封面 -->
      <div
        class="relative aspect-square w-[75%] max-w-[280px] rounded-[10px] overflow-hidden bg-white/10 mb-5 flex-shrink-0"
      >
        <img
          v-if="coverSrc"
          :src="coverSrc"
          alt="cover"
          class="w-full h-full object-cover"
        />
        <Disc3
          v-else
          class="w-12 h-12 text-white/40 absolute inset-0 m-auto"
          aria-hidden="true"
        />
      </div>

      <!-- 曲名 + 艺术家 -->
      <h1 class="text-[20px] font-bold text-white leading-tight text-center mb-1 truncate w-full">
        {{ playerStore.currentTrack.title }}
      </h1>
      <p class="text-[14px] text-white/80 text-center mb-0.5 truncate w-full">
        {{ playerStore.currentTrack.artist }}
      </p>
      <p class="text-[13px] text-white/50 text-center mb-0.5 truncate w-full">
        {{ playerStore.currentTrack.album }}
      </p>
      <p v-if="fileInfoText()" class="text-[11px] font-mono uppercase tracking-wider text-white/40 mb-4">
        {{ fileInfoText() }}
      </p>

      <!-- 进度条 -->
      <div class="w-full max-w-[360px] flex items-center gap-3 mb-4">
        <span class="text-[10px] font-mono text-white/60 w-9 text-right tabular-nums">{{ currentTimeText }}</span>
        <input
          ref="progressRef"
          type="range"
          min="0"
          :max="playerStore.durationMs || 0"
          :value="playerStore.progressMs"
          class="immersive-progress flex-1"
          :disabled="!playerStore.durationMs"
          @input="onProgressInput"
        />
        <span class="text-[10px] font-mono text-white/60 w-9 text-left tabular-nums">{{ totalTimeText }}</span>
      </div>

      <!-- Transport 控制 -->
      <div class="flex items-center gap-8 mb-5">
        <!-- 播放模式 -->
        <button
          class="transition-colors-smooth"
          :class="modeActive ? 'text-brand-orange' : 'text-white/55 active:text-white'"
          :title="`播放模式: ${playerStore.playMode}`"
          aria-label="切换播放模式"
          @click="cycleMode"
        >
          <component :is="modeIcon" class="w-[18px] h-[18px]" aria-hidden="true" />
        </button>

        <!-- 上一首 -->
        <button
          class="text-white active:text-brand-orange transition-colors-smooth"
          aria-label="上一首"
          @click="playerStore.prevTrack()"
        >
          <SkipBack class="w-[22px] h-[22px] fill-current" aria-hidden="true" />
        </button>

        <!-- Play/Pause（Primary 白底黑图标） -->
        <button
          class="w-[56px] h-[56px] rounded-full bg-white text-black flex items-center justify-center active:opacity-80 transition-opacity"
          :disabled="!playerStore.currentTrack"
          :aria-label="playerStore.isPlaying ? '暂停' : '播放'"
          @click="playerStore.togglePlay()"
        >
          <Pause v-if="playerStore.isPlaying" class="w-[26px] h-[26px] fill-current" aria-hidden="true" />
          <Play v-else class="w-[26px] h-[26px] fill-current ml-0.5" aria-hidden="true" />
        </button>

        <!-- 下一首 -->
        <button
          class="text-white active:text-brand-orange transition-colors-smooth"
          aria-label="下一首"
          @click="playerStore.nextTrack()"
        >
          <SkipForward class="w-[22px] h-[22px] fill-current" aria-hidden="true" />
        </button>

        <!-- 收藏 -->
        <button
          class="transition-colors-smooth"
          :class="trackIsFav ? 'text-brand-orange' : 'text-white/55 active:text-white'"
          :aria-label="trackIsFav ? '取消收藏' : '收藏'"
          @click="toggleFav"
        >
          <Heart v-if="trackIsFav" class="w-[18px] h-[18px] fill-current" aria-hidden="true" />
          <Heart v-else class="w-[18px] h-[18px]" aria-hidden="true" />
        </button>
      </div>

      <!-- 歌词 -->
      <div class="w-full flex-1 min-h-0 pb-8" style="padding-bottom: calc(env(safe-area-inset-bottom) + 32px);">
        <LyricsView variant="immersive" />
      </div>
    </div>
  </div>
</template>
