<script setup lang="ts">
import { 
  Shuffle, SkipBack, Play, Pause, SkipForward, Repeat, 
  ChevronDown
} from 'lucide-vue-next';
import { ref } from 'vue';

const isPlaying = ref(true);
</script>

<template>
  <div class="h-[110px] w-full bg-bg-canvas flex items-center px-6 flex-shrink-0 select-none">
    
    <!-- Left: Track Info & Waveform -->
    <div class="flex items-center w-[280px] flex-shrink-0">
      <div class="w-[56px] h-[56px] bg-gray-200 rounded-[6px] overflow-hidden flex-shrink-0 mr-3">
        <div class="w-full h-full bg-gradient-to-br from-gray-300 to-gray-400"></div>
      </div>
      
      <div class="flex flex-col justify-center min-w-0">
        <span class="text-[13px] font-semibold text-text-primary truncate leading-tight">Experience</span>
        <span class="text-[11px] text-text-muted truncate mt-0.5">Ludovico Einaudi · Divenire</span>
        <span class="text-[9px] text-text-muted font-mono mt-0.5 uppercase tracking-wider">FLAC · 24bit / 96kHz</span>
        
        <!-- Static Waveform UI -->
        <div class="flex items-end h-4 gap-[1px] mt-1.5">
          <div v-for="i in 40" :key="i" 
               class="w-[2px] bg-text-muted/30 rounded-t-sm"
               :style="{ height: `${Math.random() * 80 + 20}%` }">
          </div>
        </div>
      </div>
    </div>

    <!-- Center: Playback Controls -->
    <div class="flex-1 flex flex-col items-center justify-center px-8">
      
      <div class="flex items-center gap-8 mb-2">
        <button class="text-text-muted hover:text-text-primary transition-colors">
          <Shuffle class="w-[16px] h-[16px]" />
        </button>
        <button class="text-text-primary hover:text-brand-orange transition-colors">
          <SkipBack class="w-[18px] h-[18px] fill-current" />
        </button>
        <button 
          class="w-[48px] h-[48px] rounded-full bg-text-primary text-bg-canvas flex items-center justify-center hover:scale-105 transition-transform"
          @click="isPlaying = !isPlaying"
        >
          <Pause v-if="isPlaying" class="w-[20px] h-[20px] fill-current" />
          <Play v-else class="w-[20px] h-[20px] fill-current ml-0.5" />
        </button>
        <button class="text-text-primary hover:text-brand-orange transition-colors">
          <SkipForward class="w-[18px] h-[18px] fill-current" />
        </button>
        <button class="text-text-muted hover:text-text-primary transition-colors">
          <Repeat class="w-[16px] h-[16px]" />
        </button>
      </div>
      
      <div class="w-full flex items-center gap-3 max-w-md">
        <span class="text-[10px] font-mono text-text-muted w-8 text-right tabular-nums">01:34</span>
        <div class="flex-1 h-[3px] bg-border-solid rounded-full relative group cursor-pointer">
          <div class="absolute left-0 top-0 h-full bg-brand-orange rounded-full" style="width: 30%"></div>
          <div class="absolute top-1/2 -translate-y-1/2 w-[10px] h-[10px] bg-brand-orange rounded-full opacity-0 group-hover:opacity-100 transition-opacity" style="left: 30%; margin-left: -5px;"></div>
        </div>
        <span class="text-[10px] font-mono text-text-muted w-8 text-left tabular-nums">05:15</span>
      </div>

    </div>

    <!-- Right: Volume & Output -->
    <div class="flex items-center gap-10 flex-shrink-0">
      
      <!-- Volume Knob -->
      <div class="flex flex-col items-center">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5">Volume</span>
        <div class="relative w-[52px] h-[52px] rounded-full border-2 border-border-solid flex items-center justify-center bg-bg-canvas cursor-pointer hover:border-text-muted transition-colors">
          <div class="absolute top-[4px] left-1/2 -translate-x-1/2 w-[2px] h-[6px] bg-brand-orange rounded-full"></div>
        </div>
        <div class="flex justify-between w-14 mt-1 text-[8px] text-text-muted font-mono">
          <span>0</span>
          <span>100</span>
        </div>
      </div>
      
      <!-- Output Selector -->
      <div class="flex flex-col items-start">
        <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5">Output</span>
        <button class="flex items-center gap-1 text-[12px] font-medium text-text-primary hover:text-brand-orange transition-colors">
          Built-in Output
          <ChevronDown class="w-3 h-3 text-text-muted" />
        </button>
      </div>

    </div>

  </div>
</template>
