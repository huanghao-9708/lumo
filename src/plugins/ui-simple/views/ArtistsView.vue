<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
const playerStore = usePlayerStore();

const goToArtist = (artistId: number) => {
  playerStore.activeArtistId = artistId;
  playerStore.activeLibraryTab = '艺人详情';
};
</script>

<template>
  <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2">
    <div class="flex flex-col pb-10">
      <div 
        v-for="(artist, index) in playerStore.artists" 
        :key="artist.id"
        @click="goToArtist(artist.id)"
        class="group cursor-pointer flex items-center justify-between py-5 border-b border-[#f0eee6]/50 hover:border-black transition-colors"
      >
        <div class="flex items-center gap-8">
          <span class="text-[10px] font-bold tracking-widest text-[#a0a0a0] w-6 text-right">
            {{ String(index + 1).padStart(2, '0') }}
          </span>
          <div class="w-16 h-16 rounded-full overflow-hidden bg-[#e8e6df] shrink-0">
             <div 
               class="w-full h-full bg-gradient-to-tr opacity-70 group-hover:opacity-100 transition-opacity"
               :class="artist.avatarColor"
             ></div>
          </div>
          <h3 class="font-serif italic text-3xl text-black group-hover:translate-x-2 transition-transform duration-300">{{ artist.name }}</h3>
        </div>
        <div class="text-[10px] font-bold tracking-[0.2em] text-[#888] uppercase">
          {{ artist.trackCount }} 首歌曲
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
