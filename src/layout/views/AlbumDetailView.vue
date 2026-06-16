<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue';
import { Heart, AudioLines, Play, Plus } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import type { Track } from '../../stores/player';
import ArtworkImage from '../components/ArtworkImage.vue';

const playerStore = usePlayerStore();

const activeMenuTrackId = ref<number | null>(null);

const openPlaylistMenu = (trackId: number) => {
  if (activeMenuTrackId.value === trackId) {
    activeMenuTrackId.value = null;
  } else {
    activeMenuTrackId.value = trackId;
  }
};

const addToPlaylist = (playlistId: number, trackId: number) => {
  playerStore.addToPlaylist(playlistId, trackId);
  activeMenuTrackId.value = null;
};

const closeMenu = () => {
  activeMenuTrackId.value = null;
};

// ============ 虚拟滚动 ============
// 单行高度：py-4 (16px*2) + 行内容约 32px ≈ 64px
const ROW_HEIGHT = 64;
const BUFFER_ROWS = 6;
const scrollContainer = ref<HTMLElement | null>(null);
const tick = ref(0);

const tracksList = computed<readonly Track[]>(() => playerStore.currentAlbumDetails?.tracks ?? []);
const totalHeight = computed(() => tracksList.value.length * ROW_HEIGHT);

interface VisibleItem { index: number; data: Track }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value;
  const el = scrollContainer.value;
  const all = tracksList.value;
  if (!el || all.length === 0) return [];

  const scrollTop = el.scrollTop;
  const viewH = el.clientHeight;
  const startRow = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS);
  const visibleRows = Math.ceil(viewH / ROW_HEIGHT) + BUFFER_ROWS * 2;
  const endRow = startRow + visibleRows;
  const endIdx = Math.min(all.length, endRow);

  const out: VisibleItem[] = [];
  for (let i = startRow; i < endIdx; i++) {
    out.push({ index: i, data: all[i] });
  }
  return out;
});

const offsetY = computed(() => {
  void tick.value;
  const el = scrollContainer.value;
  if (!el) return 0;
  return Math.max(0, Math.floor(el.scrollTop / ROW_HEIGHT) - BUFFER_ROWS) * ROW_HEIGHT;
});

let ticking = false;
const handleScroll = () => {
  tick.value++;
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
};

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentAlbumDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 专辑信息头 -->
      <div class="flex flex-col md:flex-row md:items-end gap-12 mb-16 shrink-0 relative">
        <div class="w-56 h-56 relative bg-[#f5f4f0] shadow-sm shrink-0 group">
          <ArtworkImage
            :artwork-id="playerStore.currentAlbumDetails.cover_artwork_id"
            fallback-color="from-gray-300 to-gray-400"
          />

          <!-- 悬浮播放按钮 -->
          <div class="absolute right-4 bottom-4 opacity-0 group-hover:opacity-100 transition-opacity duration-300">
            <button
              @click="playerStore.playQueue(playerStore.currentAlbumDetails.tracks, 0)"
              class="w-12 h-12 bg-black text-white rounded-full flex items-center justify-center hover:scale-105 transition-transform shadow-md"
            >
              <Play class="w-5 h-5 ml-1 fill-current" />
            </button>
          </div>
        </div>
        <div class="flex flex-col pb-2">
          <h2 class="text-[10px] font-bold tracking-[0.3em] text-text-muted mb-6 uppercase">Album</h2>
          <h1 class="font-serif italic text-6xl tracking-wide text-accent mb-6 leading-tight">{{ playerStore.currentAlbumDetails.title }}</h1>
          <p class="text-[12px] font-medium text-[#555] tracking-[0.1em] uppercase">
            <span class="text-accent font-bold tracking-widest">{{ playerStore.currentAlbumDetails.artist }}</span>
            <span class="mx-4 text-[#dcdad1]">/</span>
            {{ playerStore.currentAlbumDetails.year }}
            <span class="mx-4 text-[#dcdad1]">/</span>
            {{ playerStore.currentAlbumDetails.tracks.length }} Tracks
          </p>
          <button
            v-if="playerStore.currentAlbumDetails.tracks.length > 0"
            @click="playerStore.playQueue(playerStore.currentAlbumDetails.tracks, 0)"
            class="flex items-center gap-2 bg-black text-white px-5 py-2.5 hover:bg-black/80 transition-all group rounded-sm shadow-md mt-6 w-fit"
          >
            <Play class="w-3.5 h-3.5 fill-current" />
            <span class="text-[10px] font-bold tracking-[0.2em] uppercase">播放全部</span>
          </button>
        </div>
      </div>

      <!-- 虚拟滚动歌曲列表 -->
      <div
        ref="scrollContainer"
        class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-10"
        @scroll="handleScroll"
      >
        <div :style="{ height: totalHeight + 'px', position: 'relative' }">
          <div class="absolute top-0 left-0 right-0 will-change-transform" :style="{ transform: `translateY(${offsetY}px)` }">
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @dblclick="playerStore.playQueue(playerStore.currentAlbumDetails.tracks, item.index)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/60 group transition-colors duration-200 cursor-pointer hover:bg-[#faf9f5]"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <div class="w-16 text-center text-text-muted font-medium relative">
                <template v-if="playerStore.currentTrack?.id === item.data.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-accent animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity duration-200">{{ String(item.index + 1).padStart(2, '0') }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-accent fill-current" />
                </template>
              </div>
              <div class="flex-[3] pl-2 flex items-center gap-4">
                <Heart
                  class="w-3.5 h-3.5 transition-colors stroke-[1.5]"
                  :class="[
                    item.data.isFavorite ? 'text-accent fill-current' : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent '
                  ]"
                  @click.stop="playerStore.toggleFavorite(item.data.id)"
                />
                <div class="relative flex items-center">
                  <button @click.stop="openPlaylistMenu(item.data.id)" class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent  transition-opacity" title="添加到歌单">
                    <Plus class="w-3.5 h-3.5 stroke-[1.5]" />
                  </button>
                  <div v-if="activeMenuTrackId === item.data.id" class="absolute left-6 top-0 bg-bg-base border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                    <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-text-muted whitespace-nowrap">暂无自建歌单</div>
                    <button
                      v-for="pl in playerStore.playlists"
                      :key="pl.id"
                      @click.stop="addToPlaylist(pl.id, item.data.id)"
                      class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-accent  hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
                    >
                      {{ pl.name }}
                    </button>
                  </div>
                </div>
                <span
                  class="truncate"
                  :class="playerStore.currentTrack?.id === item.data.id ? 'font-serif italic font-semibold text-[16px] text-accent' : 'text-text-main  font-medium'"
                >{{ item.data.title }}</span>
              </div>
              <div class="w-20 text-right pr-4 text-text-muted ">{{ item.data.duration }}</div>
              <div class="w-24 pl-4 text-left text-[11px] text-[#aaa] tracking-wider">{{ item.data.format }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-text-muted ">
      未找到专辑信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
