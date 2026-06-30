<script setup lang="ts">
import { ref, computed } from 'vue';
import {
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1, ChevronDown, Disc3, Volume, Volume1, Volume2,
  Heart, ListPlus,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { libraryAddToPlaylist } from '../../api/library';

const playerStore = usePlayerStore();
const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);

/* ============ 进度条 ============ */
const progressPercent = computed(() => {
  const total = playerStore.durationMs;
  if (!total) return 0;
  return Math.min(100, Math.max(0, (playerStore.progressMs / total) * 100));
});
const currentTimeText = computed(() => formatMs(playerStore.progressMs));
const totalTimeText = computed(() => formatMs(playerStore.durationMs));

function formatMs(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  return `${String(m).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

// 进度条拖拽
const isDraggingProgress = ref(false);
const progressRef = ref<HTMLElement | null>(null);

function seekFromEvent(clientX: number) {
  const el = progressRef.value;
  if (!el || playerStore.durationMs <= 0) return;
  const rect = el.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  playerStore.seek(Math.floor(pct * playerStore.durationMs));
}
function onProgressDown(e: MouseEvent) {
  isDraggingProgress.value = true;
  seekFromEvent(e.clientX);
}
function onProgressMove(e: MouseEvent) {
  if (!isDraggingProgress.value) return;
  seekFromEvent(e.clientX);
}
function onProgressUp() { isDraggingProgress.value = false; }

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

/* ============ 确定性波形（基于 track id 的固定种子） ============ */
const BARS = 48;
function seedRand(seed: number) {
  // 简单确定性 PRNG：同一 seed 永远产生同一序列
  let s = seed || 1;
  return () => {
    s = (s * 1664525 + 1013904223) % 4294967296;
    return s / 4294967296;
  };
}
const waveform = computed(() => {
  const seed = playerStore.currentTrack?.id ?? 42;
  const rand = seedRand(seed);
  const bars: number[] = [];
  // 中间高、两边低的包络，更像真实音频波形
  for (let i = 0; i < BARS; i++) {
    const center = 1 - Math.abs(i - BARS / 2) / (BARS / 2);
    const noise = 0.3 + rand() * 0.7;
    bars.push(Math.max(0.15, Math.min(1, center * 0.6 + noise * 0.5)));
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
  <div class="h-[110px] w-full bg-bg-canvas flex items-center px-6 flex-shrink-0 select-none">

    <!-- Left: Track Info & Waveform -->
    <div class="flex items-center w-[280px] flex-shrink-0">
      <div class="w-[56px] h-[56px] bg-bg-hover rounded-[6px] overflow-hidden flex-shrink-0 mr-3 flex items-center justify-center">
        <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
        <Disc3 v-else class="w-5 h-5 text-text-disabled" />
      </div>

      <div class="flex flex-col justify-center min-w-0" v-if="playerStore.currentTrack">
        <span class="text-[13px] font-semibold text-text-primary truncate leading-tight">{{ playerStore.currentTrack.title }}</span>
        <span class="text-[11px] text-text-muted truncate mt-0.5">{{ playerStore.currentTrack.artist }} · {{ playerStore.currentTrack.album }}</span>
        <span class="text-[9px] text-text-muted font-mono mt-0.5 uppercase tracking-wider">{{ playerStore.currentTrack.format }}</span>

        <!-- 确定性波形 -->
        <div class="flex items-end h-4 gap-[1px] mt-1.5">
          <div
            v-for="(h, i) in waveform"
            :key="i"
            class="w-[2px] rounded-t-sm transition-colors-smooth"
            :class="i < playedBarCount ? 'bg-brand-orange' : 'bg-text-muted/30'"
            :style="{ height: `${h * 100}%` }"
          ></div>
        </div>
      </div>
      <div v-else class="flex flex-col justify-center min-w-0">
        <span class="text-[13px] text-text-muted">未在播放</span>
        <span class="text-[11px] text-text-disabled">选择一首歌曲开始</span>
      </div>
    </div>

    <!-- Actions: Favorite + Add to Playlist -->
    <div v-if="playerStore.currentTrack" class="flex items-center gap-3 flex-shrink-0 mr-2">
      <button title="收藏" @click="toggleFav">
        <Heart v-if="trackIsFav" class="w-[16px] h-[16px] text-brand-orange fill-current cursor-pointer transition-colors-smooth" />
        <Heart v-else class="w-[16px] h-[16px] text-text-muted hover:text-text-primary cursor-pointer transition-colors-smooth" />
      </button>
      <div class="relative">
        <button title="添加到歌单" @click="showPlaylistPicker = !showPlaylistPicker">
          <ListPlus class="w-[16px] h-[16px] text-text-muted hover:text-text-primary cursor-pointer transition-colors-smooth" />
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

    <!-- Center: Playback Controls -->
    <div class="flex-1 flex flex-col items-center justify-center px-8">

      <div class="flex items-center gap-7 mb-2">
        <button
          class="transition-colors-smooth"
          :class="modeActive ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
          :title="`播放模式: ${playerStore.playMode}`"
          @click="cycleMode"
        >
          <component :is="modeIcon" class="w-[16px] h-[16px]" />
        </button>
        <button class="text-text-primary hover:text-brand-orange transition-colors-smooth" @click="playerStore.prevTrack()" title="上一首">
          <SkipBack class="w-[18px] h-[18px] fill-current" />
        </button>
        <button
          class="w-[48px] h-[48px] rounded-full bg-text-primary text-bg-canvas flex items-center justify-center hover:opacity-90 transition-opacity"
          :disabled="!playerStore.currentTrack"
          @click="playerStore.togglePlay()"
        >
          <Pause v-if="playerStore.isPlaying" class="w-[20px] h-[20px] fill-current" />
          <Play v-else class="w-[20px] h-[20px] fill-current ml-0.5" />
        </button>
        <button class="text-text-primary hover:text-brand-orange transition-colors-smooth" @click="playerStore.nextTrack()" title="下一首">
          <SkipForward class="w-[18px] h-[18px] fill-current" />
        </button>
        <button
          class="text-text-muted hover:text-text-primary transition-colors-smooth"
          title="更多"
        >
          <ChevronDown class="w-[16px] h-[16px]" />
        </button>
      </div>

      <!-- 进度条（可拖拽） -->
      <div class="w-full flex items-center gap-3 max-w-2xl">
        <span class="text-[10px] font-mono text-text-muted w-9 text-right tabular-nums">{{ currentTimeText }}</span>
        <div
          ref="progressRef"
          class="flex-1 h-[3px] bg-border-solid rounded-full relative group cursor-pointer"
          @mousedown="onProgressDown"
        >
          <div class="absolute left-0 top-0 h-full bg-brand-orange rounded-full" :style="{ width: progressPercent + '%' }"></div>
          <div
            class="absolute top-1/2 -translate-y-1/2 w-[10px] h-[10px] bg-brand-orange rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
            :style="{ left: progressPercent + '%', marginLeft: '-5px' }"
          ></div>
        </div>
        <span class="text-[10px] font-mono text-text-muted w-9 text-left tabular-nums">{{ totalTimeText }}</span>
      </div>

    </div>

    <!-- Right: Volume Knob & Output -->
    <div class="flex items-center gap-8 flex-shrink-0">

      <!-- 物理音量旋钮 -->
      <div class="flex flex-col items-center">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5 flex items-center gap-1">
          <component :is="volumeIcon" class="w-[11px] h-[11px]" />
          Volume
        </span>

        <div
          ref="knobRef"
          class="relative w-[52px] h-[52px] cursor-grab active:cursor-grabbing select-none"
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

          <div class="flex justify-between w-14 mt-1 text-[9px] text-text-muted font-mono tabular-nums">
          <span>0</span>
          <span class="text-text-primary font-bold">{{ playerStore.volume }}</span>
          <span>100</span>
        </div>
      </div>

      <!-- Output Selector -->
      <div class="flex flex-col items-start">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5">Output</span>
        <button class="flex items-center gap-1 text-[12px] font-medium text-text-primary hover:text-brand-orange transition-colors-smooth">
          Built-in Output
          <ChevronDown class="w-3 h-3 text-text-muted" />
        </button>
      </div>

    </div>

  </div>
</template>
