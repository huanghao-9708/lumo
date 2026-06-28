<script setup lang="ts">
import { ref } from 'vue';
import { Search, Filter, Play, LayoutGrid, List, MoreHorizontal, Heart } from 'lucide-vue-next';

const songs = ref([
  { id: 1, title: 'Experience', artist: 'Ludovico Einaudi', album: 'Divenire', duration: '05:15', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 2, title: 'Nuvole Bianche', artist: 'Ludovico Einaudi', album: 'Una Mattina', duration: '07:48', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 3, title: 'Arrival of the Birds', artist: 'The Cinematic Orchestra', album: 'Ma Fleur', duration: '06:10', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 4, title: 'First Breath After Coma', artist: 'Explosions in the Sky', album: 'The Earth Is Not a Cold...', duration: '09:34', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 5, title: 'Elegy for Dunkirk', artist: 'Alexandre Desplat', album: 'Dunkirk (Original Motio...', duration: '06:25', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 6, title: 'Holocene', artist: 'Bon Iver', album: 'Bon Iver', duration: '05:36', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 7, title: 'Hoppipolla', artist: 'Sigur Ros', album: 'Takk...', duration: '04:28', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 8, title: 'Time', artist: 'Hans Zimmer', album: 'Inception (Music from...', duration: '04:35', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 9, title: "Comptine d'un autre ete", artist: 'Yann Tiersen', album: 'Amelie (Original Sound...', duration: '02:20', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 10, title: 'To Build a Home', artist: 'The Cinematic Orchestra', album: 'Ma Fleur', duration: '06:07', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 11, title: 'Your Hand in Mine', artist: 'Explosions in the Sky', album: 'The Earth Is Not a Cold...', duration: '08:17', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 12, title: 'Near Light', artist: 'Olafur Arnalds', album: '...and they have escap...', duration: '03:37', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 13, title: 'Says', artist: 'Nils Frahm', album: 'All Melody', duration: '05:41', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
  { id: 14, title: 'The Truth That You Leave', artist: 'Olafur Arnalds', album: 'Living Room Songs', duration: '05:29', format: 'FLAC', bitrate: '24bit / 96kHz', dateAdded: '2 days ago' },
]);
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none">
    
    <!-- Header -->
    <div class="px-8 pt-8 pb-0 flex-shrink-0" data-tauri-drag-region>
      <div class="flex items-start justify-between mb-2">
        <div>
          <h1 class="text-[28px] font-bold text-text-primary tracking-tight leading-tight mb-1">全部歌曲</h1>
          <p class="text-[12px] text-text-muted leading-relaxed">12,483 首歌曲 · 982 GB · 31 天连续播放时长</p>
        </div>
        
        <div class="flex items-center gap-3 mt-1">
          <button class="text-text-muted hover:text-text-primary text-[12px] transition-colors">视图</button>
          <button class="text-text-muted hover:text-text-primary transition-colors">
            <List class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-y-auto px-8">
      
      <!-- Table Header -->
      <div class="flex items-center text-[11px] text-text-muted uppercase tracking-wider py-3 border-b border-border-color sticky top-0 bg-bg-content z-10">
        <div class="w-10 text-center shrink-0">#</div>
        <div class="w-8 shrink-0"></div>
        <div class="flex-[2] min-w-0 pl-1">标题</div>
        <div class="flex-[1.5] min-w-0 hidden md:block">艺术家</div>
        <div class="flex-[1.5] min-w-0 hidden lg:block">专辑</div>
        <div class="w-[60px] text-right shrink-0 hidden xl:block">时长</div>
        <div class="w-[50px] text-center shrink-0 hidden xl:block">格式</div>
        <div class="w-[100px] text-right shrink-0 hidden xl:block">比特率</div>
        <div class="w-8 shrink-0"></div>
      </div>

      <!-- Table Rows -->
      <div>
        <div 
          v-for="(song, index) in songs" 
          :key="song.id"
          class="flex items-center py-[9px] hover:bg-list-hover transition-colors group cursor-pointer border-b border-border-color/40"
          :class="{ 'bg-list-selected': index === 0 }"
        >
          <div class="w-10 text-center shrink-0 text-[12px] font-mono text-text-muted">
            <span v-if="index === 0" class="text-brand-orange">
              <Play class="w-3 h-3 fill-current inline" />
            </span>
            <span v-else class="group-hover:hidden">{{ String(index + 1).padStart(2, '0') }}</span>
            <Play v-if="index !== 0" class="w-3 h-3 fill-current mx-auto hidden group-hover:block text-text-secondary" />
          </div>
          <!-- Heart column -->
          <div class="w-8 shrink-0 flex items-center justify-center">
            <Heart v-if="index === 0" class="w-[14px] h-[14px] text-brand-orange fill-current" />
            <Heart v-else class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer" />
          </div>
          <div class="flex-[2] min-w-0 pl-1">
            <span class="text-[13px] font-medium text-text-primary truncate block" :class="{ '!text-brand-orange font-semibold': index === 0 }">
              {{ song.title }}
            </span>
          </div>
          <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate">{{ song.artist }}</div>
          <div class="flex-[1.5] min-w-0 hidden lg:block text-[13px] text-text-secondary truncate italic">{{ song.album }}</div>
          <div class="w-[60px] text-right shrink-0 hidden xl:block text-[12px] font-mono text-text-muted">{{ song.duration }}</div>
          <div class="w-[50px] text-center shrink-0 hidden xl:block text-[11px] font-mono text-text-muted">{{ song.format }}</div>
          <div class="w-[100px] text-right shrink-0 hidden xl:block text-[11px] font-mono text-text-muted">{{ song.bitrate }}</div>
          <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <MoreHorizontal class="w-4 h-4 text-text-muted" />
          </div>
        </div>
      </div>

      <!-- Footer Status -->
      <div class="flex items-center justify-between py-4 border-t border-border-color mt-4 text-[11px] text-text-muted font-mono">
        <span>12,483 首歌曲  982 GB</span>
        <span>总时长: 31 天 7 小时 42 分钟</span>
      </div>

    </div>
  </div>
</template>
