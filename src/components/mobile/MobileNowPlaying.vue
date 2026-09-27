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
import EqualizerIndicator from '../shared/EqualizerIndicator.vue';
import ActionSheet from './ActionSheet.vue';
import type { ActionItem } from './ActionSheet.vue';

/**
 * 移动端全屏沉浸式播放页。
 *
 * 从 Mini Player 底部上滑展开，下滑收起。
 * 基于桌面 NowPlayingImmersive 适配为移动端单列布局。
 *
 * 视觉增强特性：
 *   - 封面背部动态流光色晕场（Aura Halo），伴随音乐节奏呼吸流动
 *   - 曲目标题旁的动态频谱均衡器（EqualizerIndicator）
 *   - 细腻毛玻璃磨砂遮罩与自适应主色晕染
 *   - 进度条拖动防抖 Scrub 优化（松手才提交 seek，保护音频链路）
 *   - 播放倍速触控 ActionSheet
 */

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 封面 + 取色背景 ============ */

const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);
const { primary, secondary, ready } = useCoverColor(() => coverSrc.value || null);

const FALLBACK_BG = '#2A2722';
const bgPrimary = computed(() => (ready.value && primary.value ? primary.value : FALLBACK_BG));
const bgSecondary = computed(() => (ready.value && secondary.value ? secondary.value : '#E28A23'));

/* ============ 进度条与防抖 Scrub ============ */

const scrubMs = ref<number | null>(null);
const displayProgressMs = computed(() => scrubMs.value ?? playerStore.progressMs);

const currentTimeText = computed(() => formatMs(displayProgressMs.value));
const totalTimeText = computed(() => formatMs(playerStore.durationMs));

