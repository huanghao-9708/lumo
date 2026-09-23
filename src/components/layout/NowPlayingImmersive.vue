<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import type { CSSProperties } from 'vue';
import {
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1,
  ChevronDown, Disc3, Music, Heart, Volume, Volume1, Volume2,
  Minus, Square, X, ListPlus,
} from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useCoverColor } from '../../composables/useCoverColor';
import LyricsView from '../shared/LyricsView.vue';
import PlaylistPickerModal from '../shared/PlaylistPickerModal.vue';
import PlaybackRateButton from '../shared/PlaybackRateButton.vue';

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

/* ============ 方案一：浮光掠影 · 呼吸色晕场（Aura Halo） ============ */
const auraPrimaryColor = computed(() => (ready.value && primary.value ? primary.value : 'rgba(226, 138, 35, 0.55)'));
const auraSecondaryColor = computed(() => {
  if (ready.value && secondary.value) return secondary.value;
  if (ready.value && primary.value) return primary.value;
  return 'rgba(235, 110, 60, 0.45)';
});
const auraCoreColor = computed(() => (ready.value && primary.value ? primary.value : 'rgba(226, 138, 35, 0.38)'));

/**
 * 以曲目 id 作确定性种子：同一首歌的呼吸节奏恒定，不同歌略有差异。
 * - 周期 4.0–5.4s，微幅悬浮与轻微呼吸，完全在 GPU 合成器线程运行。
 */
const floatingStyle = computed<CSSProperties>(() => {
  const id = playerStore.currentTrack?.id ?? 0;
  const seed = (Math.abs(Math.imul(id, 2654435761)) % 1000) / 1000;
  return {
    '--np-float-duration': `${(4.0 + seed * 1.4).toFixed(2)}s`,
    '--np-float-scale': (1.012 + seed * 0.015).toFixed(4),
  } as CSSProperties;
});

/* ============ 进度条 ============ */
/**
 * 拖拽进度条时只更新本地预览值，松手（change）才真正 seek。
 * 原生 range 的 input 事件是逐像素触发的，每一次都发一次 IPC 会把通道打满
 * （详见 IPC 性能一节），而且后端 seek 本身要等音频线程，卡顿会非常明显。
 */
const scrubMs = ref<number | null>(null);

function commitScrub() {
  if (scrubMs.value == null) return;
  playerStore.seek(scrubMs.value);
  // seek 会立刻把 progressMs 设为目标值，这里清掉预览值不会造成回跳
  scrubMs.value = null;
}

const displayProgressMs = computed(() => scrubMs.value ?? playerStore.progressMs);
const currentTimeText = computed(() => formatMs(displayProgressMs.value));
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

