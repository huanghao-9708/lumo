<script setup lang="ts">
import { Search, Filter, List, LayoutGrid, Square, Heart, MoreVertical, AudioLines } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <main class="flex-1 flex flex-col h-full bg-white relative overflow-hidden">
    <!-- Top Header -->
    <header class="h-[88px] flex items-center justify-between px-8 shrink-0 select-none relative">
      <div data-tauri-drag-region class="absolute inset-0 z-0"></div>
      
      <div class="flex items-baseline gap-4 relative z-10 pointer-events-none">
        <h1 class="text-2xl font-bold text-gray-800">{{ playerStore.activeLibraryTab }}</h1>
        <span class="text-sm text-gray-400">{{ playerStore.tracks.length }} 首歌曲</span>
      </div>
      <div class="relative group z-10 transition-all duration-300" :class="!playerStore.isRightPanelOpen ? 'mr-56' : ''">
        <Search class="w-4 h-4 text-gray-400 absolute left-3 top-1/2 -translate-y-1/2 group-focus-within:text-brand-orange transition-smooth" />
        <input 
          type="text" 
          placeholder="搜索歌曲、专辑、艺人" 
          class="pl-9 pr-4 py-2 w-[280px] bg-gray-50 border border-gray-200 rounded-full text-sm focus:outline-none focus:ring-2 focus:ring-brand-orange-light/30 focus:bg-white transition-smooth placeholder-gray-400"
        />
      </div>
    </header>

    <!-- Toolbar -->
    <div class="px-8 pb-4 flex items-center justify-between shrink-0">
      <div class="flex-1"></div>
      <!-- View Options -->
      <div class="flex items-center gap-2">
        <button class="w-8 h-8 flex items-center justify-center rounded-lg text-gray-400 hover:text-gray-700 hover:bg-gray-100 transition-smooth">
          <Filter class="w-4 h-4" />
        </button>
        <div class="h-4 w-px bg-gray-200 mx-1"></div>
        <div class="flex items-center bg-gray-50 rounded-lg p-0.5 border border-gray-100">
          <button class="w-7 h-7 flex items-center justify-center rounded-md bg-white text-brand-orange shadow-sm transition-smooth">
            <List class="w-4 h-4" />
          </button>
          <button class="w-7 h-7 flex items-center justify-center rounded-md text-gray-400 hover:text-gray-700 transition-smooth">
            <LayoutGrid class="w-4 h-4" />
          </button>
        </div>
        <button class="w-8 h-8 flex items-center justify-center rounded-lg text-gray-400 hover:text-gray-700 hover:bg-gray-100 transition-smooth ml-1">
          <Square class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- Table Header -->
    <div class="px-8 flex items-center text-xs text-gray-400 pb-3 border-b border-gray-100 shrink-0">
      <div class="w-12 text-center">#</div>
      <div class="flex-[2] pl-2">标题</div>
      <div class="flex-[1.5]">艺人</div>
      <div class="flex-[2]">专辑</div>
      <div class="w-20 text-right pr-4">时长</div>
      <div class="w-24 pl-4 text-left">文件格式</div>
      <div class="w-8"></div>
    </div>

    <!-- Song List -->
    <div class="flex-1 overflow-y-auto px-4 py-2 custom-scrollbar">
      <div 
        v-for="(song, index) in playerStore.tracks" 
        :key="song.id"
        @click="playerStore.playTrack(song.id)"
        class="flex items-center text-sm py-2 px-4 rounded-xl group transition-smooth cursor-pointer"
        :class="playerStore.currentTrack.id === song.id ? 'bg-orange-50/40 text-brand-orange shadow-[inset_3px_0_0_0_#f58220]' : 'text-gray-600 hover:bg-gray-50 hover:shadow-sm'"
      >
        <div class="w-12 text-center font-medium" :class="playerStore.currentTrack.id === song.id ? 'text-brand-orange' : 'text-gray-400'">
          <template v-if="playerStore.currentTrack.id === song.id && playerStore.isPlaying">
            <AudioLines class="w-4 h-4 mx-auto animate-pulse" />
          </template>
          <template v-else>
            {{ index + 1 }}
          </template>
        </div>
        <div class="flex-[2] pl-2 flex items-center gap-3">
          <Heart 
            class="w-4 h-4 transition-smooth" 
            :class="[
              song.isFavorite 
                ? 'text-brand-orange-light opacity-100' 
                : 'text-gray-300 opacity-0 group-hover:opacity-100 hover:text-gray-500',
              playerStore.currentTrack.id === song.id && song.isFavorite ? 'text-brand-orange' : ''
            ]" 
          />
          <span class="truncate font-medium" :class="playerStore.currentTrack.id === song.id ? 'text-brand-orange' : 'text-gray-800'">{{ song.title }}</span>
        </div>
        <div class="flex-[1.5] truncate pr-4 text-[13px]">{{ song.artist }}</div>
        <div class="flex-[2] truncate pr-4 text-[13px]">{{ song.album }}</div>
        <div class="w-20 text-right pr-4 text-[13px] font-mono">{{ song.duration }}</div>
        <div class="w-24 pl-4 text-left text-[12px] text-gray-400 font-medium tracking-wider">{{ song.format }}</div>
        <div class="w-8 flex justify-end opacity-0 group-hover:opacity-100 transition-smooth">
          <button class="p-1 rounded-md hover:bg-gray-200 text-gray-500 hover:text-gray-800" @click.stop="">
            <MoreVertical class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Footer Stats -->
    <div class="px-8 py-4 border-t border-gray-100 text-[13px] text-gray-400 shrink-0 bg-white/80 backdrop-blur-sm z-10">
      {{ playerStore.tracks.length }} 首歌曲, 98.6 GB, 时长 3 天 14 小时
    </div>
  </main>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: transparent;
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background-color: #e5e7eb;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: #d1d5db;
}
</style>
