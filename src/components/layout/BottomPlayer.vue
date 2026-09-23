<script setup lang="ts">
import { ref, computed } from 'vue';
import {
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1, ChevronUp, ChevronDown, Disc3, Volume, Volume1, Volume2,
  Heart, ListPlus,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { libraryAddToPlaylist } from '../../api/library';
import PlaybackRateButton from '../shared/PlaybackRateButton.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();
const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);

/* ============ 进度条 ============ */
/** 拖拽中的本地预览位置：拖动期间不动后端，松手才 seek 一次（见下方注释） */
const dragMs = ref<number | null>(null);
const displayProgressMs = computed(() => dragMs.value ?? playerStore.progressMs);

const progressPercent = computed(() => {
  const total = playerStore.durationMs;
  if (!total) return 0;
  return Math.min(100, Math.max(0, (displayProgressMs.value / total) * 100));
});
const currentTimeText = computed(() => formatMs(displayProgressMs.value));
const totalTimeText = computed(() => formatMs(playerStore.durationMs));

/** 悬停预览时间与位置 */
const hoverMs = ref<number | null>(null);
const hoverPercent = ref<number>(0);
const isHoveringProgress = ref(false);
const hoverTimeText = computed(() => (hoverMs.value != null ? formatMs(hoverMs.value) : ''));

