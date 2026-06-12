<script setup lang="ts">
import {
  Music2,
  Disc3,
  Mic2,
  Heart,
  History,
  ListVideo,
  ListMusic,
  Plus,
  FolderOpen,
  Cloud,
  Settings,
  Sun,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();

const libraryItems = [
  { name: '全部歌曲', icon: Music2 },
  { name: '专辑', icon: Disc3 },
  { name: '艺人', icon: Mic2 },
];

const myItems = [
  { name: '收藏歌曲', icon: Heart },
  { name: '最近播放', icon: History },
  { name: '播放队列', icon: ListVideo },
];

const sourceItems = [
  { name: '本地音乐库', icon: FolderOpen },
  { name: 'NAS (WebDAV)', icon: Cloud },
];
</script>

<template>
  <aside class="w-[240px] flex flex-col h-full bg-[#fcfcfc] border-r border-gray-100 p-5 select-none shrink-0 relative">
    <div data-tauri-drag-region class="absolute top-0 left-0 w-full h-16 z-0"></div>
    
    <!-- Logo -->
    <div class="flex items-center gap-3 mb-8 cursor-pointer pl-1 relative z-10 pointer-events-none">
      <img src="/lumo_log.ico" class="w-8 h-8 object-contain drop-shadow-sm" />
      <span class="font-bold text-xl tracking-wide text-gray-800">Lumo</span>
    </div>

    <!-- Navigation -->
    <div class="flex-1 overflow-y-auto pr-2 -mr-2 space-y-6">
      <!-- 曲库 -->
      <div>
        <h3 class="text-xs font-semibold text-gray-400 mb-2 px-3">曲库</h3>
        <ul class="space-y-1">
          <li v-for="item in libraryItems" :key="item.name">
            <a href="#" 
               @click.prevent="playerStore.activeLibraryTab = item.name"
               class="flex items-center gap-3 px-3 py-2.5 rounded-xl transition-smooth text-sm font-medium"
               :class="[
                 playerStore.activeLibraryTab === item.name ? 'bg-brand-orange-bg text-brand-orange shadow-sm shadow-orange-100/50' : 'text-gray-600 hover:bg-white hover:shadow-sm hover:text-gray-900',
               ]">
              <component :is="item.icon" class="w-[18px] h-[18px]" :class="[playerStore.activeLibraryTab === item.name ? 'text-brand-orange' : 'text-gray-400']" />
              <span class="flex-1">{{ item.name }}</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 我的 -->
      <div>
        <h3 class="text-xs font-semibold text-gray-400 mb-2 px-3">我的</h3>
        <ul class="space-y-1">
          <li v-for="item in myItems" :key="item.name">
            <a href="#" 
               @click.prevent="playerStore.activeLibraryTab = item.name"
               class="flex items-center gap-3 px-3 py-2.5 rounded-xl transition-smooth text-sm font-medium"
               :class="[
                 playerStore.activeLibraryTab === item.name ? 'bg-brand-orange-bg text-brand-orange shadow-sm shadow-orange-100/50' : 'text-gray-600 hover:bg-white hover:shadow-sm hover:text-gray-900',
               ]">
              <component :is="item.icon" class="w-[18px] h-[18px]" :class="[playerStore.activeLibraryTab === item.name ? 'text-brand-orange' : 'text-gray-400']" />
              <span class="flex-1">{{ item.name }}</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 歌单 -->
      <div>
        <h3 class="text-xs font-semibold text-gray-400 mb-2 px-3">歌单</h3>
        <ul class="space-y-1">
          <li v-for="playlist in playerStore.playlists" :key="playlist.name">
            <a href="#" 
               @click.prevent="playerStore.activeLibraryTab = playlist.name"
               class="flex items-center gap-3 px-3 py-2.5 rounded-xl transition-smooth text-sm font-medium"
               :class="[
                 playerStore.activeLibraryTab === playlist.name ? 'bg-brand-orange-bg text-brand-orange shadow-sm shadow-orange-100/50' : 'text-gray-600 hover:bg-white hover:shadow-sm hover:text-gray-900',
               ]">
              <ListMusic class="w-[18px] h-[18px]" :class="[playerStore.activeLibraryTab === playlist.name ? 'text-brand-orange' : 'text-gray-400']" />
              <span class="flex-1 truncate">{{ playlist.name }}</span>
              <span class="text-[11px] text-gray-400 font-normal">{{ playlist.count }}</span>
            </a>
          </li>
          <li class="pt-2 px-1">
            <button class="flex items-center gap-2 px-3 py-2 rounded-xl text-sm font-medium text-gray-600 hover:text-brand-orange transition-smooth bg-white border border-gray-100 shadow-sm hover:border-brand-orange-light w-full justify-center">
              <Plus class="w-4 h-4" />
              新建歌单
            </button>
          </li>
        </ul>
      </div>

      <!-- 来源 -->
      <div>
        <h3 class="text-xs font-semibold text-gray-400 mb-2 px-3">来源</h3>
        <ul class="space-y-1">
          <li v-for="item in sourceItems" :key="item.name">
            <a href="#" 
               @click.prevent="playerStore.activeSourceTab = item.name"
               class="flex items-center gap-3 px-3 py-2.5 rounded-xl transition-smooth text-sm font-medium"
               :class="[
                 playerStore.activeSourceTab === item.name ? 'bg-orange-50/50 text-brand-orange' : 'text-gray-600 hover:bg-white hover:shadow-sm hover:text-gray-900',
               ]">
              <component :is="item.icon" class="w-[18px] h-[18px]" :class="[playerStore.activeSourceTab === item.name ? 'text-brand-orange' : 'text-gray-400']" />
              <span class="flex-1">{{ item.name }}</span>
            </a>
          </li>
        </ul>
      </div>
    </div>

    <!-- Bottom Actions -->
    <div class="mt-auto pt-6 flex items-center gap-5 text-gray-400 px-3">
      <button class="hover:text-gray-700 transition-smooth hover-scale">
        <Settings class="w-5 h-5" />
      </button>
      <button @click="playerStore.isDarkMode = !playerStore.isDarkMode" class="hover:text-gray-700 transition-smooth hover-scale">
        <Sun class="w-5 h-5" />
      </button>
    </div>
  </aside>
</template>
