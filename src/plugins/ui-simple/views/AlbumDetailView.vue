<script setup lang="ts">
import { Heart, AudioLines, MoveLeft, Play } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import { getArtworkUrl } from '../../../utils';

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
      <div class="flex flex-col md:flex-row md:items-end gap-12 mb-16 shrink-0 relative">
        <div class="w-56 h-56 relative bg-[#f5f4f0] shadow-sm shrink-0 group">
          <img 
            v-if="playerStore.currentAlbumDetails.cover_artwork_id"
            :src="getArtworkUrl(playerStore.currentAlbumDetails.cover_artwork_id)"
            class="absolute inset-0 w-full h-full object-cover"
          />
          <div 
            v-else
            class="absolute inset-0 bg-[#e8e6df]"
          ></div>
          
          <!-- 悬浮播放按钮 -->
          <div class="absolute right-4 bottom-4 opacity-0 group-hover:opacity-100 transition-opacity duration-300">
            <button 
              @click="playerStore.playTrack(playerStore.currentAlbumDetails.tracks[0]?.id)"
              class="w-12 h-12 bg-black text-white rounded-full flex items-center justify-center hover:scale-105 transition-transform shadow-md"
            >
              <Play class="w-5 h-5 ml-1 fill-current" />
            </button>
          </div>
        </div>
        <div class="flex flex-col pb-2">
          <h2 class="text-[10px] font-bold tracking-[0.3em] text-[#a0a0a0] mb-6 uppercase">Album</h2>
          <h1 class="font-serif italic text-6xl tracking-wide text-black mb-6 leading-tight">{{ playerStore.currentAlbumDetails.title }}</h1>
          <p class="text-[12px] font-medium text-[#555] tracking-[0.1em] uppercase">
            <span class="text-black font-bold tracking-widest">{{ playerStore.currentAlbumDetails.artist }}</span> 
            <span class="mx-4 text-[#dcdad1]">/</span> 
            {{ playerStore.currentAlbumDetails.year }} 
            <span class="mx-4 text-[#dcdad1]">/</span> 
            {{ playerStore.currentAlbumDetails.tracks.length }} Tracks
          </p>
        </div>
      </div>

      <!-- 歌曲列表 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-10">
        <div 
          v-for="(song, index) in playerStore.currentAlbumDetails.tracks" 
          :key="song.id"
          @click="playerStore.playTrack(song.id)"
          class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/60 group transition-colors duration-200 cursor-pointer hover:bg-[#faf9f5]"
        >
          <div class="w-16 text-center text-[#a0a0a0] font-medium relative">
            <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
              <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-black animate-pulse" />
            </template>
            <template v-else>
              <span class="group-hover:opacity-0 transition-opacity duration-200">{{ String(index + 1).padStart(2, '0') }}</span>
              <Play class="w-3.5 h-3.5 absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-black fill-current" />
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
              :class="playerStore.currentTrack?.id === song.id ? 'font-serif italic font-semibold text-[16px] text-black' : 'text-[#333] font-medium'"
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
