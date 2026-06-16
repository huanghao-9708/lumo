<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
import { Heart, Search, Filter, LayoutList, Play, MoreHorizontal } from 'lucide-vue-next';
import { ref, computed } from 'vue';

const playerStore = usePlayerStore();

// Virtual list state
const listContainer = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const itemHeight = 44; // Comfortable row height

// Virtual list computed
const visibleCount = computed(() => {
  if (!listContainer.value) return 20;
  return Math.ceil(listContainer.value.clientHeight / itemHeight) + 4;
});

const startIndex = computed(() => {
  return Math.max(0, Math.floor(scrollTop.value / itemHeight) - 2);
});

const endIndex = computed(() => {
  return Math.min(playerStore.tracks.length, startIndex.value + visibleCount.value);
});

const visibleData = computed(() => {
  return playerStore.tracks.slice(startIndex.value, endIndex.value).map((track, idx) => ({
    item: track,
    index: startIndex.value + idx
  }));
});

const totalHeight = computed(() => playerStore.tracks.length * itemHeight);
const offsetY = computed(() => startIndex.value * itemHeight);

const handleScroll = () => {
  if (listContainer.value) {
    scrollTop.value = listContainer.value.scrollTop;
  }
};
</script>

<template>
  <div class="h-full flex flex-col bg-bg-base font-sans text-sm">
    
    <!-- Header -->
    <div class="px-10 pt-10 pb-6 shrink-0 bg-bg-base">
      <div class="flex items-end justify-between mb-4">
        <h1 class="text-[28px] font-bold text-text-main tracking-wide">全部歌曲</h1>
        
        <div class="flex items-center gap-6 pb-1 text-xs">
          <!-- Search -->
          <div class="flex items-center gap-2 border-b border-border-color pb-1 w-48 text-text-muted">
            <input type="text" placeholder="搜索" class="bg-transparent border-none outline-none w-full text-text-main placeholder:text-text-muted/60" />
            <Search class="w-3.5 h-3.5" />
          </div>
          
          <button class="flex items-center gap-1 border-b border-border-color pb-1 text-text-main font-medium hover:text-accent transition-colors">
            <span>筛选</span>
            <Filter class="w-3.5 h-3.5" />
          </button>
          
          <button class="flex items-center gap-1 border-b border-border-color pb-1 text-text-main font-medium hover:text-accent transition-colors">
            <span>视图</span>
            <LayoutList class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
      <div class="flex gap-2 text-xs text-text-muted">
        <span>{{ playerStore.tracks.length.toLocaleString() }} 首歌曲</span>
        <span>&middot;</span>
        <span>982 GB</span>
        <span>&middot;</span>
        <span>31 天连续播放时长</span>
      </div>
    </div>

    <!-- Table Header -->
    <div class="px-10 flex items-center h-10 border-b border-border-color/60 text-[11px] text-text-muted font-medium shrink-0">
      <div class="w-12">#</div>
      <div class="w-8"></div>
      <div class="flex-1 min-w-[200px]">标题</div>
      <div class="w-48 shrink-0">艺术家</div>
      <div class="w-48 shrink-0">专辑</div>
      <div class="w-20 shrink-0 text-right">时长</div>
      <div class="w-20 shrink-0 text-center">格式</div>
      <div class="w-32 shrink-0 text-right pr-6">比特率</div>
    </div>

    <!-- Virtual List Body -->
    <div class="flex-1 overflow-auto custom-scrollbar relative" ref="listContainer" @scroll="handleScroll">
      <div class="relative w-full" :style="{ height: `${totalHeight}px` }">
        <div 
          class="absolute left-0 right-0"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <div 
            v-for="{ item: track, index } in visibleData" 
            :key="track.id"
            class="flex items-center px-10 transition-colors cursor-pointer border-b border-border-color/30 group hover:bg-bg-active/30"
            :class="playerStore.currentTrack?.id === track.id ? 'bg-bg-active/50' : ''"
            :style="{ height: `${itemHeight}px` }"
            @dblclick="playerStore.playTrack(index)"
          >
            <!-- ID / Play Icon -->
            <div class="w-12 flex items-center gap-2 text-xs font-mono text-text-main/80">
              <Play v-if="playerStore.currentTrack?.id === track.id" class="w-3 h-3 fill-text-main text-text-main" />
              <span v-else>{{ (index + 1).toString().padStart(2, '0') }}</span>
            </div>
            
            <!-- Favorite Heart -->
            <div class="w-8 flex items-center">
              <button 
                @click.stop="playerStore.toggleFavorite(track.id)"
                class="opacity-0 group-hover:opacity-100 transition-opacity"
                :class="{'opacity-100': track.isFavorite}"
              >
                <Heart class="w-3.5 h-3.5" :class="track.isFavorite ? 'fill-accent text-accent stroke-accent' : 'text-text-muted stroke-[2]'" />
              </button>
            </div>
            
            <!-- Title -->
            <div class="flex-1 min-w-[200px] pr-4 flex items-center">
              <span class="truncate font-medium text-text-main" :class="{'font-bold': playerStore.currentTrack?.id === track.id}">{{ track.title }}</span>
            </div>
            
            <!-- Artist -->
            <div class="w-48 shrink-0 pr-4 truncate text-text-main/90">{{ track.artist || 'Unknown Artist' }}</div>
            
            <!-- Album -->
            <div class="w-48 shrink-0 pr-4 truncate text-text-main/90">{{ track.album || 'Unknown Album' }}</div>
            
            <!-- Duration -->
            <div class="w-20 shrink-0 text-right pr-4 font-mono text-text-muted/80 text-[11px]">{{ track.duration }}</div>
            
            <!-- Format -->
            <div class="w-20 shrink-0 text-center font-mono text-text-main/70 text-[11px]">{{ track.format || 'FLAC' }}</div>
            
            <!-- Bitrate & More -->
            <div class="w-32 shrink-0 pr-2 flex justify-end items-center gap-4 text-[11px] font-mono text-text-muted/80">
              <span class="truncate">24bit / 96kHz</span>
              <button class="opacity-0 group-hover:opacity-100 text-text-muted hover:text-text-main transition-all">
                <MoreHorizontal class="w-4 h-4" />
              </button>
            </div>
            
          </div>
        </div>
      </div>
    </div>
    
    <!-- Bottom Stats -->
    <div class="px-10 h-14 shrink-0 flex items-center justify-between text-xs text-text-muted border-t border-border-color/60">
      <div class="flex gap-4">
        <span>{{ playerStore.tracks.length.toLocaleString() }} 首歌曲</span>
        <span>982 GB</span>
      </div>
      <div>总时长: 31 天 7 小时 42 分钟</div>
    </div>

  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 6px;
}
</style>
