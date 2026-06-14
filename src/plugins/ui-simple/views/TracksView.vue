<script setup lang="ts">
import { onMounted, ref, onUnmounted, watch, computed } from 'vue';
import { Heart, AudioLines, Plus, Play } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import type { Track } from '../../../stores/player';

const playerStore = usePlayerStore();
const activeMenuTrackId = ref<number | null>(null);

const loadData = () => {
  const tab = playerStore.activeLibraryTab;
  if (tab === '全部歌曲') {
    playerStore.fetchTracks(true);
  } else if (tab === '收藏歌曲') {
    playerStore.fetchFavoriteTracks();
  } else if (tab === '最近播放') {
    playerStore.fetchRecentlyPlayed();
  } else {
    const pl = playerStore.playlists.find((p: any) => p.name === tab);
    if (pl) {
      playerStore.fetchPlaylistTracks(pl.id);
    }
  }
};

// ============ 虚拟滚动 ============
// 单行高度：py-4 (16px*2) + 行内容约 32px ≈ 64px
const ROW_HEIGHT = 64;
const BUFFER_ROWS = 6;
const scrollContainer = ref<HTMLElement | null>(null);
const tick = ref(0);

const totalHeight = computed(() => playerStore.tracks.length * ROW_HEIGHT);

interface VisibleItem { index: number; data: Track }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value; // 建立对滚动的响应依赖
  const el = scrollContainer.value;
  const all = playerStore.tracks;
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
const handleScroll = (e: Event) => {
  tick.value++;
  // 滚动到底加载更多（仅"全部歌曲"有分页）
  const target = e.target as HTMLElement;
  if (target.scrollTop + target.clientHeight >= target.scrollHeight - 200) {
    if (playerStore.activeLibraryTab === '全部歌曲') {
      playerStore.fetchTracks();
    }
  }
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
};

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

const playAll = () => {
  if (playerStore.tracks && playerStore.tracks.length > 0) {
    playerStore.playQueue(playerStore.tracks, 0);
  }
};

watch(() => playerStore.activeLibraryTab, () => {
  loadData();
});

onMounted(() => {
  loadData();
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- 加载中 -->
    <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length === 0" class="flex-1 flex flex-col items-center justify-center py-20 text-[#a0a0a0] tracking-[0.25em] text-xs">
      <span class="animate-pulse">LOADING METADATA...</span>
    </div>

    <!-- 加载出错 -->
    <div v-else-if="playerStore.isErrorTracks" class="flex-1 flex flex-col items-center justify-center py-20 text-[#d25050] tracking-[0.25em] text-xs font-bold uppercase">
      <span>加载曲库失败，请稍后重试</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="playerStore.tracks.length === 0" class="flex-1 flex flex-col items-center justify-center py-20">
      <p class="font-serif italic text-2xl text-black/60 mb-4">库中尚无音乐</p>
      <p class="text-xs text-[#a0a0a0] tracking-widest max-w-sm text-center leading-relaxed">
        暂未检测到本地音频文件，请在底部的 [设置] 页面中添加您的音乐文件夹并开始扫描。
      </p>
    </div>

    <!-- 正常曲目渲染列表 -->
    <template v-else>
      <!-- 播放全部按钮 -->
      <div class="flex items-center gap-4 mb-6 shrink-0 relative z-10">
        <button
          @click="playAll"
          class="flex items-center gap-2 bg-black text-white px-5 py-2 hover:bg-black/80 transition-all group rounded-sm shadow-sm"
        >
          <Play class="w-3.5 h-3.5 fill-current text-white" />
          <span class="text-[10px] font-bold tracking-[0.2em] uppercase">播放全部</span>
        </button>
      </div>

      <!-- Table Header -->
      <div class="flex items-center text-[10px] font-bold tracking-[0.15em] text-[#888] uppercase pb-4 mb-4 border-b border-black shrink-0 relative z-10">
        <div class="w-16 text-center">序号</div>
        <div class="flex-[2] pl-2">标题</div>
        <div class="flex-[1.5]">艺人</div>
        <div class="flex-[2]">专辑</div>
        <div class="w-20 text-right pr-4">时长</div>
        <div class="w-24 pl-4 text-left">格式</div>
      </div>

      <!-- 虚拟滚动歌曲列表 -->
      <div
        ref="scrollContainer"
        class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2"
        @scroll="handleScroll"
      >
        <div :style="{ height: totalHeight + 'px', position: 'relative' }">
          <div
            class="absolute top-0 left-0 right-0 will-change-transform"
            :style="{ transform: `translateY(${offsetY}px)` }"
          >
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @dblclick="playerStore.playQueue(playerStore.tracks, item.index)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/50 group transition-colors duration-200 cursor-pointer hover:bg-black/5"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <div class="w-16 text-center text-[#888]">
                <template v-if="playerStore.currentTrack?.id === item.data.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-black animate-pulse" />
                </template>
                <template v-else>
                  {{ String(item.index + 1).padStart(2, '0') }}
                </template>
              </div>
              <div class="flex-[2] pl-2 flex items-center gap-4">
                <Heart
                  class="w-3.5 h-3.5 transition-colors stroke-[1.5]"
                  :class="[
                    item.data.isFavorite ? 'text-black fill-current' : 'text-[#ccc] opacity-0 group-hover:opacity-100 hover:text-black'
                  ]"
                  @click.stop="playerStore.toggleFavorite(item.data.id)"
                />
                <div class="relative flex items-center">
                  <button @click.stop="openPlaylistMenu(item.data.id)" class="text-[#ccc] opacity-0 group-hover:opacity-100 hover:text-black transition-opacity" title="添加到歌单">
                    <Plus class="w-3.5 h-3.5 stroke-[1.5]" />
                  </button>
                  <div v-if="activeMenuTrackId === item.data.id" class="absolute left-6 top-0 bg-white border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                    <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-[#a0a0a0] whitespace-nowrap">暂无自建歌单</div>
                    <button
                      v-for="pl in playerStore.playlists"
                      :key="pl.id"
                      @click.stop="addToPlaylist(pl.id, item.data.id)"
                      class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-black hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
                    >
                      {{ pl.name }}
                    </button>
                  </div>
                </div>
                <span
                  class="truncate"
                  :class="playerStore.currentTrack?.id === item.data.id ? 'font-serif italic font-semibold text-[16px] text-black' : 'text-[#333] font-medium'"
                >{{ item.data.title }}</span>
              </div>
              <div class="flex-[1.5] truncate pr-4 text-[#555]">{{ item.data.artist }}</div>
              <div class="flex-[2] truncate pr-4 text-[#777] italic">{{ item.data.album }}</div>
              <div class="w-20 text-right pr-4 text-[#888]">{{ item.data.duration }}</div>
              <div class="w-24 pl-4 text-left text-[11px] text-[#aaa] tracking-wider">{{ item.data.format }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer Stats -->
      <div class="mt-4 pt-6 border-t border-[#e8e6df] text-[10px] font-bold tracking-[0.2em] text-[#888] shrink-0 relative z-10 flex items-center justify-between uppercase">
        <span>{{ playerStore.tracks.length }} 首歌曲</span>
        <div class="flex items-center gap-4">
          <div class="w-12 h-px bg-[#dcdad1]"></div>
          <span>本地归档</span>
        </div>
      </div>
    </template>
  </div>
</template>
