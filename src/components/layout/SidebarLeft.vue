<script setup lang="ts">
import { computed } from 'vue';
import {
  Activity, Disc, User, Heart, Folder, Clock, ListMusic, List, Plus, Star,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();

/** Library 一级导航。activeLibraryTab 是 store 里维护的当前页标识。 */
const libraryNav = computed(() => [
  { key: '全部歌曲', label: '全部歌曲', icon: Activity, count: playerStore.albumsTotalCount ? null : null },
  { key: '专辑', label: '专辑', icon: Disc, count: playerStore.albumsTotalCount },
  { key: '艺术家', label: '艺术家', icon: User, count: playerStore.artists.length },
  { key: '文件夹', label: '文件夹', icon: Folder, count: playerStore.localSources.length },
  { key: '最近播放', label: '最近播放', icon: Clock, count: null },
  { key: '播放列表', label: '播放列表', icon: ListMusic, count: playerStore.playlists.length },
]);

const favoritesNav = computed(() => [
  { key: '喜欢的音乐', label: '喜欢的音乐', icon: Heart, count: playerStore.tracks.length > 0 ? null : null },
  { key: '收藏的专辑', label: '收藏的专辑', icon: Disc, count: playerStore.favoriteAlbums.length },
  { key: '收藏的歌手', label: '收藏的歌手', icon: Star, count: playerStore.favoriteArtists.length },
]);

function isActive(key: string): boolean {
  // 「播放列表」分组里的具体歌单被选中时，也高亮「播放列表」入口
  if (key === '播放列表') return playerStore.activeLibraryTab === '播放列表' || playerStore.activePlaylistId !== null;
  return playerStore.activeLibraryTab === key;
}

function selectNav(key: string) {
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activePlaylistId = null;
  playerStore.activeLibraryTab = key;
}

function selectPlaylist(id: number) {
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activeLibraryTab = '播放列表';
  playerStore.activePlaylistId = id;
  playerStore.refreshCurrentPlaylistTracks(id);
}
</script>

<template>
  <div class="w-[240px] h-full bg-bg-canvas flex flex-col flex-shrink-0 select-none">
    <!-- Logo -->
    <div class="px-8 pt-8 pb-6 cursor-pointer" data-tauri-drag-region>
      <h1 class="text-xl font-bold tracking-[0.15em] text-text-primary mb-2">LUMO</h1>
      <p class="text-[9px] tracking-widest text-text-muted font-mono uppercase leading-tight">Local Music System</p>
      <p class="text-[9px] text-text-muted/60 font-mono mt-0.5">v1.0.0</p>
    </div>

    <!-- Scrollable Nav -->
    <div class="flex-1 overflow-y-auto px-5 pb-4">

      <!-- LIBRARY -->
      <div class="mb-6">
        <h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">Library</h2>

        <ul class="space-y-[2px]">
          <li v-for="item in libraryNav" :key="item.key">
            <a
              href="#"
              class="flex items-center px-3 py-[7px] rounded-[6px] transition-colors-smooth"
              :class="isActive(item.key)
                ? 'bg-list-selected text-text-primary'
                : 'text-text-primary hover:bg-list-hover'"
              @click.prevent="selectNav(item.key)"
            >
              <component
                :is="item.icon"
                class="w-[16px] h-[16px] mr-3 flex-shrink-0"
                :class="isActive(item.key) ? 'text-brand-orange' : 'text-text-muted'"
              />
              <span class="text-[13px] flex-1" :class="isActive(item.key) ? 'font-medium' : ''">{{ item.label }}</span>
              <span
                v-if="item.count !== null && item.count > 0"
                class="text-[11px] font-mono tabular-nums"
                :class="isActive(item.key) ? 'text-text-secondary' : 'text-text-muted'"
              >{{ item.count.toLocaleString() }}</span>
              <div v-if="isActive(item.key)" class="w-[6px] h-[6px] rounded-full bg-brand-orange ml-2 flex-shrink-0"></div>
            </a>
          </li>
        </ul>
      </div>

      <!-- FAVORITES -->
      <div class="mb-6">
        <h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">Favorites</h2>

        <ul class="space-y-[2px]">
          <li v-for="item in favoritesNav" :key="item.key">
            <a
              href="#"
              class="flex items-center px-3 py-[7px] rounded-[6px] transition-colors-smooth"
              :class="isActive(item.key)
                ? 'bg-list-selected text-text-primary'
                : 'text-text-primary hover:bg-list-hover'"
              @click.prevent="selectNav(item.key)"
            >
              <component
                :is="item.icon"
                class="w-[16px] h-[16px] mr-3 flex-shrink-0"
                :class="isActive(item.key) ? 'text-brand-orange' : 'text-text-muted'"
              />
              <span class="text-[13px] flex-1" :class="isActive(item.key) ? 'font-medium' : ''">{{ item.label }}</span>
              <span
                v-if="item.count !== null && item.count > 0"
                class="text-[11px] font-mono tabular-nums"
                :class="isActive(item.key) ? 'text-text-secondary' : 'text-text-muted'"
              >{{ item.count.toLocaleString() }}</span>
              <div v-if="isActive(item.key)" class="w-[6px] h-[6px] rounded-full bg-brand-orange ml-2 flex-shrink-0"></div>
            </a>
          </li>
        </ul>
      </div>

      <!-- PLAYLISTS -->
      <div>
        <h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">Playlists</h2>

        <ul class="space-y-[2px]">
          <li v-if="playerStore.playlists.length === 0">
            <p class="px-3 py-1 text-[12px] text-text-muted/70">暂无歌单</p>
          </li>

          <li v-for="pl in playerStore.playlists" :key="pl.id">
            <a
              href="#"
              class="flex items-center px-3 py-[7px] rounded-[6px] transition-colors-smooth"
              :class="playerStore.activePlaylistId === pl.id
                ? 'bg-list-selected text-text-primary'
                : 'text-text-primary hover:bg-list-hover'"
              @click.prevent="selectPlaylist(pl.id)"
            >
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1 truncate">{{ pl.name }}</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">{{ pl.count }}</span>
            </a>
          </li>

          <li class="mt-3">
            <a
              href="#"
              class="flex items-center px-3 py-[7px] rounded-[6px] text-text-muted hover:bg-list-hover hover:text-text-primary transition-colors-smooth"
              @click.prevent="playerStore.isCreatePlaylistModalOpen = true"
            >
              <Plus class="w-[16px] h-[16px] mr-3 flex-shrink-0" />
              <span class="text-[13px] flex-1">新建播放列表</span>
            </a>
          </li>
        </ul>
      </div>

    </div>
  </div>
</template>
