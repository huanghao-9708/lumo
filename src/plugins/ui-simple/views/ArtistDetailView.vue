<script setup lang="ts">
import { ref } from 'vue';
import { Heart, AudioLines, MoveLeft, Play } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import { getArtworkUrl } from '../../../utils';

const playerStore = usePlayerStore();

const goBack = () => {
  playerStore.activeArtistId = null;
  playerStore.activeLibraryTab = '艺人';
};

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

const activeTab = ref<'tracks' | 'albums'>('tracks');

const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 200) {
    if (activeTab.value === 'tracks') {
      playerStore.fetchArtistTracks(playerStore.currentArtistDetails.id, true);
    } else {
      playerStore.fetchArtistAlbums(playerStore.currentArtistDetails.id, true);
    }
  }
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

      <!-- 艺人头 (巨型 Typography) -->
      <div class="flex flex-col mb-20 shrink-0 relative">
        <h2 class="text-[10px] font-bold tracking-[0.3em] text-[#a0a0a0] mb-8 uppercase z-10">Artist</h2>
        
        <div class="relative z-10">
          <h1 class="font-serif italic text-[100px] leading-[0.85] tracking-tight text-black mb-10 break-words mix-blend-multiply">
            {{ playerStore.currentArtistDetails.name }}
          </h1>
        </div>

        <div class="flex items-center gap-6 mt-4 z-10 border-t border-black pt-6 max-w-md">
          <div class="flex items-center gap-8">
            <button 
              @click="activeTab = 'tracks'"
              class="text-[12px] tracking-[0.1em] uppercase transition-all duration-300"
              :class="activeTab === 'tracks' ? 'text-black font-bold border-b-2 border-black pb-1' : 'text-[#888] font-medium hover:text-black'"
            >
              TRACKS 
              <span class="text-[10px] ml-1">{{ playerStore.currentArtistDetails.stats?.track_count || 0 }}</span>
            </button>
            
            <button 
              @click="activeTab = 'albums'"
              class="text-[12px] tracking-[0.1em] uppercase transition-all duration-300"
              :class="activeTab === 'albums' ? 'text-black font-bold border-b-2 border-black pb-1' : 'text-[#888] font-medium hover:text-black'"
            >
              ALBUMS 
              <span class="text-[10px] ml-1">{{ playerStore.currentArtistDetails.stats?.album_count || 0 }}</span>
            </button>
          </div>
        </div>

        <!-- 装饰性背景文字，错位放大 -->
        <div class="absolute -top-10 -right-10 pointer-events-none select-none overflow-hidden w-full h-full flex justify-end opacity-[0.03]">
          <h1 class="font-serif italic text-[200px] leading-none whitespace-nowrap">
            {{ playerStore.currentArtistDetails.name }}
          </h1>
        </div>
      </div>

      <!-- 下部分：滚动区域 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-10" @scroll="handleScroll">
        
        <!-- 歌曲列表 -->
        <section v-if="activeTab === 'tracks'">
          <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-4 uppercase border-b border-[#e8e6df] pb-2">热门曲目</h3>
          <div class="flex flex-col">
            <div 
              v-for="(song, index) in playerStore.currentArtistDetails.tracks" 
              :key="song.id"
              @click="playerStore.playTrack(song.id)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/60 group transition-colors duration-200 cursor-pointer hover:bg-[#faf9f5]"
            >
              <div class="w-12 text-left text-[#a0a0a0] font-medium relative">
                <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 stroke-[1.5] text-black animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity duration-200">{{ String(index + 1).padStart(2, '0') }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-0 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-black fill-current" />
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
                  :class="playerStore.currentTrack?.id === song.id ? 'font-serif italic font-semibold text-[16px] text-black' : 'text-[#333] font-medium'"
                >{{ song.title }}</span>
              </div>
              <div class="flex-[2] truncate pr-4 text-[#777] italic">{{ song.album }}</div>
              <div class="w-16 text-right pr-4 text-[#888]">{{ song.duration }}</div>
            </div>
          </div>
        </section>

        <!-- 专辑墙 -->
        <section v-if="activeTab === 'albums'">
          <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-6 uppercase border-b border-[#e8e6df] pb-2">所有专辑</h3>
          <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-8">
            <div 
              v-for="album in playerStore.currentArtistDetails.albums" 
              :key="album.id"
              @click="goToAlbum(album.id)"
              class="group cursor-pointer flex flex-col"
            >
              <div class="relative aspect-square w-full mb-4 overflow-hidden bg-[#e8e6df] shadow-sm">
                <img 
                  v-if="album.cover_artwork_id"
                  :src="getArtworkUrl(album.cover_artwork_id)"
                  class="absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition-transform duration-700 ease-out"
                />
                <div 
                  v-else
                  class="absolute inset-0 bg-[#e8e6df] group-hover:scale-105 transition-transform duration-700 ease-out"
                ></div>
                <div class="absolute inset-0 bg-black/0 group-hover:bg-black/5 transition-colors duration-300"></div>
              </div>
              <div class="flex flex-col gap-1">
                <h4 class="font-serif italic font-semibold text-base text-black truncate">{{ album.title }}</h4>
                <span class="text-[10px] tracking-widest text-[#a0a0a0]">{{ album.year }}</span>
              </div>
            </div>
          </div>
          
          <div v-if="playerStore.currentArtistDetails.isLoadingAlbums" class="text-center text-[#888] py-8 text-xs tracking-widest uppercase">
            Loading...
          </div>
        </section>

        <!-- 加载动画给Tracks -->
        <div v-if="activeTab === 'tracks' && playerStore.currentArtistDetails.isLoadingTracks" class="text-center text-[#888] py-8 text-xs tracking-widest uppercase">
          Loading...
        </div>
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
