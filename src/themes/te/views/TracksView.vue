<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
import { Heart, Search, Filter, LayoutList } from 'lucide-vue-next';
import { ref, computed } from 'vue';

const playerStore = usePlayerStore();

// Virtual list state
const listContainer = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const itemHeight = 40; // 40px row height for TE compact feel

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
  <div class="h-full flex flex-col font-mono text-xs uppercase bg-bg-base">
    
    <!-- Header -->
    <div class="p-6 shrink-0 border-b border-border-color bg-bg-base">
      <div class="flex items-end justify-between mb-2">
        <h1 class="text-2xl font-bold tracking-widest text-text-main">ALL TRACKS</h1>
        <div class="flex gap-6 items-end">
          <div class="flex items-center gap-2 border-b border-border-color pb-1 text-text-muted">
            <span class="text-[10px]">SEARCH</span>
            <Search class="w-3 h-3" />
          </div>
          <div class="flex items-center gap-2 border-b border-border-color pb-1 text-text-muted">
            <span class="text-[10px]">FILTER</span>
            <Filter class="w-3 h-3" />
          </div>
          <div class="flex items-center gap-2 border-b border-border-color pb-1 text-text-muted">
            <span class="text-[10px]">VIEW</span>
            <LayoutList class="w-3 h-3" />
          </div>
        </div>
      </div>
      <div class="flex gap-4 text-[10px] text-text-muted font-bold tracking-widest">
        <span>{{ playerStore.tracks.length.toLocaleString() }} TRACKS</span>
        <span>982 GB</span>
      </div>
    </div>

    <!-- Table Header -->
    <div class="px-6 flex items-center h-10 border-b border-border-color text-[10px] text-text-muted font-bold tracking-widest shrink-0 uppercase">
      <div class="w-12">#</div>
      <div class="flex-1 min-w-[200px]">TITLE</div>
      <div class="w-48 shrink-0">ARTIST</div>
      <div class="w-48 shrink-0">ALBUM</div>
      <div class="w-20 shrink-0 text-right">TIME</div>
      <div class="w-20 shrink-0 text-right pr-6">FORMAT</div>
    </div>

    <!-- Virtual List Body -->
    <div class="flex-1 overflow-auto custom-scrollbar" ref="listContainer" @scroll="handleScroll">
      <div class="relative w-full" :style="{ height: `${totalHeight}px` }">
        <div 
          class="absolute left-0 right-0"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <div 
            v-for="{ item: track, index } in visibleData" 
            :key="track.id"
            class="flex items-center px-6 transition-colors cursor-pointer border-b border-border-color/20 group hover:bg-bg-panel"
            :class="playerStore.currentTrack?.id === track.id 
              ? 'bg-text-main text-bg-base font-bold shadow-[inset_0_1px_0_var(--border-color),inset_0_-1px_0_var(--border-color)]' 
              : 'text-text-main'"
            :style="{ height: `${itemHeight}px` }"
            @dblclick="playerStore.playTrack(index)"
          >
            <!-- ID & Favorite -->
            <div class="w-12 flex items-center gap-2 text-[10px]">
              <span :class="playerStore.currentTrack?.id === track.id ? 'opacity-100' : 'opacity-70'">
                {{ (index + 1).toString().padStart(2, '0') }}
              </span>
              <button 
                @click.stop="playerStore.toggleFavorite(track.id)"
                class="opacity-0 group-hover:opacity-100 transition-opacity"
                :class="{'opacity-100 text-bg-base': playerStore.currentTrack?.id === track.id}"
              >
                <Heart class="w-3 h-3 stroke-[2]" :class="{'fill-current': track.isFavorite}" />
              </button>
            </div>
            
            <div class="flex-1 min-w-[200px] truncate pr-4">{{ track.title }}</div>
            <div class="w-48 shrink-0 truncate pr-4 opacity-90">{{ track.artist || 'UNKNOWN' }}</div>
            <div class="w-48 shrink-0 truncate pr-4 opacity-90">{{ track.album || 'UNKNOWN' }}</div>
            <div class="w-20 shrink-0 text-right pr-4 opacity-90">{{ track.duration }}</div>
            <div class="w-20 shrink-0 pr-6 flex justify-end items-center gap-4">
              <span class="opacity-90">{{ track.format || 'FLAC' }}</span>
              <div v-if="playerStore.currentTrack?.id === track.id && playerStore.isPlaying" class="flex gap-0.5 items-end h-3">
                <div class="w-[2px] bg-bg-base animate-[bounce_0.8s_infinite] h-2"></div>
                <div class="w-[2px] bg-bg-base animate-[bounce_1s_infinite_0.2s] h-3"></div>
                <div class="w-[2px] bg-bg-base animate-[bounce_0.9s_infinite_0.4s] h-1.5"></div>
                <div class="w-[2px] bg-bg-base animate-[bounce_1.1s_infinite_0.1s] h-2.5"></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 0;
}
</style>
