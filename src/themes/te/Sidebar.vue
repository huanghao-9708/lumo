<script setup lang="ts">
import { 
  Settings, AudioLines, Disc3, Mic2, Heart, Clock, ListMusic,
  FolderOpen, Server, HardDrive
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import ThemeControls from '../../components/ThemeControls.vue';

const playerStore = usePlayerStore();

// Icons mapped strictly
const libraryItems = [
  { label: 'ALL TRACKS', id: '全部歌曲', icon: AudioLines },
  { label: 'ALBUMS', id: '专辑', icon: Disc3 },
  { label: 'ARTISTS', id: '艺人', icon: Mic2 },
  { label: 'FAVORITES', id: '收藏歌曲', icon: Heart },
  { label: 'RECENTLY PLAYED', id: '最近播放', icon: Clock },
  { label: 'PLAYLISTS', id: '播放队列', icon: ListMusic },
];
</script>

<template>
  <aside class="w-full h-full flex flex-col font-mono text-xs uppercase tracking-widest bg-bg-base">
    
    <!-- Header / Logo -->
    <div class="h-20 shrink-0 flex items-center px-6 border-b border-border-color">
      <h1 class="text-xl font-bold tracking-[0.3em] text-text-main">LUMO</h1>
    </div>

    <div class="flex-1 overflow-y-auto pt-6 pb-20 custom-scrollbar">
      
      <!-- LIBRARY -->
      <div class="mb-8">
        <h2 class="px-6 mb-4 text-[10px] text-text-muted font-bold tracking-widest">LIBRARY</h2>
        <ul class="flex flex-col">
          <li v-for="item in libraryItems" :key="item.id">
            <a 
              href="#" 
              @click.prevent="playerStore.activeLibraryTab = item.id as any"
              class="flex items-center px-6 py-2 transition-colors cursor-pointer"
              :class="playerStore.activeLibraryTab === item.id 
                ? 'bg-accent text-bg-base font-bold' 
                : 'text-text-main hover:bg-bg-panel'"
            >
              <component :is="item.icon" class="w-4 h-4 mr-4 stroke-[1.5]" />
              <span class="flex-1">{{ item.label }}</span>
              <!-- Counts can be placed here if available -->
            </a>
          </li>
        </ul>
      </div>

      <!-- PLAYLISTS -->
      <div class="mb-8">
        <h2 class="px-6 mb-4 text-[10px] text-text-muted font-bold tracking-widest">PLAYLISTS</h2>
        <ul class="flex flex-col">
          <li v-for="playlist in playerStore.playlists" :key="playlist.id">
            <a 
              href="#" 
              @click.prevent="playerStore.activeLibraryTab = '歌单详情'; playerStore.activePlaylistId = playlist.id"
              class="flex items-center px-6 py-2 transition-colors cursor-pointer"
              :class="playerStore.activeLibraryTab === '歌单详情' && playerStore.activePlaylistId === playlist.id
                ? 'bg-accent text-bg-base font-bold' 
                : 'text-text-main hover:bg-bg-panel'"
            >
              <ListMusic class="w-4 h-4 mr-4 stroke-[1.5]" />
              <span class="flex-1 truncate">{{ playlist.name }}</span>
              <span class="text-[9px] opacity-70">{{ playlist.count }}</span>
            </a>
          </li>
          <li>
            <button class="flex items-center px-6 py-2 w-full text-left text-text-muted hover:text-text-main hover:bg-bg-panel transition-colors">
              <span class="w-4 h-4 mr-4 text-center font-bold">+</span>
              <span class="flex-1">NEW PLAYLIST</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- SOURCES -->
      <div class="mb-8">
        <h2 class="px-6 mb-4 text-[10px] text-text-muted font-bold tracking-widest">SOURCES</h2>
        <ul class="flex flex-col">
          <li>
            <a 
              href="#" 
              @click.prevent="playerStore.activeSourceTab = '本地音乐库'"
              class="flex items-center px-6 py-2 transition-colors cursor-pointer"
              :class="playerStore.activeSourceTab === '本地音乐库' 
                ? 'bg-accent text-bg-base font-bold' 
                : 'text-text-main hover:bg-bg-panel'"
            >
              <FolderOpen class="w-4 h-4 mr-4 stroke-[1.5]" />
              <span class="flex-1">LOCAL MUSIC</span>
            </a>
          </li>
          <li v-for="source in playerStore.sources.filter(s => s.kind !== 'local')" :key="source.id">
            <a href="#" class="flex items-center px-6 py-2 transition-colors cursor-pointer text-text-main hover:bg-bg-panel">
              <Server v-if="source.kind === 'webdav'" class="w-4 h-4 mr-4 stroke-[1.5]" />
              <HardDrive v-else class="w-4 h-4 mr-4 stroke-[1.5]" />
              <span class="flex-1 truncate uppercase">{{ source.name }}</span>
            </a>
          </li>
        </ul>
      </div>

    </div>

    <!-- Footer Settings + Theme Controls -->
    <div class="h-16 shrink-0 flex items-center gap-6 px-6 border-t border-border-color">
      <button 
        @click="playerStore.activeLibraryTab = '设置'" 
        class="hover:opacity-60 transition-opacity"
        :class="playerStore.activeLibraryTab === '设置' ? 'text-accent' : 'text-text-main'"
      >
        <Settings class="w-5 h-5 stroke-[1.5]" />
      </button>
      <ThemeControls variant="mono" :size="18" />
    </div>
  </aside>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 0;
}
</style>
