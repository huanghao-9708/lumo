<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { Heart, AudioLines, Play, Plus, Trash2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();

const activeMenuTrackId = ref<number | null>(null);

const removeTrack = async (trackId: number) => {
  if (playerStore.currentPlaylistDetails) {
    try {
      await playerStore.removeTrackFromPlaylist(playerStore.currentPlaylistDetails.id, trackId);
    } catch (e) {
      alert("移除歌曲失败");
    }
  }
};

const openPlaylistMenu = (trackId: number) => {
  if (activeMenuTrackId.value === trackId) {
    activeMenuTrackId.value = null;
  } else {
    activeMenuTrackId.value = trackId;
  }
};

const addToPlaylist = (playlistId: number, trackId: number) => {
  playerStore.addToPlaylist(playlistId, trackId);
  activeMenuTrackId.value = null;
};

const closeMenu = () => {
  activeMenuTrackId.value = null;
};

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});

const playAll = () => {
  if (playerStore.currentPlaylistDetails?.tracks && playerStore.currentPlaylistDetails.tracks.length > 0) {
    playerStore.playQueue(playerStore.currentPlaylistDetails.tracks, 0);
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentPlaylistDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 歌单头 (巨型 Typography) -->
      <div class="flex flex-col mb-16 shrink-0 relative">
        <h2 class="text-[10px] font-bold tracking-[0.3em] text-text-muted mb-8 uppercase z-10">Playlist</h2>
        
        <div class="relative z-10">
          <h1 class="font-serif italic text-[80px] leading-[0.85] tracking-tight text-accent mb-6 break-words mix-blend-multiply">
            {{ playerStore.currentPlaylistDetails.name }}
          </h1>
          <p v-if="playerStore.currentPlaylistDetails.description" class="max-w-md text-[#555] text-sm leading-relaxed mt-4">
            {{ playerStore.currentPlaylistDetails.description }}
          </p>
        </div>

        <div class="flex items-center gap-6 mt-8 z-10 border-t border-black pt-6 max-w-md">
          <div class="text-[12px] font-medium tracking-[0.1em] text-[#555] uppercase">
            <span class="text-accent font-bold tracking-widest">{{ playerStore.currentPlaylistDetails.count }}</span> Tracks 
          </div>
          
          <button 
            v-if="playerStore.currentPlaylistDetails.tracks.length > 0"
            @click="playAll" 
            class="flex items-center gap-2 bg-black text-white px-5 py-2.5 hover:bg-black/80 transition-all ml-auto group rounded-sm shadow-md"
          >
            <Play class="w-3.5 h-3.5 fill-current" />
            <span class="text-[10px] font-bold tracking-[0.2em] uppercase">Play All</span>
          </button>
        </div>

        <!-- 装饰性背景文字，错位放大 -->
        <div class="absolute -top-10 -right-10 pointer-events-none select-none overflow-hidden w-full h-full flex justify-end opacity-[0.03]">
          <h1 class="font-serif italic text-[200px] leading-none whitespace-nowrap">
            {{ playerStore.currentPlaylistDetails.name }}
          </h1>
        </div>
      </div>

      <!-- 滚动区域 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-10">
        <section v-if="playerStore.currentPlaylistDetails.tracks.length > 0">
          <div class="flex flex-col">
            <div 
              v-for="(song, index) in playerStore.currentPlaylistDetails.tracks" 
              :key="song.id"
              @dblclick="playerStore.playQueue(playerStore.currentPlaylistDetails.tracks, index)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/60 group transition-colors duration-200 cursor-pointer hover:bg-[#faf9f5]"
            >
              <div class="w-12 text-left text-text-muted font-medium relative">
                <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 stroke-[1.5] text-accent animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity duration-200">{{ String(index + 1).padStart(2, '0') }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-0 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-accent fill-current" />
                </template>
              </div>
              <div class="flex-[3] flex items-center gap-4">
                <Heart 
                  class="w-3.5 h-3.5 transition-colors stroke-[1.5]" 
                  :class="[
                    song.isFavorite ? 'text-accent fill-current' : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent '
                  ]"
                  @click.stop="playerStore.toggleFavorite(song.id)"
                />
                <div class="relative flex items-center">
                  <button @click.stop="openPlaylistMenu(song.id)" class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent  transition-opacity" title="添加到歌单">
                    <Plus class="w-3.5 h-3.5 stroke-[1.5]" />
                  </button>
                  <div v-if="activeMenuTrackId === song.id" class="absolute left-6 top-0 bg-bg-base border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                    <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-text-muted whitespace-nowrap">暂无自建歌单</div>
                    <button 
                      v-for="pl in playerStore.playlists" 
                      :key="pl.id"
                      @click.stop="addToPlaylist(pl.id, song.id)"
                      class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-accent  hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
                    >
                      {{ pl.name }}
                    </button>
                  </div>
                </div>
                <span 
                  class="truncate" 
                  :class="playerStore.currentTrack?.id === song.id ? 'font-serif italic font-semibold text-[16px] text-accent' : 'text-text-main  font-medium'"
                >{{ song.title }}</span>
              </div>
              <div class="flex-[2] truncate pr-4 text-text-muted italic">{{ song.artist }}</div>
              <div class="flex-[2] truncate pr-4 text-text-muted">{{ song.album }}</div>
              <div class="w-24 text-right pr-4 flex items-center justify-end gap-3 shrink-0">
                <span class="text-text-muted ">{{ song.duration }}</span>
                <button 
                  @click.stop="removeTrack(song.id)"
                  class="text-text-muted hover:text-[#d25050] transition-colors opacity-0 group-hover:opacity-100"
                  title="从歌单中移除"
                >
                  <Trash2 class="w-3.5 h-3.5 stroke-[1.5]" />
                </button>
              </div>
            </div>
          </div>
        </section>
        
        <div v-else-if="!playerStore.currentPlaylistDetails.isLoadingTracks" class="text-center text-text-muted  py-20 text-xs tracking-widest uppercase">
          该歌单目前为空，快去添加歌曲吧
        </div>
        
        <div v-if="playerStore.currentPlaylistDetails.isLoadingTracks" class="text-center text-text-muted  py-8 text-xs tracking-widest uppercase">
          Loading...
        </div>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-text-muted ">
      加载中或未找到歌单信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
