<script setup lang="ts">
import { Heart, AudioLines, MoveLeft } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();

const goBack = () => {
  playerStore.activeArtistId = null;
  playerStore.activeLibraryTab = '艺人';
};

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentArtistDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 顶部返回 -->
      <div class="mb-8 shrink-0">
        <button 
          @click="goBack" 
          class="flex items-center gap-2 text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] hover:text-black transition-colors uppercase group"
        >
          <MoveLeft class="w-4 h-4 stroke-[1.5] group-hover:-translate-x-1 transition-transform" />
          返回艺人列表
        </button>
      </div>

      <!-- 艺人头 -->
      <div class="flex flex-col mb-12 shrink-0">
        <h2 class="text-[10px] font-bold tracking-[0.2em] text-[#888888] mb-4 uppercase">艺人</h2>
        <h1 class="font-serif italic text-6xl tracking-wide text-black mb-6">{{ playerStore.currentArtistDetails.name }}</h1>
        <div class="flex items-center gap-6">
          <div class="w-12 h-12 rounded-full overflow-hidden bg-[#e8e6df] shrink-0">
             <div 
               class="w-full h-full bg-gradient-to-tr opacity-90"
               :class="playerStore.currentArtistDetails.avatarColor"
             ></div>
          </div>
          <div class="text-[10px] font-bold tracking-[0.2em] text-[#888] uppercase">
            {{ playerStore.currentArtistDetails.tracks.length }} 首歌曲 <span class="mx-2 text-[#dcdad1]">|</span> {{ playerStore.currentArtistDetails.albums.length }} 张专辑
          </div>
        </div>
      </div>

      <!-- 下部分：滚动区域 (热门歌曲 + 专辑) -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-10 space-y-16">
        
        <!-- 歌曲列表 -->
        <section v-if="playerStore.currentArtistDetails.tracks.length > 0">
          <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-4 uppercase border-b border-[#e8e6df] pb-2">热门曲目</h3>
          <div class="flex flex-col">
            <div 
              v-for="(song, index) in playerStore.currentArtistDetails.tracks" 
              :key="song.id"
              @click="playerStore.playTrack(song.id)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/50 group transition-colors cursor-pointer hover:bg-black/5"
            >
              <div class="w-12 text-left text-[#888]">
                <template v-if="playerStore.currentTrack.id === song.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 stroke-[1.5] text-black animate-pulse" />
                </template>
                <template v-else>
                  {{ String(index + 1).padStart(2, '0') }}
                </template>
              </div>
              <div class="flex-[3] flex items-center gap-4">
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
              <div class="flex-[2] truncate pr-4 text-[#777] italic">{{ song.album }}</div>
              <div class="w-16 text-right pr-4 text-[#888]">{{ song.duration }}</div>
            </div>
          </div>
        </section>

        <!-- 专辑墙 -->
        <section v-if="playerStore.currentArtistDetails.albums.length > 0">
          <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-6 uppercase border-b border-[#e8e6df] pb-2">所有专辑</h3>
          <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8">
            <div 
              v-for="album in playerStore.currentArtistDetails.albums" 
              :key="album.id"
              @click="goToAlbum(album.id)"
              class="group cursor-pointer flex flex-col"
            >
              <div class="relative aspect-square w-full mb-4 overflow-hidden bg-[#e8e6df] shadow-sm">
                <div 
                  class="absolute inset-0 bg-gradient-to-br opacity-80 group-hover:scale-105 transition-transform duration-700 ease-out"
                  :class="album.coverColor"
                ></div>
                <div class="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors duration-300"></div>
              </div>
              <div class="flex flex-col gap-1">
                <h4 class="font-serif italic font-semibold text-base text-black truncate">{{ album.title }}</h4>
                <span class="text-[10px] tracking-widest text-[#a0a0a0]">{{ album.year }}</span>
              </div>
            </div>
          </div>
        </section>

      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-[#888]">
      未找到艺人信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
