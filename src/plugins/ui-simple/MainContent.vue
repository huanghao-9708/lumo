<script setup lang="ts">
import { Search, Filter, List, LayoutGrid, Heart, AudioLines } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
</script>

<template>
  <main class="flex-1 flex flex-col h-full bg-transparent relative overflow-hidden px-12 py-10">
    <div data-tauri-drag-region class="absolute inset-0 z-0"></div>

    <!-- Header -->
    <header class="flex items-end justify-between shrink-0 mb-10 relative z-10">
      <div>
        <p class="text-[10px] font-bold tracking-[0.2em] text-[#888888] mb-2">INDEX — VOL. 01</p>
        <h1 class="font-serif italic text-5xl tracking-wide text-black">{{ playerStore.activeLibraryTab }}</h1>
      </div>

      <div class="flex items-center gap-8 mb-2 transition-all duration-300" :class="!playerStore.isRightPanelOpen ? 'mr-56' : ''">
        <div class="relative group">
          <Search class="w-4 h-4 text-[#888] absolute left-0 top-1/2 -translate-y-1/2" />
          <input 
            type="text" 
            placeholder="SEARCH..." 
            class="pl-8 pr-4 py-2 w-[240px] bg-transparent border-b border-[#dcdad1] focus:border-black text-xs tracking-widest focus:outline-none transition-colors placeholder-[#a0a0a0] uppercase"
          />
        </div>
        <div class="flex items-center gap-4 text-[#888]">
          <button class="hover:text-black transition-colors"><Filter class="w-4 h-4 stroke-[1.5]" /></button>
          <button class="hover:text-black transition-colors"><List class="w-4 h-4 stroke-[1.5]" /></button>
          <button class="hover:text-black transition-colors"><LayoutGrid class="w-4 h-4 stroke-[1.5]" /></button>
        </div>
      </div>
    </header>

    <!-- Table Header -->
    <div class="flex items-center text-[10px] font-bold tracking-[0.15em] text-[#888] uppercase pb-4 mb-4 border-b border-black shrink-0 relative z-10">
      <div class="w-16 text-center">NO.</div>
      <div class="flex-[2] pl-2">TITLE</div>
      <div class="flex-[1.5]">ARTIST</div>
      <div class="flex-[2]">ALBUM</div>
      <div class="w-20 text-right pr-4">TIME</div>
      <div class="w-24 pl-4 text-left">FORMAT</div>
    </div>

    <!-- Song List -->
    <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10">
      <div 
        v-for="(song, index) in playerStore.tracks" 
        :key="song.id"
        @click="playerStore.playTrack(song.id)"
        class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/50 group transition-colors cursor-pointer hover:bg-black/5"
      >
        <div class="w-16 text-center text-[#888]">
          <template v-if="playerStore.currentTrack.id === song.id && playerStore.isPlaying">
            <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-black animate-pulse" />
          </template>
          <template v-else>
            {{ String(index + 1).padStart(2, '0') }}
          </template>
        </div>
        <div class="flex-[2] pl-2 flex items-center gap-4">
          <Heart 
            class="w-3.5 h-3.5 transition-colors stroke-[1.5]" 
            :class="[
              song.isFavorite ? 'text-black fill-current' : 'text-[#ccc] opacity-0 group-hover:opacity-100 hover:text-black'
            ]"
            @click.stop="song.isFavorite = !song.isFavorite"
          />
          <span 
            class="truncate" 
            :class="playerStore.currentTrack.id === song.id ? 'font-serif italic font-semibold text-[16px] text-black' : 'text-[#333] font-medium'"
          >{{ song.title }}</span>
        </div>
        <div class="flex-[1.5] truncate pr-4 text-[#555]">{{ song.artist }}</div>
        <div class="flex-[2] truncate pr-4 text-[#777] italic">{{ song.album }}</div>
        <div class="w-20 text-right pr-4 text-[#888]">{{ song.duration }}</div>
        <div class="w-24 pl-4 text-left text-[11px] text-[#aaa] tracking-wider">{{ song.format }}</div>
      </div>
    </div>

    <!-- Footer Stats -->
    <div class="mt-4 pt-6 border-t border-[#e8e6df] text-[10px] font-bold tracking-[0.2em] text-[#888] shrink-0 relative z-10 flex items-center justify-between uppercase">
      <span>{{ playerStore.tracks.length }} TRACKS / 98.6 GB / 3D 14H</span>
      <div class="flex items-center gap-4">
        <div class="w-12 h-px bg-[#dcdad1]"></div>
        <span>ARCHIVE</span>
      </div>
    </div>
  </main>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: transparent;
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background-color: #dcdad1;
}
</style>
