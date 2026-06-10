<script setup lang="ts">
import { onMounted, ref, onUnmounted, watch } from 'vue';
import { Heart, AudioLines, Plus } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();
const activeMenuTrackId = ref<number | null>(null);

const loadData = () => {
  const tab = playerStore.activeLibraryTab;
  if (tab === '全部歌曲') {
    playerStore.fetchTracks(true);
  } else if (tab === '收藏歌曲') {
    playerStore.fetchFavoriteTracks();
  } else if (tab === '最近播放') {
    playerStore.fetchRecentlyPlayed();
  } else {
    const pl = playerStore.playlists.find((p: any) => p.name === tab);
    if (pl) {
      playerStore.fetchPlaylistTracks(pl.id);
    }
  }
};

const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  if (target.scrollTop + target.clientHeight >= target.scrollHeight - 50) {
    if (playerStore.activeLibraryTab === '全部歌曲') {
      playerStore.fetchTracks();
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

watch(() => playerStore.activeLibraryTab, () => {
  loadData();
});

onMounted(() => {
  loadData();
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- Table Header -->
    <div class="flex items-center text-[10px] font-bold tracking-[0.15em] text-[#888] uppercase pb-4 mb-4 border-b border-black shrink-0 relative z-10">
      <div class="w-16 text-center">序号</div>
      <div class="flex-[2] pl-2">标题</div>
      <div class="flex-[1.5]">艺人</div>
      <div class="flex-[2]">专辑</div>
      <div class="w-20 text-right pr-4">时长</div>
      <div class="w-24 pl-4 text-left">格式</div>
    </div>

    <!-- Song List -->
    <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2" @scroll="handleScroll">
      <div 
        v-for="(song, index) in playerStore.tracks" 
        :key="song.id"
        @dblclick="playerStore.playQueue(playerStore.tracks, index)"
        class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/50 group transition-colors cursor-pointer hover:bg-black/5"
      >
        <div class="w-16 text-center text-[#888]">
          <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
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
            @click.stop="playerStore.toggleFavorite(song.id)"
          />
          <div class="relative flex items-center">
            <button @click.stop="openPlaylistMenu(song.id)" class="text-[#ccc] opacity-0 group-hover:opacity-100 hover:text-black transition-opacity" title="添加到歌单">
              <Plus class="w-3.5 h-3.5 stroke-[1.5]" />
            </button>
            <div v-if="activeMenuTrackId === song.id" class="absolute left-6 top-0 bg-white border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
              <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-[#a0a0a0] whitespace-nowrap">暂无自建歌单</div>
              <button 
                v-for="pl in playerStore.playlists" 
                :key="pl.id"
                @click.stop="addToPlaylist(pl.id, song.id)"
                class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-black hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
              >
                {{ pl.name }}
              </button>
            </div>
          </div>
          <span 
            class="truncate" 
            :class="playerStore.currentTrack?.id === song.id ? 'font-serif italic font-semibold text-[16px] text-black' : 'text-[#333] font-medium'"
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
      <span>{{ playerStore.tracks.length }} 首歌曲 / 98.6 GB / 3天14小时</span>
      <div class="flex items-center gap-4">
        <div class="w-12 h-px bg-[#dcdad1]"></div>
        <span>本地归档</span>
      </div>
    </div>
  </div>
</template>