function formatMs(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

const progressRef = ref<HTMLInputElement | null>(null);

function onProgressInput(e: Event) {
  const input = e.target as HTMLInputElement;
  scrubMs.value = Math.floor(Number(input.value));
}

function onProgressChange() {
  if (scrubMs.value !== null) {
    playerStore.seek(scrubMs.value);
    scrubMs.value = null;
  }
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
  if (e.touches.length === 1) {
    touchStartY.value = e.touches[0].clientY;
  }
}

function onSwipeEnd(e: TouchEvent) {
  if (!e.changedTouches.length) return;
  const deltaY = e.changedTouches[0].clientY - touchStartY.value;
  // 下滑超过 80px 收起沉浸页
  if (deltaY > 80) {
    exit();
  }
}

function exit() {
  uiStore.closeImmersiveView();
}

/* ============ 封面左右横滑切歌（MA4 A4-5） ============ */

const coverTouchStartX = ref(0);
const coverOffsetX = ref(0);
const isSwipingCover = ref(false);

function onCoverTouchStart(e: TouchEvent) {
  if (e.touches.length === 1) {
    coverTouchStartX.value = e.touches[0].clientX;
    isSwipingCover.value = true;
    coverOffsetX.value = 0;
  }
}

function onCoverTouchMove(e: TouchEvent) {
  if (!isSwipingCover.value || !e.touches.length) return;
  const deltaX = e.touches[0].clientX - coverTouchStartX.value;
  coverOffsetX.value = Math.max(-50, Math.min(50, deltaX * 0.4));
}

function onCoverTouchEnd(e: TouchEvent) {
  if (!isSwipingCover.value) return;
  isSwipingCover.value = false;
  if (!e.changedTouches.length) {
    coverOffsetX.value = 0;
    return;
  }
  const deltaX = e.changedTouches[0].clientX - coverTouchStartX.value;
  coverOffsetX.value = 0;
  if (deltaX < -80) {
    playerStore.nextTrack();
  } else if (deltaX > 80) {
    playerStore.prevTrack();
  }
}

/* ============ 播放倍速 ============ */
const showSpeedSheet = ref(false);
const speedActions = computed<ActionItem[]>(() => {
  return playerStore.PLAYBACK_RATES.map(rate => ({
    label: `${rate}x${rate === 1.0 ? '（正常速度）' : ''}`,
    onClick: () => playerStore.setPlaybackRate(rate),
  }));
});

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
    <!-- 1. 高斯模糊封面大背景 -->
    <img
      v-if="coverSrc"
      :src="coverSrc"
      alt=""
      class="absolute inset-0 w-full h-full object-cover scale-150 blur-[90px] opacity-45 transition-opacity duration-700 pointer-events-none"
    />

    <!-- 2. 主色与次色渐变混色层 -->
    <div
      class="absolute inset-0 pointer-events-none transition-colors duration-700 opacity-60 mix-blend-color"
      :style="{ background: `radial-gradient(circle at 50% 35%, ${bgSecondary}, ${bgPrimary})` }"
    ></div>

    <!-- 3. 暗化与半透明磨砂层 -->
    <div class="absolute inset-0 bg-black/40 backdrop-blur-[2px] pointer-events-none"></div>

    <!-- ===== 顶部：收起按钮 ===== -->
    <div
      class="relative z-10 flex items-center flex-shrink-0 px-4"
      :style="{ height: 'calc(56px + env(safe-area-inset-top, 0px))', paddingTop: 'env(safe-area-inset-top, 0px)' }"
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
      
      <!-- 封面区包装：带背部色晕场 (Aura Halo) + 横滑切歌 -->
      <div class="relative w-[75%] max-w-[280px] aspect-square mb-5 flex-shrink-0 flex items-center justify-center">
        <!-- 背部呼吸光晕场 -->
        <div
          class="mobile-aura-container pointer-events-none"
          :class="{ 'is-playing': playerStore.isPlaying }"
        >
          <div
            class="mobile-aura-blob mobile-aura-blob-1"
            :style="{ background: bgPrimary }"
          ></div>
          <div
            class="mobile-aura-blob mobile-aura-blob-2"
            :style="{ background: bgSecondary }"
          ></div>
        </div>

        <!-- 封面卡片 -->
        <div
          class="relative w-full h-full rounded-[14px] overflow-hidden bg-white/10 shadow-2xl transition-transform duration-150 ease-out z-10 border border-white/10"
          :style="{ transform: `translateX(${coverOffsetX}px)` }"
          @touchstart.stop="onCoverTouchStart"
          @touchmove.stop="onCoverTouchMove"
          @touchend.stop="onCoverTouchEnd"
        >
          <img
            v-if="coverSrc"
            :src="coverSrc"
            alt="cover"
            class="w-full h-full object-cover select-none pointer-events-none"
          />
          <Disc3
            v-else
            class="w-12 h-12 text-white/40 absolute inset-0 m-auto"
            aria-hidden="true"
          />
        </div>
      </div>

      <!-- 曲名 + 律动均衡器 -->
      <div class="w-full flex items-center justify-center gap-2 mb-1 px-2">
        <h1 class="text-[20px] font-bold text-white leading-tight truncate text-center">
          {{ playerStore.currentTrack.title }}
        </h1>
        <EqualizerIndicator :playing="playerStore.isPlaying" size="sm" class="shrink-0 text-brand-orange" />
      </div>

      <!-- 艺术家与专辑 -->
      <p class="text-[14px] text-white/80 text-center mb-0.5 truncate w-full">
        {{ playerStore.currentTrack.artist }}
      </p>
      <p class="text-[13px] text-white/50 text-center mb-0.5 truncate w-full">
        {{ playerStore.currentTrack.album }}
      </p>
      <p v-if="fileInfoText()" class="text-[11px] font-mono uppercase tracking-wider text-white/40 mb-3">
        {{ fileInfoText() }}
      </p>

      <!-- 进度条（带拖动防抖 Scrub 优化） -->
      <div class="w-full max-w-[360px] flex items-center gap-3 mb-1">
        <span class="text-[10px] font-mono text-white/60 w-9 text-right tabular-nums">{{ currentTimeText }}</span>
        <input
          ref="progressRef"
          type="range"
          min="0"
          :max="playerStore.durationMs || 0"
          :value="displayProgressMs"
          class="immersive-progress flex-1"
          :disabled="!playerStore.durationMs"
          @input="onProgressInput"
          @change="onProgressChange"
        />
        <span class="text-[10px] font-mono text-white/60 w-9 text-left tabular-nums">{{ totalTimeText }}</span>
      </div>

      <!-- 倍速标签：非 1.0x 时高亮品牌橙 -->
      <div class="w-full max-w-[360px] flex justify-end mb-2">
        <button
          class="h-7 px-2.5 rounded-full text-[11px] font-mono font-medium transition-colors-smooth"
          :class="playerStore.playbackRate !== 1.0
            ? 'text-brand-orange bg-brand-orange/20 border border-brand-orange/30'
            : 'text-white/50 active:text-white'"
          @click="showSpeedSheet = true"
        >
          {{ playerStore.playbackRate }}x
        </button>
      </div>

      <!-- Transport 控制栏 -->
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
          class="w-[56px] h-[56px] rounded-full bg-white text-black flex items-center justify-center active:opacity-80 transition-opacity shadow-lg"
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

    <!-- 倍速选择 ActionSheet -->
    <ActionSheet
      :visible="showSpeedSheet"
      :actions="speedActions"
      @close="showSpeedSheet = false"
    />
  </div>
</template>

<style scoped>
/* 移动端背部流光呼吸色晕场（Aura Halo） */
.mobile-aura-container {
  position: absolute;
  inset: -15%;
  width: 130%;
  height: 130%;
  display: flex;
  align-items: center;
  justify-content: center;
  filter: blur(48px);
  opacity: 0.65;
  transition: opacity 0.6s ease;
  z-index: 0;
}

.mobile-aura-blob {
  position: absolute;
  transform-origin: center center;
  will-change: transform, opacity;
  animation-play-state: paused;
}

.mobile-aura-container.is-playing .mobile-aura-blob {
  animation-play-state: running;
}

.mobile-aura-blob-1 {
  width: 85%;
  height: 85%;
  border-radius: 46% 54% 65% 35% / 40% 48% 52% 60%;
  animation: mobile-aura-flow-1 8s ease-in-out infinite;
}

.mobile-aura-blob-2 {
  width: 75%;
  height: 75%;
  border-radius: 60% 40% 30% 70% / 50% 60% 40% 50%;
  animation: mobile-aura-flow-2 10s ease-in-out infinite;
}

@keyframes mobile-aura-flow-1 {
  0% {
    transform: translate(0, 0) scale(1) rotate(0deg);
  }
  33% {
    transform: translate(8%, -6%) scale(1.08) rotate(120deg);
  }
  66% {
    transform: translate(-6%, 8%) scale(0.94) rotate(240deg);
  }
  100% {
    transform: translate(0, 0) scale(1) rotate(360deg);
  }
}

@keyframes mobile-aura-flow-2 {
  0% {
    transform: translate(0, 0) scale(1) rotate(0deg);
  }
  33% {
    transform: translate(-7%, 6%) scale(1.05) rotate(-120deg);
  }
  66% {
    transform: translate(6%, -5%) scale(0.96) rotate(-240deg);
  }
  100% {
    transform: translate(0, 0) scale(1) rotate(-360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .mobile-aura-blob {
    animation: none !important;
  }
}
</style>
