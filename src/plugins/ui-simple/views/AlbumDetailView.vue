<script setup lang="ts">
import { Heart, AudioLines, MoveLeft } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();

const goBack = () => {
  playerStore.activeAlbumId = null;
  playerStore.activeLibraryTab = '专辑';
};
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentAlbumDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 顶部返回 -->
      <div class="mb-8 shrink-0">
        <button 
          @click="goBack" 
          class="flex items-center gap-2 text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] hover:text-black transition-colors uppercase group"
        >
          <MoveLeft class="w-4 h-4 stroke-[1.5] group-hover:-translate-x-1 transition-transform" />
          返回专辑列表
        </button>
      </div>

      <!-- 专辑信息头 -->
      <div class="flex items-end gap-10 mb-12 shrink-0">
        <div class="w-48 h-48 relative overflow-hidden bg-[#e8e6df] shadow-sm shrink-0">
          <div 
            class="absolute inset-0 bg-gradient-to-br opacity-80"
            :class="playerStore.currentAlbumDetails.coverColor"
          ></div>
        </div>
        <div class="flex flex-col pb-2">
          <h2 class="text-[10px] font-bold tracking-[0.2em] text-[#888888] mb-4 uppercase">专辑</h2>
          <h1 class="font-serif italic text-5xl tracking-wide text-black mb-4">{{ playerStore.currentAlbumDetails.title }}</h1>
          <p class="text-sm font-medium text-[#555] tracking-widest uppercase">
            <span class="text-black font-bold">{{ playerStore.currentAlbumDetails.artist }}</span> 
            <span class="mx-3 text-[#dcdad1]">|</span> 
            {{ playerStore.currentAlbumDetails.year }} 
            <span class="mx-3 text-[#dcdad1]">|</span> 
            {{ playerStore.currentAlbumDetails.tracks.length }} 首歌曲
          </p>
        </div>
      </div>

      <!-- 歌曲列表 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-10">
        <div 
          v-for="(song, index) in playerStore.currentAlbumDetails.tracks" 
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
          <div class="flex-[3] pl-2 flex items-center gap-4">
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
          <div class="w-20 text-right pr-4 text-[#888]">{{ song.duration }}</div>
          <div class="w-24 pl-4 text-left text-[11px] text-[#aaa] tracking-wider">{{ song.format }}</div>
        </div>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-[#888]">
      未找到专辑信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
