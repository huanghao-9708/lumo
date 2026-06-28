<script setup lang="ts">
import { Heart, MoreHorizontal } from 'lucide-vue-next';
import { useUiStore } from '../../stores/ui';

const uiStore = useUiStore();
</script>

<template>
  <div v-if="uiStore.isRightSidebarVisible" class="hidden xl:flex w-[360px] h-full bg-bg-canvas flex-col flex-shrink-0 select-none">
    
    <!-- Top Tabs - underline overlays the border -->
    <div class="relative h-[60px] flex-shrink-0" data-tauri-drag-region>
      <div class="absolute bottom-0 left-0 w-full h-px bg-border-color"></div>
      <div class="flex items-end justify-center gap-10 h-full">
        <button class="relative pb-3 text-[13px] font-medium text-brand-orange transition-smooth">
          正在播放
          <div class="absolute bottom-[-1px] left-0 w-full h-[2px] bg-brand-orange z-10"></div>
        </button>
        <button class="relative pb-3 text-[13px] font-medium text-text-muted hover:text-text-primary transition-smooth">
          播放列表
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-6 pt-4 pb-4 flex flex-col min-h-0">
      <!-- Album Art - compact -->
      <div class="w-full aspect-square bg-gray-200 rounded-[10px] mb-4 overflow-hidden flex-shrink-0">
        <div class="w-full h-full bg-gradient-to-br from-gray-300 to-gray-400"></div>
      </div>

      <!-- Track Info - tighter -->
      <div class="mb-4 flex-shrink-0">
        <div class="flex items-center justify-between mb-0.5">
          <h2 class="text-[18px] font-bold text-text-primary truncate pr-4 leading-tight">Experience</h2>
          <div class="flex items-center gap-2 flex-shrink-0">
            <button class="text-brand-orange hover:scale-110 transition-transform">
              <Heart class="w-[18px] h-[18px] fill-current" />
            </button>
            <button class="text-text-muted hover:text-text-primary transition-colors">
              <MoreHorizontal class="w-[18px] h-[18px]" />
            </button>
          </div>
        </div>
        
        <p class="text-[14px] text-text-primary mb-0">Ludovico Einaudi</p>
        <p class="text-[13px] text-text-muted mb-1.5">Divenire</p>
        <p class="text-[10px] text-text-muted font-mono uppercase tracking-wider leading-relaxed">
          2015 · 1 TRACK · 5:15 · FLAC · 24bit / 96kHz
        </p>
      </div>

      <!-- Lyrics - takes remaining space -->
      <div class="flex-1 overflow-y-auto min-h-0">
        <h3 class="text-[10px] font-semibold text-text-muted uppercase tracking-widest mb-3">Lyrics</h3>
        
        <div class="space-y-3">
          <p class="text-[13px] text-text-secondary leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            We are going on a journey
          </p>
          <p class="text-[13px] text-brand-orange font-medium leading-[1.8] transition-smooth cursor-pointer">
            A journey to experience
          </p>
          <p class="text-[13px] text-text-secondary leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            We are going on a journey
          </p>
          <p class="text-[13px] text-text-muted leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            A journey to experience
          </p>
          <p class="text-[13px] text-text-muted leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            We are going on a journey
          </p>
          <p class="text-[13px] text-text-muted leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            We are going on a journey
          </p>
          <p class="text-[13px] text-text-muted leading-[1.8] transition-smooth cursor-pointer hover:text-text-primary">
            We are going on a journey
          </p>
        </div>
      </div>

    </div>
  </div>
</template>
