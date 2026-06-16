<script setup lang="ts">
import { 
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, Repeat1
} from 'lucide-vue-next';
import { usePlayerControls } from '../../composables/usePlayerControls';
import { getArtworkUrl } from '../../utils';
import { ref, onUnmounted, computed } from 'vue';

const {
  playerStore,
  progressPercent,
  handleProgressClick,
  formatTimeMs,
  cyclePlayMode
} = usePlayerControls();

// Knob logic
const knobContainer = ref<HTMLElement | null>(null);
const isDragging = ref(false);

const knobRotation = computed(() => {
  // Volume 0-100 maps to -135deg to +135deg
  return -135 + (playerStore.volume / 100) * 270;
});

const updateVolumeFromEvent = (e: MouseEvent) => {
  if (!knobContainer.value) return;
  const rect = knobContainer.value.getBoundingClientRect();
  const centerX = rect.left + rect.width / 2;
  const centerY = rect.top + rect.height / 2;
  
  const x = e.clientX - centerX;
  const y = e.clientY - centerY;
  
  let angle = Math.atan2(y, x) * (180 / Math.PI);
  // Default atan2: right is 0, bottom is 90, left is 180/-180, top is -90
  // We want to map it to our -135 to 135 range (where top is 0)
  angle = angle + 90; // now top is 0
  
  if (angle > 180) angle -= 360;
  
  if (angle < -135) angle = -135;
  if (angle > 135) angle = 135;
  
  const percent = (angle + 135) / 270 * 100;
  playerStore.setVolume(Math.round(percent));
};

const handleKnobMouseDown = (e: MouseEvent) => {
  isDragging.value = true;
  updateVolumeFromEvent(e);
  window.addEventListener('mousemove', handleKnobMouseMove);
  window.addEventListener('mouseup', handleKnobMouseUp);
};

const handleKnobMouseMove = (e: MouseEvent) => {
  if (isDragging.value) {
    updateVolumeFromEvent(e);
  }
};

const handleKnobMouseUp = () => {
  isDragging.value = false;
  window.removeEventListener('mousemove', handleKnobMouseMove);
  window.removeEventListener('mouseup', handleKnobMouseUp);
};

onUnmounted(() => {
  window.removeEventListener('mousemove', handleKnobMouseMove);
  window.removeEventListener('mouseup', handleKnobMouseUp);
});

// Mock waveform bars
const waveformBars = ref(Array.from({ length: 50 }, () => Math.max(10, Math.random() * 100)));

</script>

