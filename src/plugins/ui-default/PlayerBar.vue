<script setup lang="ts">
import { 
  Heart, 
  Shuffle, 
  SkipBack, 
  Play, 
  Pause, 
  SkipForward, 
  Repeat, 
  Volume2, 
  ListMusic, 
  SlidersHorizontal, 
  Maximize2 
} from 'lucide-vue-next';
import { computed, onMounted, onUnmounted } from 'vue';
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';

const playerStore = usePlayerStore();

// 播放进度百分比
const progressPercent = computed(() => {
  const total = playerStore.currentTrack.durationSec;
  if (!total) return 0;
  return (playerStore.currentTime / total) * 100;
});

// 模拟播放计时器
let timer: ReturnType<typeof setInterval> | null = null;

const startTimer = () => {
  if (timer) clearInterval(timer);
  timer = setInterval(() => {
    if (playerStore.isPlaying) {
      if (playerStore.currentTime < playerStore.currentTrack.durationSec) {
        playerStore.currentTime++;
      } else {
        playerStore.nextTrack();
      }
    }
  }, 1000);
};

const stopTimer = () => {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
};

onMounted(() => {
  startTimer();
});

onUnmounted(() => {
  stopTimer();
});
</script>

<template>
  <footer class="h-[80px] bg-white border-t border-gray-100 flex items-center justify-between px-6 shrink-0 shadow-[0_-4px_20px_rgba(0,0,0,0.02)] z-20">
    <!-- Left: Track Info -->
    <div class="flex items-center gap-4 w-[300px]">
      <div 
        class="w-14 h-14 rounded-md shadow-sm overflow-hidden shrink-0 relative group cursor-pointer"
        :class="['bg-gradient-to-br', playerStore.currentTrack.coverColor]"
      >
        <img 
          v-if="playerStore.currentTrack?.cover_artwork_id"
          :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
          class="absolute inset-0 w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
        />
        <div class="absolute inset-0 bg-black/10 group-hover:bg-black/0 transition-smooth"></div>
      </div>
      <div class="flex flex-col justify-center overflow-hidden">
        <div class="flex items-center gap-2">
          <h3 class="font-semibold text-gray-900 text-sm truncate cursor-pointer hover:underline">{{ playerStore.currentTrack.title }}</h3>
          <Heart 
            class="w-4 h-4 cursor-pointer hover-scale transition-smooth" 
            :class="playerStore.currentTrack.isFavorite ? 'text-brand-orange fill-current' : 'text-gray-400 hover:text-gray-600'"
            @click="playerStore.currentTrack.isFavorite = !playerStore.currentTrack.isFavorite"
          />
        </div>
        <p class="text-xs text-gray-500 mt-0.5 truncate cursor-pointer hover:underline">{{ playerStore.currentTrack.artist }}</p>
      </div>
    </div>

    <!-- Center: Playback Controls & Progress -->
    <div class="flex-1 flex flex-col items-center justify-center max-w-[600px] px-8">
      <div class="flex items-center gap-6 mb-2">
        <button class="text-gray-400 hover:text-gray-700 transition-smooth hover-scale"><Shuffle class="w-[18px] h-[18px]" /></button>
        <button @click="playerStore.prevTrack" class="text-gray-600 hover:text-gray-900 transition-smooth hover-scale"><SkipBack class="w-5 h-5 fill-current" /></button>
        
        <button 
          @click="playerStore.togglePlay"
          class="w-10 h-10 rounded-full bg-brand-orange text-white flex items-center justify-center hover:bg-brand-orange-light hover:shadow-lg hover:shadow-brand-orange/30 transition-smooth hover-scale"
        >
          <Pause v-if="playerStore.isPlaying" class="w-5 h-5 fill-current" />
          <Play v-else class="w-5 h-5 fill-current ml-1" />
        </button>

        <button @click="playerStore.nextTrack()" class="text-gray-600 hover:text-gray-900 transition-smooth hover-scale"><SkipForward class="w-5 h-5 fill-current" /></button>
        <button class="text-gray-400 hover:text-gray-700 transition-smooth hover-scale"><Repeat class="w-[18px] h-[18px]" /></button>
      </div>
      
      <div class="w-full flex items-center gap-3 text-xs text-gray-400 font-medium font-mono">
        <span>{{ playerStore.formatTime(playerStore.currentTime) }}</span>
        <div class="flex-1 h-1.5 bg-gray-100 rounded-full overflow-hidden relative group cursor-pointer">
          <div 
            class="absolute left-0 top-0 h-full bg-brand-orange rounded-full group-hover:bg-brand-orange-light transition-colors"
            :style="{ width: progressPercent + '%' }"
          ></div>
          <!-- Slider Thumb (visible on hover) -->
          <div 
            class="absolute top-1/2 -translate-y-1/2 -ml-1.5 w-3 h-3 bg-white rounded-full shadow border border-gray-200 opacity-0 group-hover:opacity-100 transition-opacity"
            :style="{ left: progressPercent + '%' }"
          ></div>
        </div>
        <span>{{ playerStore.currentTrack.duration }}</span>
      </div>
    </div>

    <!-- Right: Volume & Actions -->
    <div class="flex items-center justify-end gap-5 w-[300px] text-gray-500">
      <div class="flex items-center gap-2 w-32 group cursor-pointer">
        <Volume2 class="w-[18px] h-[18px] text-gray-400 group-hover:text-gray-600 transition-smooth" />
        <div class="flex-1 h-1.5 bg-gray-100 rounded-full overflow-hidden relative">
          <div 
            class="absolute left-0 top-0 h-full bg-brand-orange-light rounded-full"
            :style="{ width: playerStore.volume + '%' }"
          ></div>
        </div>
      </div>
      
      <button 
        @click="playerStore.isRightPanelOpen = !playerStore.isRightPanelOpen" 
        class="transition-smooth hover-scale"
        :class="playerStore.isRightPanelOpen ? 'text-brand-orange' : 'hover:text-gray-800'"
      >
        <ListMusic class="w-[18px] h-[18px]" />
      </button>
      <button class="hover:text-gray-800 transition-smooth hover-scale"><SlidersHorizontal class="w-[18px] h-[18px]" /></button>
      <button class="hover:text-gray-800 transition-smooth hover-scale"><Maximize2 class="w-[18px] h-[18px]" /></button>
    </div>
  </footer>
</template>
