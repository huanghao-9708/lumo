<script setup lang="ts">
import { computed } from 'vue';
import {
  Play, Shuffle, Loader2, ListMusic, Heart, MoreHorizontal, Clock,
} from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';

const playerStore = usePlayerStore();

const detail = computed(() => playerStore.currentPlaylistDetails);

const tracks = computed<Track[]>(() => detail.value?.tracks ?? []);
const isLoading = computed(() => detail.value?.isLoadingTracks ?? false);
const isLoaded = computed(() => !!detail.value && !detail.value.isLoadingTracks);

const trackCount = computed(() => tracks.value.length);
const totalDuration = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
});
const metaText = computed(() => {
  if (!detail.value) return '';
  return `${trackCount.value} TRACKS · ${totalDuration.value}`;
});

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playAll() {
  if (tracks.value.length > 0) playerStore.playAll(tracks.value, 0);
}

function shufflePlay() {
  if (tracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * tracks.value.length);
  playerStore.playAll(tracks.value, idx);
}

function playTrack(index: number) {
  playerStore.playAll(tracks.value, index);
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- 加载中 -->
    <div v-if="isLoading && tracks.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载歌单…</span>
    </div>

    <template v-else-if="isLoaded || tracks.length > 0">
      <!-- 歌单头部 -->
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <!-- 封面（歌单无真实封面，用 ListMusic 占位） -->
          <div class="w-[180px] h-[180px] rounded-[10px] overflow-hidden flex-shrink-0 bg-bg-hover flex items-center justify-center">
            <ListMusic class="w-12 h-12 text-text-disabled" />
          </div>

          <!-- 标题 + 元数据 + 按钮 -->
          <div class="flex-1 min-w-0 pt-2">
            <h1 class="text-[28px] font-bold text-text-primary tracking-tight leading-tight mb-1">{{ detail?.name || '歌单' }}</h1>

            <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mb-5">{{ metaText }}</p>

            <!-- 操作按钮 -->
            <div class="flex items-center gap-3">
              <button
                class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
                @click="playAll"
              >
                <Play class="w-[14px] h-[14px] fill-current" />
                播放全部
              </button>
              <button
                class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-primary flex items-center gap-2 hover:bg-list-hover transition-colors-smooth"
                @click="shufflePlay"
              >
                <Shuffle class="w-[14px] h-[14px]" />
                随机播放
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 分割线 -->
      <div class="h-px bg-border-color mx-8"></div>

      <!-- 轨道列表 -->
      <div class="flex-1 overflow-y-auto px-8">
        <!-- 表头 -->
        <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
          <div class="w-10 text-center shrink-0">#</div>
          <div class="w-8 shrink-0"></div>
          <div class="flex-[2] min-w-0 pl-1">标题</div>
          <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
          <div class="w-[56px] text-right shrink-0">
            <Clock class="w-[12px] h-[12px] inline-block" />
          </div>
          <div class="w-8 shrink-0"></div>
        </div>

        <!-- 空列表 -->
        <div v-if="tracks.length === 0" class="flex flex-col items-center justify-center py-16 gap-3 text-text-muted">
          <span class="text-[12px]">该歌单暂无曲目</span>
        </div>

        <!-- 轨道行 -->
        <div
          v-for="(track, index) in tracks"
          :key="track.id"
          class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer"
          style="height: 40px;"
          :class="{ 'playing-row bg-list-selected': isPlayingTrack(track.id) }"
          @dblclick="playTrack(index)"
        >
          <div class="w-10 text-center shrink-0 text-[12px] font-mono">
            <span v-if="isPlayingTrack(track.id)" class="text-brand-orange inline-flex items-center justify-center">
              <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
              <Play v-else class="w-[12px] h-[12px] fill-current" />
            </span>
            <template v-else>
              <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
              <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
            </template>
          </div>

          <div class="w-8 shrink-0 flex items-center justify-center">
            <Heart
              v-if="track.isFavorite"
              class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
            <Heart
              v-else
              class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
          </div>

          <div class="flex-[2] min-w-0 pl-1">
            <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
              {{ track.title }}
            </span>
          </div>

          <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ track.artist }}</div>

          <div class="w-[56px] text-right shrink-0 text-[12px] font-mono text-text-muted tabular-nums">{{ track.duration }}</div>

          <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <MoreHorizontal class="w-4 h-4 text-text-muted" />
          </div>
        </div>

        <!-- 底部统计 -->
      </div>
    </template>
  </div>

  <!-- 底部统计（固定在底部） -->
  <div v-if="tracks.length > 0" class="flex-shrink-0 flex items-center justify-between px-8 py-4 border-t border-border-color text-[11px] text-text-muted font-mono bg-bg-content">
    <span>{{ trackCount }} 首曲目 · {{ totalDuration }}</span>
    <span>双击播放</span>
  </div>
</template>
