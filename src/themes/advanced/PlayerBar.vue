<script setup lang="ts">
import { 
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1,
  Volume2, Heart, Disc3
} from 'lucide-vue-next';
import { getArtworkUrl } from '../../utils';
import { usePlayerControls } from '../../composables/usePlayerControls';

// 完全复用业务逻辑
const {
  playerStore,
  progressPercent,
  handleProgressClick,
  handleVolumeClick,
  formatTimeMs,
  cyclePlayMode
} = usePlayerControls();
</script>

<template>
  <footer class="h-[120px] bg-white/5 backdrop-blur-2xl border-t border-white/10 flex items-center justify-between px-8 relative z-20">
    <!-- Left: Spinning Vinyl & Info -->
    <div class="flex items-center gap-6 w-[350px]">
      <div 
        class="w-20 h-20 rounded-full bg-black/80 shadow-[0_0_20px_rgba(0,0,0,0.5)] border-4 border-[#222] overflow-hidden relative flex items-center justify-center shrink-0"
        :class="playerStore.isPlaying ? 'animate-[spin_4s_linear_infinite]' : ''"
      >
        <!-- Vinyl Grooves -->
        <div class="absolute inset-0 rounded-full border border-white/10 m-2 pointer-events-none"></div>
        <div class="absolute inset-0 rounded-full border border-white/5 m-4 pointer-events-none"></div>
        
        <img 
          v-if="playerStore.currentTrack?.cover_artwork_id"
          :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
          class="w-8 h-8 rounded-full object-cover"
        />
        <div v-else class="w-8 h-8 rounded-full bg-gradient-to-tr from-purple-500 to-brand-orange flex items-center justify-center">
          <Disc3 class="w-4 h-4 text-white" />
        </div>
      </div>
      
      <div class="flex-1 min-w-0">
        <h3 class="text-white text-xl font-bold truncate tracking-wide drop-shadow-md">
          {{ playerStore.currentTrack?.title || 'Lumo Advanced' }}
        </h3>
        <p class="text-sm text-blue-300/80 font-medium truncate mt-1">
          {{ playerStore.currentTrack?.artist || 'Ready for immersion' }}
        </p>
      </div>
    </div>

    <!-- Center: Glowing Playback Controls -->
    <div class="flex-1 flex flex-col items-center max-w-[600px] px-8">
      <div class="flex items-center gap-8 mb-4">
        <button @click="cyclePlayMode" class="text-white/50 hover:text-white hover:drop-shadow-[0_0_8px_rgba(255,255,255,0.8)] transition-all">
          <Repeat v-if="playerStore.playMode === 'normal'" class="w-5 h-5" />
          <Repeat v-else-if="playerStore.playMode === 'repeat'" class="w-5 h-5 text-blue-400" />
          <Repeat1 v-else-if="playerStore.playMode === 'repeat-one'" class="w-5 h-5 text-blue-400" />
          <Shuffle v-else-if="playerStore.playMode === 'shuffle'" class="w-5 h-5 text-blue-400" />
        </button>

        <button @click="playerStore.prevTrack" class="text-white hover:drop-shadow-[0_0_10px_rgba(255,255,255,1)] transition-all">
          <SkipBack class="w-6 h-6 fill-current" />
        </button>
        
        <button 
          @click="playerStore.togglePlay"
          class="w-14 h-14 rounded-full bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-400 hover:to-purple-400 flex items-center justify-center text-white shadow-[0_0_20px_rgba(168,85,247,0.4)] transition-all hover:scale-105"
        >
          <Pause v-if="playerStore.isPlaying" class="w-6 h-6 fill-current" />
          <Play v-else class="w-6 h-6 fill-current ml-1" />
        </button>

        <button @click="playerStore.nextTrack(false)" class="text-white hover:drop-shadow-[0_0_10px_rgba(255,255,255,1)] transition-all">
          <SkipForward class="w-6 h-6 fill-current" />
        </button>
        
        <button @click="playerStore.toggleFavorite(playerStore.currentTrack?.id || 0)" class="text-white/50 hover:text-pink-500 transition-all">
          <Heart class="w-5 h-5" :class="{ 'fill-pink-500 text-pink-500 drop-shadow-[0_0_8px_rgba(236,72,153,0.8)]': playerStore.currentTrack?.isFavorite }" />
        </button>
      </div>
      
      <!-- Neon Progress Bar -->
      <div class="w-full flex items-center gap-4 text-xs text-white/50 font-mono">
        <span>{{ formatTimeMs(playerStore.progressMs) }}</span>
        <div class="flex-1 h-3 flex items-center group cursor-pointer" @click="handleProgressClick">
          <div class="w-full h-1.5 bg-white/10 rounded-full relative overflow-hidden">
            <div 
              class="absolute left-0 top-0 h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-300 shadow-[0_0_10px_rgba(168,85,247,0.8)]"
              :style="{ width: progressPercent + '%' }"
            ></div>
          </div>
        </div>
        <span>{{ formatTimeMs(playerStore.durationMs) }}</span>
      </div>
    </div>

    <!-- Right: Volume -->
    <div class="flex items-center justify-end w-[350px]">
      <div class="flex items-center gap-3 w-32 cursor-pointer group" @click="handleVolumeClick">
        <Volume2 class="w-5 h-5 text-white/50 group-hover:text-white transition-colors" />
        <div class="flex-1 h-3 flex items-center">
          <div class="w-full h-1.5 bg-white/10 rounded-full relative overflow-hidden">
            <div class="absolute left-0 top-0 h-full bg-blue-400 transition-all duration-150" :style="{ width: playerStore.volume + '%' }"></div>
          </div>
        </div>
      </div>
    </div>
  </footer>
</template>
