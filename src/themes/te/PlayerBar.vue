<script setup lang="ts">
import { 
  Shuffle, SkipBack, Play, SkipForward, Repeat, Repeat1,
  Volume2, Heart, VolumeX
} from 'lucide-vue-next';
import { usePlayerControls } from '../../composables/usePlayerControls';
import { ref } from 'vue';

const {
  playerStore,
  progressPercent,
  handleProgressClick,
  handleVolumeClick,
  formatTimeMs,
  cyclePlayMode
} = usePlayerControls();

// Simple mock for a waveform (TE style often has these little visualizations)
const waveformBars = ref(Array.from({ length: 40 }, () => Math.random() * 100));
</script>

<template>
  <footer class="h-28 bg-bg-base border-t border-border-color flex items-stretch font-mono text-xs uppercase tracking-widest text-text-main relative z-20">
    
    <!-- Left: Metadata & Waveform -->
    <div class="w-80 shrink-0 border-r border-border-color p-4 flex flex-col justify-between">
      <div>
        <p class="text-[9px] text-text-muted mb-1 font-bold">PLAYING</p>
        <div class="flex items-center gap-2">
          <h3 class="font-bold truncate text-sm">{{ playerStore.currentTrack?.title || 'NO TRACK' }}</h3>
          <button 
            v-if="playerStore.currentTrack"
            @click="playerStore.toggleFavorite(playerStore.currentTrack.id)"
          >
            <Heart class="w-3 h-3 stroke-[2]" :class="{'fill-current text-text-main': playerStore.currentTrack?.isFavorite}" />
          </button>
        </div>
        <p class="text-[10px] text-text-muted truncate">{{ playerStore.currentTrack?.artist || 'UNKNOWN' }}</p>
      </div>
      
      <!-- Waveform Progress Bar -->
      <div 
        class="h-8 flex items-end gap-[1px] cursor-pointer mt-2 group relative"
        @click="handleProgressClick"
      >
        <div 
          v-for="(bar, i) in waveformBars" 
          :key="i"
          class="flex-1 bg-border-color/30 transition-all duration-75"
          :class="[
            (i / waveformBars.length) * 100 <= progressPercent ? 'bg-text-main' : 'bg-border-color/20',
            playerStore.isPlaying ? 'animate-pulse' : ''
          ]"
          :style="{ height: `${Math.max(10, bar)}%` }"
        ></div>
        <!-- Progress overlay -->
        <div class="absolute inset-0 bg-transparent group-hover:bg-text-main/5 transition-colors"></div>
      </div>
    </div>

    <!-- Center: Playback Controls -->
    <div class="flex-1 flex flex-col">
      <!-- Progress Track (Alternative or extra precision) -->
      <div class="h-1 bg-border-color/20 w-full relative cursor-pointer" @click="handleProgressClick">
        <div class="absolute left-0 top-0 bottom-0 bg-text-main" :style="{ width: progressPercent + '%' }"></div>
      </div>
      
      <div class="flex-1 flex items-center justify-center gap-12 relative">
        <!-- Time indicators -->
        <div class="absolute left-6 bottom-4 text-[10px] font-bold">{{ formatTimeMs(playerStore.progressMs) }}</div>
        <div class="absolute right-6 bottom-4 text-[10px] font-bold">{{ formatTimeMs(playerStore.durationMs) }}</div>

        <div class="flex items-center gap-8">
          <!-- Shuffle -->
          <button @click="cyclePlayMode" class="flex flex-col items-center gap-2 text-text-muted hover:text-text-main transition-colors w-12">
            <Shuffle v-if="playerStore.playMode === 'shuffle'" class="w-4 h-4 text-text-main" />
            <Repeat1 v-else-if="playerStore.playMode === 'repeat-one'" class="w-4 h-4 text-text-main" />
            <Repeat v-else-if="playerStore.playMode === 'repeat'" class="w-4 h-4 text-text-main" />
            <Shuffle v-else class="w-4 h-4" />
            <span class="text-[9px] font-bold scale-90">{{ playerStore.playMode.toUpperCase() }}</span>
          </button>

          <!-- Prev -->
          <button @click="playerStore.prevTrack" class="flex flex-col items-center gap-2 text-text-main hover:opacity-70 transition-opacity w-12">
            <SkipBack class="w-5 h-5 fill-current" />
            <span class="text-[9px] font-bold scale-90">PREV</span>
          </button>

          <!-- Play/Pause (The iconic TE massive double line) -->
          <button 
            @click="playerStore.togglePlay"
            class="flex flex-col items-center gap-2 text-text-main hover:opacity-70 transition-opacity w-16"
          >
            <div class="h-10 flex items-center justify-center">
              <div v-if="playerStore.isPlaying" class="flex gap-2 h-8">
                <div class="w-2.5 bg-text-main rounded-[1px]"></div>
                <div class="w-2.5 bg-text-main rounded-[1px]"></div>
              </div>
              <Play v-else class="w-10 h-10 fill-current ml-1" />
            </div>
            <span class="text-[9px] font-bold scale-90 mt-1">PLAY / PAUSE</span>
          </button>

          <!-- Next -->
          <button @click="playerStore.nextTrack(false)" class="flex flex-col items-center gap-2 text-text-main hover:opacity-70 transition-opacity w-12">
            <SkipForward class="w-5 h-5 fill-current" />
            <span class="text-[9px] font-bold scale-90">NEXT</span>
          </button>

          <!-- Repeat (already handled by cyclePlayMode, but keeping layout symmetry) -->
          <div class="w-12"></div>
        </div>
      </div>
    </div>

    <!-- Right: Volume & Output -->
    <div class="w-80 shrink-0 border-l border-border-color p-6 flex flex-col justify-center gap-6">
      
      <!-- Volume -->
      <div class="flex items-center gap-4">
        <span class="text-[10px] w-12 font-bold">VOLUME</span>
        <span class="text-[10px] w-6 font-bold text-right">{{ playerStore.volume }}</span>
        <div class="flex-1 h-6 flex items-center relative cursor-pointer" @click="handleVolumeClick">
          <!-- Slider Track -->
          <div class="w-full h-px bg-border-color absolute top-1/2 -translate-y-1/2"></div>
          <!-- Slider Thumb (Fader style) -->
          <div 
            class="w-1.5 h-4 bg-text-main absolute top-1/2 -translate-y-1/2 -ml-[3px]"
            :style="{ left: playerStore.volume + '%' }"
          ></div>
        </div>
        <VolumeX v-if="playerStore.volume === 0" class="w-4 h-4 ml-2" />
        <Volume2 v-else class="w-4 h-4 ml-2" />
      </div>

      <!-- Output -->
      <div class="flex items-center gap-4">
        <span class="text-[10px] w-12 font-bold">OUTPUT</span>
        <div class="flex-1 border-b border-border-color flex justify-between items-center pb-1 cursor-pointer hover:opacity-70">
          <span class="font-bold text-[10px]">BUILT-IN</span>
          <span class="text-[8px]">▼</span>
        </div>
        <div class="w-4 flex items-end gap-[1px] h-3 ml-2">
           <div class="w-1 bg-text-main h-full"></div>
           <div class="w-1 bg-text-main h-2/3"></div>
           <div class="w-1 bg-text-main h-1/2"></div>
        </div>
      </div>

    </div>

  </footer>
</template>