<template>
  <footer class="h-32 bg-[#ebe7e0] border-t border-border-color flex items-stretch font-sans z-20 shadow-[0_-10px_40px_rgba(0,0,0,0.03)] dark:bg-[#1a1a1a]">
    
    <!-- Left: Metadata & Mini Waveform -->
    <div class="w-[320px] shrink-0 p-6 flex gap-4 items-center relative">
      <div class="w-20 h-20 rounded-lg overflow-hidden shrink-0 shadow-sm relative group bg-[#dcd8cf]">
        <img 
          v-if="playerStore.currentTrack?.cover_artwork_id"
          :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
          class="w-full h-full object-cover"
        />
      </div>
      
      <div class="flex-1 min-w-0 flex flex-col justify-center">
        <h3 class="font-bold text-sm truncate text-text-main mb-0.5">{{ playerStore.currentTrack?.title || 'Experience' }}</h3>
        <p class="text-[11px] text-text-main font-medium truncate mb-1">
          {{ playerStore.currentTrack?.artist || 'Ludovico Einaudi' }} &middot; <span class="text-text-muted">{{ playerStore.currentTrack?.album || 'Divenire' }}</span>
        </p>
        <p class="text-[9px] text-text-muted uppercase tracking-widest font-mono mb-2">
          {{ playerStore.currentTrack?.format || 'FLAC' }} &middot; 24bit / 96kHz
        </p>
        
        <!-- Mini Waveform -->
        <div class="h-4 flex items-end gap-[1px]">
           <div 
             v-for="(bar, i) in waveformBars" 
             :key="i"
             class="flex-1 bg-text-muted/40 transition-all duration-75 rounded-[1px]"
             :class="[(i / waveformBars.length) * 100 <= progressPercent ? 'bg-text-main' : 'bg-text-muted/30']"
             :style="{ height: `${bar}%` }"
           ></div>
        </div>
      </div>
    </div>

    <!-- Center: Controls & Timeline -->
    <div class="flex-1 flex flex-col items-center justify-center relative px-8">
      
      <!-- Top controls -->
      <div class="flex items-center gap-8 mb-4">
        <!-- Shuffle -->
        <button @click="cyclePlayMode" class="text-text-muted hover:text-text-main transition-colors">
          <Shuffle v-if="playerStore.playMode === 'shuffle'" class="w-4 h-4 text-accent" />
          <Repeat1 v-else-if="playerStore.playMode === 'repeat-one'" class="w-4 h-4 text-accent" />
          <Repeat v-else-if="playerStore.playMode === 'repeat'" class="w-4 h-4 text-accent" />
          <Shuffle v-else class="w-4 h-4" />
        </button>

        <!-- Prev -->
        <button @click="playerStore.prevTrack" class="text-text-main hover:opacity-70 transition-opacity">
          <SkipBack class="w-5 h-5 fill-current" />
        </button>

        <!-- Solid Play/Pause Circle -->
        <button 
          @click="playerStore.togglePlay"
          class="w-12 h-12 rounded-full bg-text-main text-bg-base flex items-center justify-center hover:scale-105 transition-transform shadow-md"
        >
          <Pause v-if="playerStore.isPlaying" class="w-5 h-5 fill-current" />
          <Play v-else class="w-5 h-5 fill-current ml-1" />
        </button>

        <!-- Next -->
        <button @click="playerStore.nextTrack(false)" class="text-text-main hover:opacity-70 transition-opacity">
          <SkipForward class="w-5 h-5 fill-current" />
        </button>

        <!-- Repeat toggle explicitly? Kept symmetric for visual balance -->
        <button @click="cyclePlayMode" class="text-text-muted hover:text-text-main transition-colors opacity-0">
          <Repeat class="w-4 h-4" />
        </button>
      </div>

      <!-- Ruler Timeline Progress Bar -->
      <div class="w-full max-w-2xl flex items-center gap-4 text-[10px] font-mono text-text-muted font-bold">
        <span class="w-10 text-right">{{ formatTimeMs(playerStore.progressMs) }}</span>
        
        <div 
          class="flex-1 h-8 relative cursor-pointer group flex flex-col justify-center"
          @click="handleProgressClick"
        >
          <!-- Ruler Background -->
          <div class="absolute inset-x-0 top-1/2 h-3 -translate-y-1/2 bg-transparent pointer-events-none"
               style="background-image: repeating-linear-gradient(to right, var(--border-color) 0, var(--border-color) 1px, transparent 1px, transparent 10px); background-size: 10px 100%;">
          </div>
          
          <!-- Colored Progress Fill Overlay -->
          <div class="absolute left-0 top-1/2 h-3 -translate-y-1/2 bg-accent/20 pointer-events-none transition-all duration-75"
               :style="{ width: progressPercent + '%' }">
          </div>
          
          <!-- Track Base Line -->
          <div class="w-full h-[1px] bg-border-color absolute top-1/2 -translate-y-1/2"></div>
          <div class="h-[1.5px] bg-accent absolute top-1/2 -translate-y-1/2" :style="{ width: progressPercent + '%' }"></div>
          
          <!-- Orange Knob Indicator -->
          <div class="absolute top-1/2 -translate-y-1/2 w-1.5 h-4 bg-accent shadow-[0_0_4px_rgba(245,130,32,0.5)] rounded-sm"
               :style="{ left: `calc(${progressPercent}% - 3px)` }"></div>
        </div>
        
        <span class="w-10">{{ formatTimeMs(playerStore.durationMs) }}</span>
      </div>

    </div>

    <!-- Right: Volume Knob & Output -->
    <div class="w-80 shrink-0 p-6 flex items-center justify-end gap-10">
      
      <!-- 3D Volume Knob -->
      <div class="flex items-center gap-4">
        <span class="text-[9px] font-bold tracking-widest text-text-muted">VOLUME</span>
        <div 
          class="w-14 h-14 rounded-full bg-gradient-to-br from-[#f8f6f2] to-[#dfdad0] dark:from-[#333] dark:to-[#1a1a1a] shadow-[4px_4px_10px_rgba(0,0,0,0.1),-2px_-2px_8px_rgba(255,255,255,0.8),inset_1px_1px_2px_rgba(255,255,255,0.9),inset_-1px_-1px_2px_rgba(0,0,0,0.05)] flex items-center justify-center relative cursor-pointer"
          ref="knobContainer"
          @mousedown="handleKnobMouseDown"
        >
          <!-- Knob inner dial -->
          <div class="w-10 h-10 rounded-full bg-gradient-to-tl from-[#f0ece5] to-[#e4ded3] dark:from-[#2a2a2a] dark:to-[#202020] shadow-[inset_2px_2px_4px_rgba(0,0,0,0.05),inset_-1px_-1px_3px_rgba(255,255,255,0.5)] relative"
               :style="{ transform: `rotate(${knobRotation}deg)` }">
            <!-- Indicator dot -->
            <div class="w-1.5 h-1.5 rounded-full bg-accent absolute top-1 left-1/2 -translate-x-1/2 shadow-[0_0_3px_rgba(245,130,32,0.5)]"></div>
          </div>
          
          <!-- Min/Max text -->
          <span class="absolute -bottom-2 -left-2 text-[8px] font-mono text-text-muted">0</span>
          <span class="absolute -bottom-2 -right-2 text-[8px] font-mono text-text-muted">100</span>
        </div>
      </div>

      <!-- Output -->
      <div class="flex flex-col gap-1 items-end pr-2">
        <span class="text-[9px] font-bold tracking-widest text-text-muted">OUTPUT</span>
        <div class="text-[11px] font-bold text-text-main flex items-center gap-1 cursor-pointer hover:opacity-70">
          Built-in Output <span class="text-[8px] ml-1">▼</span>
        </div>
      </div>

    </div>

  </footer>
</template>