function formatMs(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

// 进度条拖拽
const isDraggingProgress = ref(false);
const progressRef = ref<HTMLElement | null>(null);

/** 鼠标横坐标 → 目标毫秒（超出范围时夹到两端） */
function msFromEvent(clientX: number): number | null {
  const el = progressRef.value;
  if (!el || playerStore.durationMs <= 0) return null;
  const rect = el.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  return Math.floor(pct * playerStore.durationMs);
}

/**
 * 拖拽过程只在本地预览，松手才 seek。
 * mousemove 是逐像素触发的，每次都发 IPC 会把通道打满（本项目最大的坑），
 * 而后端 seek 还要等音频线程确认，卡顿会被放大得很明显。
 */
function onProgressDown(e: MouseEvent) {
  const ms = msFromEvent(e.clientX);
  if (ms == null) return;
  isDraggingProgress.value = true;
  dragMs.value = ms;
}
function onProgressMove(e: MouseEvent) {
  if (!isDraggingProgress.value) return;
  const ms = msFromEvent(e.clientX);
  if (ms != null) {
    dragMs.value = ms;
    if (playerStore.durationMs > 0) {
      hoverPercent.value = (ms / playerStore.durationMs) * 100;
    }
  }
}
function onProgressUp() {
  if (!isDraggingProgress.value) return;
  isDraggingProgress.value = false;
  const ms = dragMs.value;
  dragMs.value = null;
  if (ms != null) playerStore.seek(ms);
}

function onProgressHover(e: MouseEvent) {
  const el = progressRef.value;
  if (!el || playerStore.durationMs <= 0) return;
  const rect = el.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  hoverPercent.value = pct * 100;
  hoverMs.value = Math.floor(pct * playerStore.durationMs);
  isHoveringProgress.value = true;
}

function onProgressLeave() {
  isHoveringProgress.value = false;
  hoverMs.value = null;
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

/* ============ 确定性波形（基于 track id 的固定种子，纯 CSS 动画驱动） ============ */
const BARS = 32;
function seedRand(seed: number) {
  // 简单确定性 PRNG：同一 seed 永远产生同一序列
  let s = seed || 1;
  return () => {
    s = (s * 1664525 + 1013904223) % 4294967296;
    return s / 4294967296;
  };
}

const waveformBars = computed(() => {
  const seed = playerStore.currentTrack?.id ?? 42;
  const rand = seedRand(seed);
  const bars: { height: number; delay: number; duration: number }[] = [];
  
  // 中间高、两边低的包络，更像真实音频波形
  for (let i = 0; i < BARS; i++) {
    const center = 1 - Math.abs(i - BARS / 2) / (BARS / 2);
    const staticNoise = 0.3 + rand() * 0.7;
    const base = Math.max(0.15, Math.min(1, center * 0.6 + staticNoise * 0.5));
    // 错落有致的律动周期与延时（纯 CSS 动画使用，GPU 合成，不占 JS 线程）
    const delay = ((i * 7) % 11) * 0.08;
    const duration = 0.6 + ((i * 3) % 5) * 0.12;
    bars.push({
      height: Math.max(15, Math.round(base * 100)),
      delay,
      duration,
    });
  }
  return bars;
});
const playedBarCount = computed(() => Math.round((progressPercent.value / 100) * BARS));

/* ============ 音量旋钮（物理刻度环） ============
   SVG 圆盘 + 一圈刻度 + 中心指针，指针角度 = -135°(0) ~ +135°(100)。
   拖动 / 滚轮调节。 */
const KNOB_TICKS = 11; // 0~10 共 11 个刻度
const knobRef = ref<HTMLElement | null>(null);
const isDraggingKnob = ref(false);

// 指针角度（度）：0 音量 -> -135°，100 音量 -> +135°
const knobAngle = computed(() => -135 + (playerStore.volume / 100) * 270);

const volumeIcon = computed(() => {
  if (playerStore.volume === 0) return Volume;
  if (playerStore.volume < 40) return Volume1;
  return Volume2;
});

// 拖动旋钮：把鼠标相对旋钮中心的角度换算成音量百分比。
// 旋钮指针正上方=最大值，缺口在正下方=0，有效角度范围 [-135°, 135°]。
function setVolumeFromPointer(clientX: number, clientY: number) {
  const el = knobRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  // 计算鼠标相对旋钮中心的角度。屏幕坐标 y 向下，转换为数学角度（逆时针，0°在右）
  let deg = Math.atan2(clientY - cy, clientX - cx) * 180 / Math.PI;
  // 我们让 0° 指向正下方（指针向上=最大）。把坐标系旋成「顶部=最大」：
  // 旋钮指针顶部=100%，底部缺口=0%。即有效范围是 [-135°, 135°]，以正上方为 0°。
  // 转换：指针角度从「正上」计起，顺时针为正。
  // 正上方在 atan2 里是 -90°（因为 y 向下）。换算：
  deg = deg + 90; // 让正上方=0
  // 归一化到 [-180,180]
  if (deg > 180) deg -= 360;
  if (deg < -180) deg += 360;
  // 限制到 [-135, 135]
  deg = Math.max(-135, Math.min(135, deg));
  const pct = (deg + 135) / 270;
  playerStore.setVolume(Math.round(pct * 100));
}

function onKnobDown(e: MouseEvent) {
  isDraggingKnob.value = true;
  setVolumeFromPointer(e.clientX, e.clientY);
}
function onKnobMove(e: MouseEvent) {
  if (!isDraggingKnob.value) return;
  setVolumeFromPointer(e.clientX, e.clientY);
}
function onKnobUp() { isDraggingKnob.value = false; }

function onKnobWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY < 0 ? 5 : -5;
  playerStore.setVolume(Math.max(0, Math.min(100, playerStore.volume + delta)));
}

// 全局监听拖拽移动/释放（在 window 上，避免移出元素丢失）
if (typeof window !== 'undefined') {
  window.addEventListener('mousemove', (e) => {
    if (isDraggingKnob.value) onKnobMove(e);
    if (isDraggingProgress.value) onProgressMove(e);
  });
  window.addEventListener('mouseup', () => {
    if (isDraggingKnob.value) onKnobUp();
    if (isDraggingProgress.value) onProgressUp();
  });
}

const showPlaylistPicker = ref(false);
const trackIsFav = computed(() => playerStore.currentTrack?.isFavorite ?? false);

function toggleFav() {
  const t = playerStore.currentTrack;
  if (t) playerStore.toggleFavorite(t.id);
}

async function addCurrentToPlaylist(playlistId: number) {
  const t = playerStore.currentTrack;
  if (!t) return;
  try {
    await libraryAddToPlaylist(playlistId, t.id);
    showPlaylistPicker.value = false;
  } catch (e) {
    console.error('添加到歌单失败:', e);
  }
}
</script>

<template>
  <div class="relative h-[110px] w-full bg-bg-canvas/85 backdrop-blur-xl border-t border-border-color/60 flex items-center justify-between px-6 flex-shrink-0 select-none transition-colors-smooth">

    <!-- Left: Track Info & Actions (左侧自适应，设置最大宽度，不越界挤压中控) -->
    <div class="flex items-center min-w-0 max-w-[32vw] flex-shrink-0 z-10">
      <!-- 封面（悬浮有微质感与展开沉浸提示） -->
      <div
        class="group relative w-[54px] h-[54px] bg-bg-hover rounded-[8px] overflow-hidden flex-shrink-0 mr-3.5 flex items-center justify-center cursor-pointer shadow-sm ring-1 ring-black/5 dark:ring-white/10 transition-transform duration-200 hover:scale-[1.02]"
        title="进入沉浸式播放"
        @click="uiStore.openImmersiveView()"
      >
        <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover transition-opacity duration-200 group-hover:opacity-85" alt="cover" />
        <Disc3 v-else class="w-6 h-6 text-text-disabled" />
        <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-white">
          <ChevronUp class="w-5 h-5 drop-shadow-sm" />
        </div>
      </div>

      <div class="flex flex-col justify-center min-w-0 pr-2.5" v-if="playerStore.currentTrack">
        <span class="text-[13px] font-semibold text-text-primary truncate leading-tight">{{ playerStore.currentTrack.title }}</span>
        <span class="text-[11px] text-text-muted truncate mt-0.5">{{ playerStore.currentTrack.artist }} · {{ playerStore.currentTrack.album }}</span>
        
        <div class="flex items-center gap-2 mt-1">
          <span class="text-[9px] text-text-muted font-mono uppercase tracking-wider px-1 py-0.5 bg-bg-hover rounded">{{ playerStore.currentTrack.format }}</span>
          <!-- 确定性微波形律动（纯 CSS GPU 驱动，零 JS 开销） -->
          <div class="flex items-end h-3 gap-[1px]" :class="{ 'is-playing': playerStore.isPlaying }">
            <div
              v-for="(bar, i) in waveformBars"
              :key="i"
              class="w-[2px] rounded-t-sm transition-colors-smooth waveform-bar"
              :class="i < playedBarCount ? 'bg-brand-orange' : 'bg-text-muted/25'"
              :style="{
                height: `${bar.height}%`,
                animationDelay: `${bar.delay}s`,
                animationDuration: `${bar.duration}s`,
              }"
            ></div>
          </div>
        </div>
      </div>
      <div v-else class="flex flex-col justify-center min-w-0">
        <span class="text-[13px] text-text-muted font-medium">未在播放</span>
        <span class="text-[11px] text-text-disabled">选择一首歌曲开始</span>
      </div>

      <!-- Actions: Favorite + Add to Playlist + PlaybackRate -->
      <div v-if="playerStore.currentTrack" class="flex items-center gap-2 flex-shrink-0 ml-1">
        <PlaybackRateButton />
        <button
          title="收藏"
          @click="toggleFav"
          class="w-7 h-7 flex items-center justify-center rounded-[6px] hover:bg-bg-hover transition-colors-smooth text-text-muted"
        >
          <Heart v-if="trackIsFav" class="w-[16px] h-[16px] text-brand-orange fill-current" />
          <Heart v-else class="w-[16px] h-[16px] hover:text-text-primary" />
        </button>
        <div class="relative">
          <button
            title="添加到歌单"
            @click="showPlaylistPicker = !showPlaylistPicker"
            class="w-7 h-7 flex items-center justify-center rounded-[6px] hover:bg-bg-hover transition-colors-smooth text-text-muted hover:text-text-primary"
          >
            <ListPlus class="w-[16px] h-[16px]" />
          </button>
          <div
            v-if="showPlaylistPicker"
            class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-50 bg-bg-canvas border border-border-solid rounded-[8px] shadow-lg py-1 min-w-[160px]"
            @click.outside="showPlaylistPicker = false"
          >
            <button
              v-for="pl in playerStore.playlists"
              :key="pl.id"
              @click="addCurrentToPlaylist(pl.id)"
              class="block w-full text-left px-3 py-1.5 text-[12px] text-text-primary hover:bg-list-hover transition-colors-smooth whitespace-nowrap"
            >
              {{ pl.name }}
            </button>
            <div v-if="playerStore.playlists.length === 0" class="px-3 py-2 text-[11px] text-text-muted text-center">暂无歌单</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Center: Playback Controls & Progress (屏幕物理绝对居中，无视左右宽度差) -->
    <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-[540px] max-w-[min(540px,42vw)] flex flex-col items-center justify-center z-20 pointer-events-none">
      
      <!-- 核心控制按钮行：左右对称几何分布 -->
      <div class="flex items-center justify-center gap-7 mb-2 pointer-events-auto">
        <!-- 播放模式切换 -->
        <button
          class="w-8 h-8 flex items-center justify-center rounded-full hover:bg-bg-hover transition-all"
          :class="modeActive ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
          :title="`播放模式: ${playerStore.playMode}`"
          @click="cycleMode"
        >
          <component :is="modeIcon" class="w-[16px] h-[16px]" />
        </button>

        <!-- 上一首 -->
        <button
          class="w-8 h-8 flex items-center justify-center rounded-full text-text-primary hover:text-brand-orange hover:bg-bg-hover active:scale-95 transition-all"
          @click="playerStore.prevTrack()"
          title="上一首"
        >
          <SkipBack class="w-[18px] h-[18px] fill-current" />
        </button>

        <!-- 播放/暂停 大圆钮（微阴影与高光交互） -->
        <button
          class="w-[46px] h-[46px] rounded-full bg-text-primary text-bg-canvas flex items-center justify-center shadow-sm hover:shadow-md hover:scale-[1.04] active:scale-[0.96] transition-all disabled:opacity-50 disabled:hover:scale-100"
          :disabled="!playerStore.currentTrack"
          @click="playerStore.togglePlay()"
          :title="playerStore.isPlaying ? '暂停' : '播放'"
        >
          <Pause v-if="playerStore.isPlaying" class="w-[20px] h-[20px] fill-current" />
          <Play v-else class="w-[20px] h-[20px] fill-current ml-0.5" />
        </button>

        <!-- 下一首 -->
        <button
          class="w-8 h-8 flex items-center justify-center rounded-full text-text-primary hover:text-brand-orange hover:bg-bg-hover active:scale-95 transition-all"
          @click="playerStore.nextTrack()"
          title="下一首"
        >
          <SkipForward class="w-[18px] h-[18px] fill-current" />
        </button>

        <!-- 展开沉浸式视图（ChevronUp 向上展开，与底栏弹出方向一致） -->
        <button
          class="w-8 h-8 flex items-center justify-center rounded-full text-text-muted hover:text-text-primary hover:bg-bg-hover active:scale-95 transition-all"
          title="展开沉浸式播放"
          @click="uiStore.openImmersiveView()"
        >
          <ChevronUp class="w-[17px] h-[17px]" />
        </button>
      </div>

      <!-- 进度条（带悬浮气泡 + 平滑加粗 + 光晕滑块） -->
      <div class="w-full flex items-center gap-3 pointer-events-auto">
        <span class="text-[10px] font-mono text-text-muted w-10 text-right tabular-nums select-none">{{ currentTimeText }}</span>
        
        <div
          ref="progressRef"
          class="group relative flex-1 py-2 cursor-pointer"
          @mousedown="onProgressDown"
          @mousemove="onProgressHover"
          @mouseleave="onProgressLeave"
        >
          <!-- 悬浮时间提示气泡 (Time Tooltip) -->
          <div
            v-if="hoverMs !== null || isDraggingProgress"
            class="absolute -top-6 -translate-x-1/2 px-1.5 py-0.5 rounded text-[10px] font-mono bg-bg-content text-text-primary border border-border-color shadow-md pointer-events-none transition-opacity"
            :style="{ left: `${isDraggingProgress ? progressPercent : hoverPercent}%` }"
          >
            {{ isDraggingProgress ? currentTimeText : hoverTimeText }}
          </div>

          <!-- 轨道底槽与激活槽：hover 时从 3px 平滑扩展到 5px -->
          <div class="w-full h-[3px] group-hover:h-[5px] bg-border-solid rounded-full relative transition-[height] duration-150 overflow-visible flex items-center">
            <div
              class="h-full bg-brand-orange rounded-full relative"
              :style="{ width: progressPercent + '%' }"
            ></div>
            <!-- 滑块圆点：悬浮/拖拽时放大带柔和光晕 -->
            <div
              class="absolute top-1/2 -translate-y-1/2 w-[10px] h-[10px] group-hover:w-[12px] group-hover:h-[12px] bg-brand-orange rounded-full opacity-0 group-hover:opacity-100 shadow-[0_0_8px_rgba(226,138,35,0.6)] transition-all pointer-events-none"
              :class="{ 'opacity-100 w-[12px] h-[12px]': isDraggingProgress }"
              :style="{ left: progressPercent + '%', marginLeft: '-5px' }"
            ></div>
          </div>
        </div>

        <span class="text-[10px] font-mono text-text-muted w-10 text-left tabular-nums select-none">{{ totalTimeText }}</span>
      </div>

    </div>

    <!-- Right: Volume Knob & Output (右侧靠右对齐) -->
    <div class="flex items-center justify-end min-w-0 max-w-[32vw] flex-shrink-0 gap-8 z-10">
      <!-- 物理音量旋钮 -->
      <div class="flex flex-col items-center">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5 flex items-center gap-1 select-none">
          <component :is="volumeIcon" class="w-[11px] h-[11px]" />
          Volume
        </span>

        <div
          ref="knobRef"
          class="relative w-[50px] h-[50px] cursor-grab active:cursor-grabbing select-none"
          :class="isDraggingKnob ? 'cursor-grabbing' : ''"
          @mousedown="onKnobDown"
          @wheel="onKnobWheel"
          title="拖动 / 滚轮调节音量"
        >
          <!-- 刻度环 -->
          <svg class="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 52 52">
            <g
              v-for="i in KNOB_TICKS"
              :key="i"
              :transform="`rotate(${-135 + ((i - 1) / (KNOB_TICKS - 1)) * 270} 26 26)`"
            >
              <line
                x1="26" y1="3.5"
                x2="26" :y2="i === 1 || i === KNOB_TICKS ? 7 : 6"
                :stroke="(((i - 1) / (KNOB_TICKS - 1)) * 100) <= playerStore.volume ? '#E28A23' : 'rgba(139,139,139,0.45)'"
                :stroke-width="i === 1 || i === KNOB_TICKS ? 1.5 : 1"
                stroke-linecap="round"
              />
            </g>
          </svg>

          <!-- 旋钮主体 -->
          <div class="absolute inset-[10px] rounded-full bg-bg-content border border-border-solid shadow-[inset_0_1px_2px_rgba(0,0,0,0.06)]"></div>

          <!-- 指针 -->
          <div
            class="absolute inset-0 transition-transform duration-150 ease-out"
            :style="{ transform: `rotate(${knobAngle}deg)` }"
            :class="isDraggingKnob ? 'transition-none' : ''"
          >
            <div class="absolute left-1/2 top-[5px] -translate-x-1/2 w-[2px] h-[7px] bg-brand-orange rounded-full"></div>
          </div>
        </div>

        <div class="flex justify-between w-14 mt-1 text-[9px] text-text-muted font-mono tabular-nums select-none">
          <span>0</span>
          <span class="text-text-primary font-bold">{{ playerStore.volume }}</span>
          <span>100</span>
        </div>
      </div>

      <!-- Output Selector -->
      <div class="flex flex-col items-start select-none">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5">Output</span>
        <button class="flex items-center gap-1 text-[12px] font-medium text-text-primary hover:text-brand-orange transition-colors-smooth">
          Built-in Output
          <ChevronDown class="w-3 h-3 text-text-muted" />
        </button>
      </div>
    </div>

  </div>
</template>

<style scoped>
.waveform-bar {
  transform-origin: bottom;
  will-change: transform;
}

.is-playing .waveform-bar {
  animation: waveform-bounce ease-in-out infinite alternate;
}

@keyframes waveform-bounce {
  0% {
    transform: scaleY(0.4);
  }
  100% {
    transform: scaleY(1.2);
  }
}
</style>
