<script setup lang="ts">
import { 
  Shuffle, 
  SkipBack, 
  Play, 
  Pause, 
  SkipForward, 
  Repeat, 
  Volume2, 
  ListMusic, 
  SlidersHorizontal
} from 'lucide-vue-next';
import { computed } from 'vue';
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';

const playerStore = usePlayerStore();

// 播放进度百分比
const progressPercent = computed(() => {
  const total = playerStore.durationMs;
  if (!total) return 0;
  return Math.min(100, Math.max(0, (playerStore.progressMs / total) * 100));
});

const handleProgressClick = (e: MouseEvent) => {
  const target = e.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const percent = (e.clientX - rect.left) / rect.width;
  if (playerStore.durationMs > 0) {
    playerStore.seek(Math.floor(percent * playerStore.durationMs));
  }
};

const handleVolumeClick = (e: MouseEvent) => {
  const target = e.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  // Limit target matching to inner line rect if needed, but here target is the wrapper
  const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  playerStore.setVolume(Math.floor(percent * 100));
};

const formatTimeMs = (ms: number) => {
  const seconds = Math.floor(ms / 1000);
  const min = Math.floor(seconds / 60);
  const sec = seconds % 60;
  return `${min.toString().padStart(2, '0')}:${sec.toString().padStart(2, '0')}`;
};


</script>

<template>
  <footer class="h-[100px] bg-transparent border-t border-[#e8e6df] flex items-center justify-between px-10 shrink-0 relative z-20">
    <!-- Left: Track Info -->
    <div class="flex items-center gap-6 w-[300px]">
      <div class="w-12 h-12 bg-[#e8e6df] rounded-sm overflow-hidden shadow-sm shrink-0 relative group">
        <img 
          v-if="playerStore.currentTrack?.cover_artwork_id"
          :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
          class="absolute inset-0 w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
        />
        <div v-else class="absolute inset-0 opacity-20" style="background-image: radial-gradient(circle, #000 1px, transparent 1px); background-size: 4px 4px;"></div>
      </div>
      <div class="flex flex-col justify-center overflow-hidden">
        <h3 class="font-serif italic font-semibold text-black text-lg truncate mb-1">{{ playerStore.currentTrack?.title || 'Lumo Music' }}</h3>
        <p class="text-[10px] text-[#555] font-bold tracking-widest uppercase truncate">{{ playerStore.currentTrack?.artist_name || playerStore.currentTrack?.artist || 'Ready to play' }}</p>
      </div>
    </div>

    <!-- Center: Playback Controls & Progress -->
    <div class="flex-1 flex flex-col items-center justify-center max-w-[800px] px-8">
      <div class="flex items-center gap-10 mb-4">
        <button 
          class="transition-colors"
          :class="playerStore.playMode === 'shuffle' ? 'text-black' : 'text-[#a0a0a0] hover:text-black'"
          @click="playerStore.playMode = 'shuffle'"
        >
          <Shuffle class="w-4 h-4 stroke-[1.5]" />
        </button>
        <button @click="playerStore.prevTrack" class="text-black hover:opacity-70 transition-opacity"><SkipBack class="w-5 h-5 fill-current" /></button>
        
        <button 
          @click="playerStore.togglePlay"
          class="w-8 h-8 flex items-center justify-center text-black hover:opacity-70 transition-opacity"
        >
          <Pause v-if="playerStore.isPlaying" class="w-6 h-6 fill-current" />
          <Play v-else class="w-6 h-6 fill-current ml-1" />
        </button>

        <button @click="playerStore.nextTrack" class="text-black hover:opacity-70 transition-opacity"><SkipForward class="w-5 h-5 fill-current" /></button>
        <button 
          class="transition-colors"
          :class="playerStore.playMode === 'repeat' ? 'text-black' : 'text-[#a0a0a0] hover:text-black'"
          @click="playerStore.playMode = 'repeat'"
        >
          <Repeat class="w-4 h-4 stroke-[1.5]" />
        </button>
      </div>
      
      <div class="w-full flex items-center gap-6 text-[10px] text-[#888] font-bold tracking-widest">
        <span>{{ formatTimeMs(playerStore.progressMs) }}</span>
        <div class="flex-1 h-[10px] flex items-center relative group cursor-pointer" @click="handleProgressClick">
          <div class="w-full h-px bg-[#dcdad1] relative pointer-events-none">
            <div 
              class="absolute left-0 top-0 h-full bg-black transition-all duration-300 ease-linear"
              :style="{ width: progressPercent + '%' }"
            ></div>
            <div 
              class="absolute top-1/2 -translate-y-1/2 w-[2px] h-3 bg-black transition-all duration-300 ease-linear"
              :style="{ left: progressPercent + '%' }"
            ></div>
          </div>
        </div>
        <span>{{ formatTimeMs(playerStore.durationMs) }}</span>
      </div>
    </div>

    <!-- Right: Volume & Actions -->
    <div class="flex items-center justify-end gap-8 w-[300px] text-[#888]">
      <div class="flex items-center gap-4 w-32 group cursor-pointer" @click="handleVolumeClick">
        <Volume2 class="w-4 h-4 stroke-[1.5] group-hover:text-black transition-colors" />
        <div class="flex-1 h-[10px] flex items-center relative pointer-events-none">
          <div class="w-full h-px bg-[#dcdad1] relative">
            <div class="absolute left-0 top-0 h-full bg-[#555] transition-all duration-150" :style="{ width: playerStore.volume + '%' }"></div>
          </div>
        </div>
      </div>
      
      <button 
        @click="playerStore.isRightPanelOpen = !playerStore.isRightPanelOpen" 
        class="transition-colors"
        :class="playerStore.isRightPanelOpen ? 'text-black' : 'hover:text-black'"
      >
        <ListMusic class="w-4 h-4 stroke-[1.5]" />
      </button>
      <button class="hover:text-black transition-colors"><SlidersHorizontal class="w-4 h-4 stroke-[1.5]" /></button>
    </div>
  </footer>
</template>
