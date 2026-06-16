<script setup lang="ts">
import { computed, watch } from 'vue';
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

</script>

<template>
  <main class="flex-1 flex flex-col h-full bg-bg-base relative overflow-hidden">
    <div data-tauri-drag-region class="absolute inset-0 z-0 h-10 pointer-events-none"></div>

    <!-- Content Area -->
    <div class="flex-1 overflow-hidden relative z-10 w-full h-full">
      <transition name="fade" mode="out-in">
        <component :is="currentView" />
      </transition>
    </div>
  </main>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
