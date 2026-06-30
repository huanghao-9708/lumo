<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import {
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1,
  ChevronDown, Disc3, Heart, Volume, Volume1, Volume2,
  Minus, Square, X,
} from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useCoverColor } from '../../composables/useCoverColor';
import LyricsView from '../shared/LyricsView.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 窗口控制（复用 TopBar 的方式） ============ */
const appWindow = getCurrentWindow();
const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();

/* ============ 封面 + 主色提取 ============ */
const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);
const { primary, secondary, ready } = useCoverColor(() => coverSrc.value || null);

/**
 * 取色失败（canvas 被自定义协议 URL 污染）或封面未加载完时，
 * 回落到兜底色，保证背景不空白。
 */
const FALLBACK_BG = '#2A2722';
const bgPrimary = computed(() => (ready.value && primary.value ? primary.value : FALLBACK_BG));
const bgSecondary = computed(() => (ready.value && secondary.value ? secondary.value : FALLBACK_BG));

/* ============ 进度条 ============ */
const currentTimeText = computed(() => formatMs(playerStore.progressMs));
const totalTimeText = computed(() => formatMs(playerStore.durationMs));

function formatMs(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

/* ============ 播放模式 ============ */
function cycleMode() {
  const modes = ['normal', 'repeat', 'repeat-one', 'shuffle'] as const;
  const idx = modes.indexOf(playerStore.playMode as any);
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

/* ============ 音量（滚轮调节） ============ */
const volumeIcon = computed(() => {
  if (playerStore.volume === 0) return Volume;
  if (playerStore.volume < 40) return Volume1;
  return Volume2;
});
function onVolumeWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY < 0 ? 5 : -5;
  playerStore.setVolume(Math.max(0, Math.min(100, playerStore.volume + delta)));
}

/* ============ 当前轨道信息 ============ */
function fileInfoText(): string {
  const fi = playerStore.currentTrackFileInfo as any;
  const parts: string[] = [];
  if (fi?.release_year) parts.push(String(fi.release_year));
  if (playerStore.currentTrack?.format) parts.push(playerStore.currentTrack.format);
  if (fi?.bits_per_sample && fi?.sample_rate) {
    parts.push(`${fi.bits_per_sample}bit / ${(fi.sample_rate / 1000).toFixed(0)}kHz`);
  }
  return parts.join(' · ');
}

/* ============ 收藏 ============ */
const trackIsFav = computed(() => playerStore.currentTrack?.isFavorite ?? false);
function toggleFav() {
  const t = playerStore.currentTrack;
  if (t) playerStore.toggleFavorite(t.id);
}

/* ============ 退出（进入/退出动画由 App.vue 的 <Transition> 控制） ============ */
function exit() {
  uiStore.closeImmersiveView();
}
function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault();
    exit();
  }
}
onMounted(() => window.addEventListener('keydown', onKey));
onUnmounted(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div
    class="fixed inset-0 z-[200] flex flex-col overflow-hidden now-playing-immersive"
    :style="{ background: bgPrimary }"
  >
    <!-- ===== 背景三层叠 ===== -->
    <!-- 1. 最底：全屏高斯模糊封面铺底 -->
    <img
      v-if="coverSrc"
      :src="coverSrc"
      alt=""
      class="absolute inset-0 w-full h-full object-cover scale-150 blur-[90px] opacity-55 transition-opacity duration-500 pointer-events-none"
    />
    <!-- 2. 中：主色纯色叠加，把取色染进模糊封面 -->
    <div
      class="absolute inset-0 pointer-events-none transition-colors duration-500"
      :style="{ background: bgPrimary, opacity: 0.55, mixBlendMode: 'color' }"
    ></div>
    <!-- 3. 顶：辅色径向晕影，加深氛围 -->
    <div
      class="absolute inset-0 pointer-events-none transition-opacity duration-500"
      :style="{
        background: `radial-gradient(ellipse at 50% 30%, transparent 0%, ${bgSecondary} 120%)`,
        opacity: 0.5,
      }"
    ></div>
    <!-- 4. 暗化层：保证前景白字可读 -->
    <div class="absolute inset-0 bg-black/35 pointer-events-none"></div>

    <!-- ===== 顶部栏：左侧收起按钮 + 右侧窗口控制（可拖拽） ===== -->
    <div
      class="relative z-10 h-[60px] flex-shrink-0 flex items-center justify-between px-4 select-none"
      data-tauri-drag-region
    >
      <!-- 左：收起（退出沉浸式） -->
      <div class="pointer-events-auto">
        <button
          class="h-8 px-3 flex items-center gap-1.5 rounded-[8px] text-white/70 hover:text-white hover:bg-white/10 transition-colors-smooth text-[12px] font-medium"
          title="收起沉浸式 (Esc)"
          @click="exit"
        >
          <ChevronDown class="w-[18px] h-[18px]" />
          收起
        </button>
      </div>

      <!-- 右：窗口控制三件套 -->
      <div class="flex items-center gap-1 pointer-events-auto">
        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-white/70 hover:text-white hover:bg-white/10 transition-colors-smooth"
          title="最小化"
          @click="minimize"
        >
          <Minus class="w-4 h-4" />
        </button>
        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-white/70 hover:text-white hover:bg-white/10 transition-colors-smooth"
          title="最大化"
          @click="toggleMaximize"
        >
          <Square class="w-3.5 h-3.5" />
        </button>
        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-white/70 hover:text-white hover:bg-[#E81123] transition-colors-smooth"
          title="关闭"
          @click="close"
        >
          <X class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- ===== 内容区：三栏，左右镜像拥抱中间封面，严格对称 ===== -->
    <div
      v-if="playerStore.currentTrack"
      class="relative z-10 flex-1 min-h-0 grid grid-cols-[1fr_auto_1fr] gap-6 px-8 pb-2"
    >
      <!-- 左：歌曲信息区（贴中线对齐） -->
      <div class="flex flex-col justify-end pb-10 max-w-[340px] w-full justify-self-end">
        <p class="text-[10px] font-mono uppercase tracking-[0.2em] text-white/50 mb-3">Now Playing</p>
        <h1 class="text-[30px] font-bold text-white leading-[1.15] mb-2 break-words">
          {{ playerStore.currentTrack.title }}
        </h1>
        <p class="text-[17px] text-white/85 mb-0.5 truncate">{{ playerStore.currentTrack.artist }}</p>
        <p class="text-[13px] text-white/55 mb-3 truncate">{{ playerStore.currentTrack.album }}</p>
        <p v-if="fileInfoText()" class="text-[11px] font-mono uppercase tracking-wider text-white/45 mb-4">
          {{ fileInfoText() }}
        </p>

        <button
          class="self-start flex items-center gap-2 text-white/80 hover:text-white transition-colors-smooth text-[13px] font-medium"
          @click="toggleFav"
        >
          <Heart
            class="w-[18px] h-[18px]"
            :class="trackIsFav ? 'text-brand-orange fill-current' : ''"
          />
          {{ trackIsFav ? '已收藏' : '收藏' }}
        </button>
      </div>

      <!-- 中：封面（正方形四边羽化嵌入内容区） -->
      <div class="flex items-center justify-center min-w-0">
        <div class="relative aspect-square h-[min(58vh,500px)] w-[min(58vh,500px)] square-feather">
          <img
            v-if="coverSrc"
            :src="coverSrc"
            alt="cover"
            class="w-full h-full object-cover"
          />
          <div
            v-else
            class="w-full h-full flex items-center justify-center"
          >
            <Disc3 class="w-16 h-16 text-white/60 animate-spin" style="animation-duration: 8s;" />
          </div>
        </div>
      </div>

      <!-- 右：歌词滚动（贴中线对齐，与左栏等宽镜像） -->
      <div class="flex flex-col min-h-0 max-w-[340px] w-full justify-self-start">
        <LyricsView variant="immersive" />
      </div>
    </div>

    <!-- 空队列占位 -->
    <div v-else class="relative z-10 flex-1 flex flex-col items-center justify-center gap-3 text-white/60">
      <Disc3 class="w-10 h-10 text-white/40" />
      <span class="text-[14px]">未在播放</span>
    </div>

    <!-- ===== 底部播放栏（无封面，左侧歌名作退出入口） ===== -->
    <div class="relative z-10 h-[96px] flex-shrink-0 flex items-center px-8 border-t border-white/10 backdrop-blur-sm bg-black/15">
      <!-- 左：歌名（点击退出） -->
      <button
        class="w-[260px] flex-shrink-0 flex flex-col justify-center text-left group min-w-0"
        title="点击退出沉浸式"
        @click="exit"
      >
        <span class="text-[10px] font-mono uppercase tracking-wider text-white/45 mb-0.5 flex items-center gap-1">
          <ChevronDown class="w-3 h-3" /> 点击收起
        </span>
        <span class="text-[14px] font-semibold text-white truncate group-hover:text-brand-orange transition-colors-smooth">
          {{ playerStore.currentTrack?.title ?? '未在播放' }}
        </span>
        <span class="text-[11px] text-white/55 truncate">
          {{ playerStore.currentTrack?.artist }}
        </span>
      </button>

      <!-- 中：控制 + 进度条 -->
      <div class="flex-1 flex flex-col items-center justify-center px-8">
        <div class="flex items-center gap-7 mb-2.5">
          <button
            class="transition-colors-smooth"
            :class="modeActive ? 'text-brand-orange' : 'text-white/55 hover:text-white'"
            :title="`播放模式: ${playerStore.playMode}`"
            @click="cycleMode"
          >
            <component :is="modeIcon" class="w-[17px] h-[17px]" />
          </button>
          <button class="text-white hover:text-brand-orange transition-colors-smooth" @click="playerStore.prevTrack()" title="上一首">
            <SkipBack class="w-[20px] h-[20px] fill-current" />
          </button>
          <button
            class="w-[50px] h-[50px] rounded-full bg-white text-black flex items-center justify-center hover:opacity-90 transition-opacity"
            :disabled="!playerStore.currentTrack"
            @click="playerStore.togglePlay()"
          >
            <Pause v-if="playerStore.isPlaying" class="w-[22px] h-[22px] fill-current" />
            <Play v-else class="w-[22px] h-[22px] fill-current ml-0.5" />
          </button>
          <button class="text-white hover:text-brand-orange transition-colors-smooth" @click="playerStore.nextTrack()" title="下一首">
            <SkipForward class="w-[20px] h-[20px] fill-current" />
          </button>
          <div class="w-[17px]"></div>
        </div>

        <!-- 进度条（原生 range，橙色 thumb） -->
        <div class="w-full flex items-center gap-3 max-w-2xl">
          <span class="text-[10px] font-mono text-white/60 w-9 text-right tabular-nums">{{ currentTimeText }}</span>
          <input
            type="range"
            min="0"
            :max="playerStore.durationMs || 0"
            :value="playerStore.progressMs"
            class="immersive-progress flex-1"
            :disabled="!playerStore.durationMs"
            @input="playerStore.seek(Math.floor(Number(($event.target as HTMLInputElement).value)))"
          />
          <span class="text-[10px] font-mono text-white/60 w-9 text-left tabular-nums">{{ totalTimeText }}</span>
        </div>
      </div>

      <!-- 右：音量 -->
      <div class="flex items-center gap-3 flex-shrink-0 w-[200px] justify-end">
        <component :is="volumeIcon" class="w-[16px] h-[16px] text-white/60" />
        <input
          type="range"
          min="0"
          max="100"
          :value="playerStore.volume"
          class="immersive-progress w-[110px]"
          @input="playerStore.setVolume(Math.floor(Number(($event.target as HTMLInputElement).value)))"
          @wheel="onVolumeWheel"
        />
        <span class="text-[10px] font-mono text-white/60 w-7 tabular-nums">{{ playerStore.volume }}</span>
      </div>
    </div>
  </div>
</template>

<style>
/* 沉浸式页面一次性内联样式块（全局规范是无 scoped CSS，此处为原生 range 滑块美化
   + 正方形羽化遮罩的例外）。动画遵循 LDL：仅 250ms ease-out。 */

/* 正方形四边羽化：两条线性渐变相交，保留四个角，仅四边淡出 */
.square-feather {
  -webkit-mask-image:
    linear-gradient(to right, transparent 0%, #000 9%, #000 91%, transparent 100%),
    linear-gradient(to bottom, transparent 0%, #000 9%, #000 91%, transparent 100%);
  -webkit-mask-composite: source-in;
  mask-image:
    linear-gradient(to right, transparent 0%, #000 9%, #000 91%, transparent 100%),
    linear-gradient(to bottom, transparent 0%, #000 9%, #000 91%, transparent 100%);
  mask-composite: intersect;
}

/* 原生 range 滑块美化（深色背景下用白色轨道 + 橙色 thumb） */
.immersive-progress {
  -webkit-appearance: none;
  appearance: none;
  height: 3px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.2);
  outline: none;
  cursor: pointer;
}
.immersive-progress:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.immersive-progress::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #e28a23;
  border: none;
  cursor: pointer;
  box-shadow: 0 0 0 3px rgba(226, 138, 35, 0.2);
}
.immersive-progress::-moz-range-thumb {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #e28a23;
  border: none;
  cursor: pointer;
}
</style>
