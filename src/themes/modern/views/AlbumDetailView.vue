<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue';
import { Heart, AudioLines, Play, Plus, ArrowLeft, Clock } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import type { Track } from '../../../stores/player';
import ArtworkImage from '../components/ArtworkImage.vue';

const playerStore = usePlayerStore();

const activeMenuTrackId = ref<number | null>(null);

const openPlaylistMenu = (trackId: number) => {
  activeMenuTrackId.value = activeMenuTrackId.value === trackId ? null : trackId;
};

const addToPlaylist = (playlistId: number, trackId: number) => {
  playerStore.addToPlaylist(playlistId, trackId);
  activeMenuTrackId.value = null;
};

const closeMenu = () => { activeMenuTrackId.value = null; };

const goBack = () => playerStore.goBack();

// ============ 虚拟滚动 ============
// 单行高度：py-3.5 (14*2) + 行内容 ~28 ≈ 56，含分隔线余量取 60
const ROW_HEIGHT = 60;
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

// 专辑总时长（秒）
const totalDurationSec = computed(() =>
  tracksList.value.reduce((sum, t) => sum + (t.durationSec || 0), 0)
);
const formattedTotalDuration = computed(() => {
  const sec = totalDurationSec.value;
  if (!sec) return '0 min';
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  return h > 0 ? `${h} hr ${m} min` : `${m} min`;
});

onMounted(() => window.addEventListener('click', closeMenu));
onUnmounted(() => window.removeEventListener('click', closeMenu));
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentAlbumDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 返回按钮 -->
      <button
        @click="goBack"
        class="flex items-center gap-2 text-text-muted hover:text-text-main transition-colors mb-6 w-fit text-[12px] tracking-wide"
      >
        <ArrowLeft class="w-4 h-4 stroke-[1.5]" />
        <span>返回</span>
      </button>

      <!-- 专辑信息头：大圆角封面 + 现代排版 -->
      <div class="flex flex-col md:flex-row md:items-end gap-8 mb-10 shrink-0">
        <div class="w-52 h-52 relative shrink-0">
          <div class="w-full h-full rounded-3xl overflow-hidden bg-bg-panel shadow-2xl group">
            <ArtworkImage
              :artwork-id="playerStore.currentAlbumDetails.cover_artwork_id"
              fallback-color="from-gray-400 to-gray-600"
              img-class="group-hover:scale-105 transition-transform duration-500"
            />
          </div>
        </div>
        <div class="flex flex-col pb-2 min-w-0 flex-1">
          <span class="text-[11px] font-bold tracking-[0.25em] text-text-muted uppercase mb-3">Album</span>
          <h1 class="font-bold text-5xl text-text-main mb-4 leading-tight truncate">{{ playerStore.currentAlbumDetails.title }}</h1>
          <div class="flex items-center gap-3 text-[13px] text-text-muted flex-wrap">
            <span class="font-semibold text-text-main">{{ playerStore.currentAlbumDetails.artist }}</span>
            <span class="w-1 h-1 rounded-full bg-text-muted/50"></span>
            <span>{{ playerStore.currentAlbumDetails.year }}</span>
            <span class="w-1 h-1 rounded-full bg-text-muted/50"></span>
            <span>{{ playerStore.currentAlbumDetails.tracks.length }} 首</span>
            <span class="w-1 h-1 rounded-full bg-text-muted/50"></span>
            <span class="flex items-center gap-1"><Clock class="w-3.5 h-3.5" />{{ formattedTotalDuration }}</span>
          </div>

          <!-- 操作按钮组 -->
          <div class="flex items-center gap-3 mt-6">
            <button
              v-if="playerStore.currentAlbumDetails.tracks.length > 0"
              @click="playerStore.playQueue(playerStore.currentAlbumDetails.tracks, 0)"
              class="flex items-center gap-2 bg-accent text-white px-6 py-2.5 rounded-full hover:scale-105 hover:shadow-lg active:scale-95 transition-all font-medium text-[13px] shadow-md"
            >
              <Play class="w-4 h-4 fill-current" />
              <span>播放</span>
            </button>
            <button
              class="w-10 h-10 rounded-full border border-border-color flex items-center justify-center text-text-main hover:bg-bg-active hover:border-text-muted transition-colors"
              title="更多"
            >
              <Plus class="w-4 h-4 stroke-[2]" />
            </button>
          </div>
        </div>
      </div>

      <!-- 表头 -->
      <div class="flex items-center px-4 py-2 text-[10px] font-bold tracking-[0.2em] text-text-muted uppercase border-b border-border-color shrink-0">
        <span class="w-8 text-center">#</span>
        <span class="flex-[3] pl-4">标题</span>
        <span class="w-24 text-right pr-4">时长</span>
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
              class="flex items-center px-4 text-[13px] group transition-colors duration-150 cursor-pointer rounded-lg hover:bg-bg-active/50"
              :class="playerStore.currentTrack?.id === item.data.id ? 'bg-accent/10' : ''"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <div class="w-8 text-center text-text-muted font-medium relative shrink-0">
                <template v-if="playerStore.currentTrack?.id === item.data.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-accent animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity">{{ item.index + 1 }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity text-accent fill-current" />
                </template>
              </div>
              <div class="flex-[3] pl-4 flex items-center gap-3 min-w-0">
                <span
                  class="truncate"
                  :class="playerStore.currentTrack?.id === item.data.id ? 'text-accent font-semibold' : 'text-text-main font-medium'"
                >{{ item.data.title }}</span>
              </div>
              <div class="flex items-center gap-3 shrink-0 pr-4">
                <button
                  @click.stop="openPlaylistMenu(item.data.id)"
                  class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent transition-opacity relative"
                  title="添加到歌单"
                >
                  <Plus class="w-4 h-4 stroke-[1.5]" />
                </button>
                <Heart
                  class="w-4 h-4 transition-colors stroke-[1.5] cursor-pointer"
                  :class="item.data.isFavorite ? 'text-accent fill-current' : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent'"
                  @click.stop="playerStore.toggleFavorite(item.data.id)"
                />
                <span class="w-16 text-right text-text-muted font-mono text-[12px]">{{ item.data.duration }}</span>

                <!-- 添加到歌单下拉菜单 -->
                <div v-if="activeMenuTrackId === item.data.id" class="absolute right-8 top-full mt-1 bg-bg-base border border-border-color shadow-xl z-50 py-1 min-w-[140px] rounded-lg overflow-hidden">
                  <div v-if="playerStore.playlists.length === 0" class="px-3 py-2 text-[11px] text-text-muted whitespace-nowrap">暂无自建歌单</div>
                  <button
                    v-for="pl in playerStore.playlists"
                    :key="pl.id"
                    @click.stop="addToPlaylist(pl.id, item.data.id)"
                    class="block w-full text-left px-3 py-1.5 text-[12px] text-text-main hover:bg-bg-active hover:text-accent transition-colors whitespace-nowrap truncate"
                  >
                    {{ pl.name }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-text-muted text-sm">
      未找到专辑信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: var(--border-color); }
</style>
