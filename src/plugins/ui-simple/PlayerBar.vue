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
import { computed, onMounted, onUnmounted } from 'vue';
import { usePlayerStore } from '../../stores/player';

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

onMounted(() => startTimer());
onUnmounted(() => stopTimer());
</script>

<template>
  <footer class="h-[100px] bg-transparent border-t border-[#e8e6df] flex items-center justify-between px-10 shrink-0 relative z-20">
    <!-- Left: Track Info -->
    <div class="flex items-center gap-6 w-[300px]">
      <div 
        class="w-[50px] h-[50px] overflow-hidden shrink-0 relative border border-[#dcdad1]/50 bg-[#eae8e1] transition-colors duration-700"
        :class="playerStore.currentTrack.coverColor"
      >
        <div class="absolute inset-0 opacity-20" style="background-image: radial-gradient(circle, #000 1px, transparent 1px); background-size: 4px 4px;"></div>
      </div>
      <div class="flex flex-col justify-center overflow-hidden">
        <h3 class="font-serif italic font-semibold text-black text-lg truncate mb-1">{{ playerStore.currentTrack.title }}</h3>
        <p class="text-[10px] text-[#555] font-bold tracking-widest uppercase truncate">{{ playerStore.currentTrack.artist }}</p>
      </div>
    </div>

    <!-- Center: Playback Controls & Progress -->
    <div class="flex-1 flex flex-col items-center justify-center max-w-[800px] px-8">
      <div class="flex items-center gap-10 mb-4">
        <button class="text-[#a0a0a0] hover:text-black transition-colors"><Shuffle class="w-4 h-4 stroke-[1.5]" /></button>
        <button @click="playerStore.prevTrack" class="text-black hover:opacity-70 transition-opacity"><SkipBack class="w-5 h-5 fill-current" /></button>
        
        <button 
          @click="playerStore.togglePlay"
          class="w-8 h-8 flex items-center justify-center text-black hover:opacity-70 transition-opacity"
        >
          <Pause v-if="playerStore.isPlaying" class="w-6 h-6 fill-current" />
          <Play v-else class="w-6 h-6 fill-current ml-1" />
        </button>

        <button @click="playerStore.nextTrack" class="text-black hover:opacity-70 transition-opacity"><SkipForward class="w-5 h-5 fill-current" /></button>
        <button class="text-[#a0a0a0] hover:text-black transition-colors"><Repeat class="w-4 h-4 stroke-[1.5]" /></button>
      </div>
      
      <div class="w-full flex items-center gap-6 text-[10px] text-[#888] font-bold tracking-widest">
        <span>{{ playerStore.formatTime(playerStore.currentTime) }}</span>
        <div class="flex-1 h-px bg-[#dcdad1] relative group cursor-pointer">
          <div 
            class="absolute left-0 top-0 h-full bg-black transition-all duration-1000 ease-linear"
            :style="{ width: progressPercent + '%' }"
          ></div>
          <div 
            class="absolute top-1/2 -translate-y-1/2 w-[2px] h-3 bg-black transition-all duration-1000 ease-linear"
            :style="{ left: progressPercent + '%' }"
          ></div>
        </div>
        <span>{{ playerStore.currentTrack.duration }}</span>
      </div>
    </div>

    <!-- Right: Volume & Actions -->
    <div class="flex items-center justify-end gap-8 w-[300px] text-[#888]">
      <div class="flex items-center gap-4 w-32 group cursor-pointer">
        <Volume2 class="w-4 h-4 stroke-[1.5] group-hover:text-black transition-colors" />
        <div class="flex-1 h-px bg-[#dcdad1] relative">
          <div class="absolute left-0 top-0 h-full bg-[#555]" :style="{ width: playerStore.volume + '%' }"></div>
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
