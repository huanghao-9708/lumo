<script setup lang="ts">
import { computed, watch } from 'vue';
import { Search, Filter, List, LayoutGrid } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

import TracksView from './views/TracksView.vue';
import AlbumsView from './views/AlbumsView.vue';
import ArtistsView from './views/ArtistsView.vue';
import SettingsView from './views/SettingsView.vue';
import AlbumDetailView from './views/AlbumDetailView.vue';
import ArtistDetailView from './views/ArtistDetailView.vue';
import PlaylistDetailView from './views/PlaylistDetailView.vue';
import FoldersView from './views/FoldersView.vue';

const playerStore = usePlayerStore();

let searchTimeout: ReturnType<typeof setTimeout> | null = null;
watch(() => playerStore.searchQuery, () => {
  if (searchTimeout) clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    playerStore.fetchTracks(true);
    playerStore.fetchAlbums(true);
    playerStore.fetchArtists(true);
  }, 300);
});

// 根据 activeLibraryTab 决定当前渲染哪个组件
const currentView = computed(() => {
  const tab = playerStore.activeLibraryTab;
  if (tab === '专辑') return AlbumsView;
  if (tab === '艺人') return ArtistsView;
  if (tab === '设置') return SettingsView;
  if (tab === '文件夹') return FoldersView;
  if (tab === '专辑详情') return AlbumDetailView;
  if (tab === '艺人详情') return ArtistDetailView;
  if (tab === '歌单详情') return PlaylistDetailView;
  // 全部歌曲、收藏歌曲、最近播放、播放队列、歌单...默认都走 TracksView
  return TracksView;
});

// 计算主标题（特殊处理设置页）
const mainTitle = computed(() => {
  if (playerStore.activeLibraryTab === '设置') return 'Settings';
  return playerStore.activeLibraryTab;
});

const isDetailView = computed(() => {
  return ['专辑详情', '艺人详情', '歌单详情'].includes(playerStore.activeLibraryTab);
});
</script>

<template>
  <main class="flex-1 flex flex-col h-full bg-transparent relative overflow-hidden px-12 py-10">
    <div data-tauri-drag-region class="absolute inset-0 z-0"></div>

    <!-- Header -->
    <header v-if="!isDetailView" class="flex items-end justify-between shrink-0 mb-10 relative z-10">
      <div>
        <p class="text-[10px] font-bold tracking-[0.2em] text-[#888888] mb-2 uppercase">
          {{ playerStore.activeLibraryTab === '设置' ? 'SYSTEM & PREFERENCES' : 'INDEX — VOL. 01' }}
        </p>
        <h1 class="font-serif italic text-5xl tracking-wide text-black">{{ mainTitle }}</h1>
      </div>

      <div class="flex items-center gap-8 mb-2 transition-all duration-300" :class="!playerStore.isRightPanelOpen ? 'mr-56' : ''">
        <!-- 如果是设置页，可以隐藏搜索和过滤器，保持干净 -->
        <template v-if="playerStore.activeLibraryTab !== '设置'">
          <div class="relative group">
            <Search class="w-4 h-4 text-[#888] absolute left-0 top-1/2 -translate-y-1/2" />
            <input 
              type="text" 
              v-model="playerStore.searchQuery"
              placeholder="SEARCH..." 
              class="pl-8 pr-4 py-2 w-[240px] bg-transparent border-b border-[#dcdad1] focus:border-black text-xs tracking-widest focus:outline-none transition-colors placeholder-[#a0a0a0] uppercase"
            />
          </div>
          <div class="flex items-center gap-4 text-[#888]">
            <button class="hover:text-black transition-colors"><Filter class="w-4 h-4 stroke-[1.5]" /></button>
            <button class="hover:text-black transition-colors"><List class="w-4 h-4 stroke-[1.5]" /></button>
            <button class="hover:text-black transition-colors"><LayoutGrid class="w-4 h-4 stroke-[1.5]" /></button>
          </div>
        </template>
      </div>
    </header>

    <!-- 动态视图区域 -->
    <div class="flex-1 flex flex-col min-h-0 relative z-10">
      <component :is="currentView" />
    </div>
  </main>
</template>

<style scoped>
/* Scoped styles removed from MainContent since scrollbars are now handled in the sub-views */
</style>
