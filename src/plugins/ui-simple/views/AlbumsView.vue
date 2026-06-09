<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
const playerStore = usePlayerStore();

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};
</script>

<template>
  <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2">
    <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8 pb-10 pt-2">
      <div 
        v-for="album in playerStore.albums" 
        :key="album.id"
        @click="goToAlbum(album.id)"
        class="group cursor-pointer flex flex-col"
      >
        <!-- 专辑封面 -->
        <div class="relative aspect-square w-full mb-4 overflow-hidden bg-[#e8e6df] shadow-sm">
          <div 
            class="absolute inset-0 bg-gradient-to-br opacity-80 group-hover:scale-105 transition-transform duration-700 ease-out"
            :class="album.coverColor"
          ></div>
          <div class="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors duration-300"></div>
        </div>
        
        <!-- 专辑信息 -->
        <div class="flex flex-col gap-1">
          <h3 class="font-serif italic font-semibold text-lg text-black truncate">{{ album.title }}</h3>
          <div class="flex items-center justify-between">
            <p class="text-xs font-medium text-[#777] truncate">{{ album.artist }}</p>
            <span class="text-[10px] tracking-widest text-[#a0a0a0]">{{ album.year }}</span>
          </div>
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
