<script setup lang="ts">
import { ref } from 'vue';
import { Heart, MoreHorizontal, ListMusic, Disc3 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import LyricsView from '../shared/LyricsView.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ Tab：正在播放 / 播放队列 ============ */
const tab = ref<'now-playing' | 'queue'>('now-playing');

/* ============ 封面图 ============ */
const coverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);

/* ============ 当前轨道信息 ============ */
const track = () => playerStore.currentTrack;
function fileInfoText(): string {
  const fi = playerStore.currentTrackFileInfo as any;
  const parts: string[] = [];
  if (fi?.release_year) parts.push(String(fi.release_year));
  parts.push('1 TRACK');
  if (track()?.duration) parts.push(track()!.duration);
  if (track()?.format) parts.push(track()!.format);
  if (fi?.bits_per_sample && fi?.sample_rate) {
    parts.push(`${fi.bits_per_sample}bit / ${(fi.sample_rate / 1000).toFixed(0)}kHz`);
  }
  return parts.join(' · ');
}

/* ============ 收藏 ============ */
function toggleFav() {
  const t = playerStore.currentTrack;
  if (t) playerStore.toggleFavorite(t.id);
}
</script>

<template>
  <div v-if="uiStore.isRightSidebarVisible" class="absolute right-0 top-0 h-full w-[360px] bg-bg-canvas flex-col z-20 flex border-l border-border-color shadow-[-4px_0_16px_rgba(0,0,0,0.12)] dark:shadow-[-4px_0_16px_rgba(0,0,0,0.3)]">

    <!-- Top Tabs -->
    <div class="relative h-[60px] flex-shrink-0" data-tauri-drag-region>
      <div class="absolute bottom-0 left-0 w-full h-px bg-border-color"></div>
      <div class="flex items-end justify-center gap-10 h-full">
        <button
          class="relative pb-3 text-[13px] transition-colors-smooth"
          :class="tab === 'now-playing' ? 'font-medium text-brand-orange' : 'text-text-muted hover:text-text-primary'"
          @click="tab = 'now-playing'"
        >
          正在播放
          <div v-if="tab === 'now-playing'" class="absolute bottom-[-1px] left-0 w-full h-[2px] bg-brand-orange z-10"></div>
        </button>
        <button
          class="relative pb-3 text-[13px] transition-colors-smooth"
          :class="tab === 'queue' ? 'font-medium text-brand-orange' : 'text-text-muted hover:text-text-primary'"
          @click="tab = 'queue'"
        >
          播放列表
          <div v-if="tab === 'queue'" class="absolute bottom-[-1px] left-0 w-full h-[2px] bg-brand-orange z-10"></div>
        </button>
      </div>
    </div>

    <!-- ===== Now Playing ===== -->
    <div v-if="tab === 'now-playing'" class="flex-1 overflow-hidden flex flex-col min-h-0">

      <!-- 无曲目占位 -->
      <div v-if="!playerStore.currentTrack" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">未在播放</span>
      </div>

      <template v-else>
        <div class="flex-1 overflow-y-auto px-6 pt-4 pb-4 flex flex-col min-h-0">
          <!-- Album Cover -->
          <div class="w-full aspect-square max-h-[40vh] bg-bg-hover rounded-[10px] mb-4 overflow-hidden flex-shrink-0 flex items-center justify-center">
            <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
            <Disc3 v-else class="w-10 h-10 text-text-disabled" />
          </div>

          <!-- Track Info -->
          <div class="mb-4 flex-shrink-0">
            <div class="flex items-center justify-between mb-0.5">
              <h2 class="text-[18px] font-bold text-text-primary truncate pr-4 leading-tight">{{ playerStore.currentTrack.title }}</h2>
              <div class="flex items-center gap-2 flex-shrink-0">
                <button
                  class="transition-colors-smooth"
                  :class="playerStore.currentTrack.isFavorite ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
                  @click="toggleFav"
                >
                  <Heart class="w-[18px] h-[18px]" :class="playerStore.currentTrack.isFavorite ? 'fill-current' : ''" />
                </button>
                <button class="text-text-muted hover:text-text-primary transition-colors-smooth">
                  <MoreHorizontal class="w-[18px] h-[18px]" />
                </button>
              </div>
            </div>

            <p class="text-[14px] text-text-primary mb-0 truncate">{{ playerStore.currentTrack.artist }}</p>
            <p class="text-[13px] text-text-muted mb-1.5 truncate">{{ playerStore.currentTrack.album }}</p>
            <p class="text-[10px] text-text-muted font-mono uppercase tracking-wider leading-relaxed">
              {{ fileInfoText() }}
            </p>
          </div>

          <!-- Lyrics -->
          <LyricsView variant="sidebar" />
        </div>
      </template>
    </div>

    <!-- ===== Queue ===== -->
    <div v-else class="flex-1 overflow-y-auto px-4 pt-4 pb-4 min-h-0">
      <h3 class="text-[10px] font-semibold text-text-muted uppercase tracking-widest mb-3 px-2">
        播放队列 · {{ playerStore.queue.length }}
      </h3>

      <div v-if="playerStore.queue.length === 0" class="flex flex-col items-center justify-center py-16 gap-3 text-text-muted">
        <ListMusic class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">队列为空</span>
      </div>

      <ul v-else class="space-y-[2px]">
        <li
          v-for="(t, i) in playerStore.queue"
          :key="t.id"
          class="flex items-center gap-2 px-2 py-[7px] rounded-[6px] cursor-pointer transition-colors-smooth group relative"
          :class="i === playerStore.currentIndex ? 'bg-list-selected playing-row' : 'hover:bg-list-hover'"
          @click="playerStore.playQueue(playerStore.queue, i)"
        >
          <span class="w-5 text-[11px] font-mono text-text-muted tabular-nums shrink-0">{{ String(i + 1).padStart(2, '0') }}</span>
          <div class="min-w-0 flex-1">
            <p class="text-[12px] truncate" :class="i === playerStore.currentIndex ? 'text-brand-orange font-medium' : 'text-text-primary'">{{ t.title }}</p>
            <p class="text-[11px] text-text-muted truncate">{{ t.artist }}</p>
          </div>
          <span class="text-[11px] font-mono text-text-muted tabular-nums shrink-0">{{ t.duration }}</span>
        </li>
      </ul>
    </div>

  </div>
</template>