/* ============ 当前轨道信息（音质 / 文件大小） ============ */
function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || bytes <= 0) return '';
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  return `${value >= 100 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

/** 音质：格式 + 位深/采样率（无损）或码率（有损）+ 声道 */
const qualityText = computed(() => {
  const track = playerStore.currentTrack;
  const fi = playerStore.currentTrackFileInfo;
  const parts: string[] = [];
  if (track?.format) parts.push(track.format);
  if (fi?.bit_depth && fi?.sample_rate) {
    const khz = fi.sample_rate % 1000 === 0 ? (fi.sample_rate / 1000).toFixed(0) : (fi.sample_rate / 1000).toFixed(1);
    parts.push(`${fi.bit_depth}bit / ${khz}kHz`);
  } else if (fi?.bitrate) {
    parts.push(`${Math.round(fi.bitrate / 1000)} kbps`);
  }
  if (fi?.channels === 1) parts.push('单声道');
  else if (fi?.channels === 2) parts.push('立体声');
  else if (fi?.channels) parts.push(`${fi.channels} 声道`);
  return parts.join(' · ');
});

/** 文件大小：优先物理文件信息，回退到曲目上缓存的值 */
const fileSizeText = computed(() =>
  formatBytes(playerStore.currentTrackFileInfo?.file_size ?? playerStore.currentTrack?.fileSize ?? null)
);

/* ============ 收藏 / 添加到歌单 ============ */
const trackIsFav = computed(() => playerStore.currentTrack?.isFavorite ?? false);
function toggleFav() {
  const t = playerStore.currentTrack;
  if (t) playerStore.toggleFavorite(t.id);
}

const showPlaylistPicker = ref(false);

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
onMounted(() => {
  window.addEventListener('keydown', onKey);
});
onUnmounted(() => {
  window.removeEventListener('keydown', onKey);
});
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

    <!-- ===== 内容区：三栏 —— 左(信息+收藏) · 中(封面) · 右(歌词)，全部居中，共享同一水平中线 ===== -->
    <div
      v-if="playerStore.currentTrack"
      class="relative z-10 flex-1 min-h-0 grid grid-cols-[1fr_auto_1fr] gap-8 px-8 pb-2"
    >
      <!-- 左：歌曲信息 + 收藏（在左侧空间内完全水平居中对齐） -->
      <div class="flex flex-col items-center justify-center text-center gap-2 max-w-[400px] w-full justify-self-center min-w-0">
        <h1 class="text-[26px] font-bold text-white leading-[1.2] break-words w-full">
          {{ playerStore.currentTrack.title }}
        </h1>
        <p class="text-[15px] text-white/85 truncate w-full">{{ playerStore.currentTrack.artist }}</p>
        <p class="text-[13px] text-white/55 truncate w-full">{{ playerStore.currentTrack.album }}</p>

        <!-- 音质 / 文件大小 -->
        <p v-if="qualityText" class="mt-1 text-[11px] font-mono uppercase tracking-wider text-white/45 w-full">
          {{ qualityText }}
        </p>
        <p v-if="fileSizeText" class="text-[11px] font-mono tracking-wider text-white/40 w-full">
          {{ fileSizeText }}
        </p>

        <div class="mt-3 flex items-center gap-5">
          <button
            class="flex items-center gap-2 text-white/80 hover:text-white transition-colors-smooth text-[13px] font-medium"
            :title="trackIsFav ? '取消收藏' : '收藏'"
            @click="toggleFav"
          >
            <Heart
              class="w-[18px] h-[18px]"
              :class="trackIsFav ? 'text-brand-orange fill-current' : ''"
            />
            {{ trackIsFav ? '已收藏' : '收藏' }}
          </button>

          <button
            class="flex items-center gap-2 text-white/80 hover:text-white transition-colors-smooth text-[13px] font-medium"
            title="添加到歌单"
            @click="showPlaylistPicker = true"
          >
            <ListPlus class="w-[18px] h-[18px]" />
            添加到歌单
          </button>
        </div>
      </div>

      <!-- 中：封面舞台（背部流光呼吸色晕场 + 悬浮主体卡片） -->
      <div class="flex items-center justify-center min-w-0 relative">
        <div
          class="relative flex items-center justify-center select-none"
          style="width: var(--np-cover); height: var(--np-cover);"
        >
          <!-- 1. 背部动态流光呼吸色晕场（Aura Halo） -->
          <div
            class="aura-container pointer-events-none"
            :class="{ 'is-playing': playerStore.isPlaying }"
          >
            <div
              class="aura-blob aura-blob-1"
              :style="{ background: auraPrimaryColor }"
            ></div>
            <div
              class="aura-blob aura-blob-2"
              :style="{ background: auraSecondaryColor }"
            ></div>
            <div
              class="aura-blob aura-blob-core"
              :style="{ background: auraCoreColor }"
            ></div>
          </div>

          <!-- 2. 悬浮封面主体卡片（现代实体感 + 微玻璃高光） -->
          <div
            class="relative z-10 shrink-0 cover-card cover-floating overflow-hidden rounded-2xl ring-1 ring-white/15"
            :class="{ 'is-playing': playerStore.isPlaying }"
            style="width: 100%; height: 100%;"
            :style="floatingStyle"
          >
            <img
              v-if="coverSrc"
              :src="coverSrc"
              alt="cover"
              class="w-full h-full object-cover pointer-events-none"
            />
            <div
              v-else
              class="w-full h-full flex flex-col items-center justify-center gap-2.5 bg-white/5 backdrop-blur-md"
            >
              <Music class="w-16 h-16 text-white/35" />
              <span class="text-[12px] font-mono tracking-wider text-white/35 uppercase">No Artwork</span>
            </div>

            <!-- 卡片表面微妙的玻璃斜高光（非旋转、纯静态光泽层） -->
            <div class="absolute inset-0 pointer-events-none cover-glass-sheen"></div>
          </div>
        </div>
      </div>

      <!-- 右：歌词（高度 = 封面高度，在右侧空间内完全水平居中对齐） -->
      <div class="flex flex-col items-center justify-center min-h-0 max-w-[400px] w-full justify-self-center text-center">
        <div class="lyrics-fade overflow-hidden w-full" style="height: var(--np-cover);">
          <LyricsView variant="immersive" />
        </div>
      </div>
    </div>

    <!-- 空队列占位 -->
    <div v-else class="relative z-10 flex-1 flex flex-col items-center justify-center gap-3 text-white/60">
      <Disc3 class="w-10 h-10 text-white/40" />
      <span class="text-[14px]">未在播放</span>
    </div>

    <!-- ===== 底部播放栏（无封面，左侧歌名作退出入口） ===== -->
    <div class="relative z-10 h-[96px] flex-shrink-0 flex items-center justify-between px-8 border-t border-white/10 backdrop-blur-md bg-black/20 select-none">
      <!-- 左：歌名（点击退出） -->
      <button
        class="w-[260px] flex-shrink-0 flex flex-col justify-center text-left group min-w-0 z-10"
        title="点击退出沉浸式 (Esc)"
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

      <!-- 中：控制 + 进度条（屏幕绝对物理居中） -->
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-[560px] max-w-[min(560px,44vw)] flex flex-col items-center justify-center z-20 pointer-events-none">
        <!-- 核心按钮行：完美轴对称分布 -->
        <div class="flex items-center justify-center gap-7 mb-2.5 pointer-events-auto">
          <!-- 播放模式 -->
          <button
            class="w-8 h-8 flex items-center justify-center rounded-full hover:bg-white/10 transition-colors-smooth"
            :class="modeActive ? 'text-brand-orange' : 'text-white/60 hover:text-white'"
            :title="`播放模式: ${playerStore.playMode}`"
            @click="cycleMode"
          >
            <component :is="modeIcon" class="w-[17px] h-[17px]" />
          </button>

          <!-- 上一首 -->
          <button
            class="w-8 h-8 flex items-center justify-center rounded-full text-white/90 hover:text-white hover:bg-white/10 active:scale-95 transition-all"
            @click="playerStore.prevTrack()"
            title="上一首"
          >
            <SkipBack class="w-[20px] h-[20px] fill-current" />
          </button>

          <!-- 播放/暂停大圆钮 -->
          <button
            class="w-[50px] h-[50px] rounded-full bg-white text-black flex items-center justify-center shadow-lg hover:scale-105 active:scale-95 transition-all disabled:opacity-40 disabled:hover:scale-100"
            :disabled="!playerStore.currentTrack"
            @click="playerStore.togglePlay()"
            :title="playerStore.isPlaying ? '暂停' : '播放'"
          >
            <Pause v-if="playerStore.isPlaying" class="w-[22px] h-[22px] fill-current" />
            <Play v-else class="w-[22px] h-[22px] fill-current ml-0.5" />
          </button>

          <!-- 下一首 -->
          <button
            class="w-8 h-8 flex items-center justify-center rounded-full text-white/90 hover:text-white hover:bg-white/10 active:scale-95 transition-all"
            @click="playerStore.nextTrack()"
            title="下一首"
          >
            <SkipForward class="w-[20px] h-[20px] fill-current" />
          </button>

          <!-- 收藏（与左侧播放模式形成优雅对称） -->
          <button
            class="w-8 h-8 flex items-center justify-center rounded-full hover:bg-white/10 transition-colors-smooth"
            :class="trackIsFav ? 'text-brand-orange' : 'text-white/60 hover:text-white'"
            :title="trackIsFav ? '取消收藏' : '收藏'"
            @click="toggleFav"
          >
            <Heart class="w-[17px] h-[17px]" :class="trackIsFav ? 'fill-current' : ''" />
          </button>
        </div>

        <!-- 进度条（原生 range，橙色 thumb） -->
        <div class="w-full flex items-center gap-3 pointer-events-auto">
          <span class="text-[10px] font-mono text-white/60 w-10 text-right tabular-nums select-none">{{ currentTimeText }}</span>
          <input
            type="range"
            min="0"
            :max="playerStore.durationMs || 0"
            :value="scrubMs ?? playerStore.progressMs"
            class="immersive-progress flex-1"
            :disabled="!playerStore.durationMs"
            @input="scrubMs = Math.floor(Number(($event.target as HTMLInputElement).value))"
            @change="commitScrub"
          />
          <span class="text-[10px] font-mono text-white/60 w-10 text-left tabular-nums select-none">{{ totalTimeText }}</span>
        </div>
      </div>

      <!-- 右：播放速度 + 音量 -->
      <div class="flex items-center gap-3 flex-shrink-0 w-[260px] justify-end z-10">
        <PlaybackRateButton variant="light" />
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
        <span class="text-[10px] font-mono text-white/60 w-7 tabular-nums select-none">{{ playerStore.volume }}</span>
      </div>
    </div>

    <!-- 添加到歌单（AppModal 走 Teleport，放在这一层里不会影响布局，也不会被 z-200 压住） -->
    <PlaylistPickerModal
      v-if="showPlaylistPicker && playerStore.currentTrack"
      :track-ids="[playerStore.currentTrack.id]"
      :track-title="playerStore.currentTrack.title"
      @close="showPlaylistPicker = false"
    />
  </div>
</template>

<style>
/* 沉浸式页面一次性内联样式块（全局规范是无 scoped CSS，此处为原生 range 滑块美化
   + 正方形羽化遮罩的例外）。动画遵循 LDL：仅 250ms ease-out。 */

/* 版面基准尺寸：中间封面 = 右侧歌词区高度 */
.now-playing-immersive {
  --np-cover: min(56vh, 480px);
}

/* ===== 方案一：背部流光呼吸色晕场（Aura Halo） ===== */
.aura-container {
  position: absolute;
  inset: -14%;
  width: 128%;
  height: 128%;
  display: flex;
  align-items: center;
  justify-content: center;
  filter: blur(52px);
  opacity: 0.85;
  transition: opacity 0.6s ease;
  z-index: 0;
}

.aura-blob {
  position: absolute;
  transform-origin: center center;
  will-change: transform, opacity;
  animation-play-state: paused;
}

.aura-container.is-playing .aura-blob {
  animation-play-state: running;
}

/* 光团 1：主色流光团，顺时针椭圆慢巡游 */
.aura-blob-1 {
  width: 82%;
  height: 82%;
  border-radius: 46% 54% 65% 35% / 40% 48% 52% 60%;
  animation: aura-flow-1 8.5s ease-in-out infinite;
}

/* 光团 2：次色流光团，逆时针交错慢巡游 */
.aura-blob-2 {
  width: 76%;
  height: 76%;
  border-radius: 58% 42% 38% 62% / 55% 38% 62% 45%;
  animation: aura-flow-2 11.5s ease-in-out infinite;
}

/* 光团 3：中心氛围光脉冲，随播放节奏微呼吸 */
.aura-blob-core {
  width: 64%;
  height: 64%;
  border-radius: 50%;
  animation: aura-pulse-core var(--np-float-duration, 4.6s) ease-in-out infinite;
}

@keyframes aura-flow-1 {
  0% {
    transform: translate(-7%, -6%) rotate(0deg) scale(0.98);
  }
  33% {
    transform: translate(6%, -4%) rotate(120deg) scale(1.08);
  }
  66% {
    transform: translate(-3%, 7%) rotate(240deg) scale(0.94);
  }
  100% {
    transform: translate(-7%, -6%) rotate(360deg) scale(0.98);
  }
}

@keyframes aura-flow-2 {
  0% {
    transform: translate(6%, 6%) rotate(0deg) scale(0.96);
  }
  50% {
    transform: translate(-7%, -5%) rotate(-180deg) scale(1.09);
  }
  100% {
    transform: translate(6%, 6%) rotate(-360deg) scale(0.96);
  }
}

@keyframes aura-pulse-core {
  0%, 100% {
    transform: scale(0.9);
    opacity: 0.42;
  }
  50% {
    transform: scale(1.16);
    opacity: 0.72;
  }
}

/* ===== 悬浮封面卡片 ===== */
.cover-card {
  box-shadow:
    0 22px 55px -12px rgba(0, 0, 0, 0.75),
    0 0 40px 1px rgba(255, 255, 255, 0.05);
  transform-origin: center center;
  will-change: transform;
}

.cover-floating {
  animation: cover-float var(--np-float-duration, 4.6s) cubic-bezier(0.4, 0, 0.2, 1) infinite;
  animation-play-state: paused;
}

.cover-floating.is-playing {
  animation-play-state: running;
}

@keyframes cover-float {
  0%, 100% {
    transform: translateY(0px) scale(1);
  }
  50% {
    transform: translateY(-5px) scale(var(--np-float-scale, 1.02));
  }
}

/* 玻璃高光掠光层（静态通透斜高光，不晃眼） */
.cover-glass-sheen {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.12) 0%,
    rgba(255, 255, 255, 0.03) 38%,
    transparent 55%
  );
}

@media (prefers-reduced-motion: reduce) {
  .aura-blob,
  .cover-floating {
    animation: none !important;
  }
}

/* 歌词区上下渐隐（渐进滚动的视觉基础） */
.lyrics-fade {
  -webkit-mask-image: linear-gradient(to bottom, transparent 0%, #000 16%, #000 84%, transparent 100%);
  mask-image: linear-gradient(to bottom, transparent 0%, #000 16%, #000 84%, transparent 100%);
}

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
