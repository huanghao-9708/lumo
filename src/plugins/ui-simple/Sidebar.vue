<script setup lang="ts">
import {
  ArrowLeft,
  Music,
  Disc,
  Mic,
  Heart,
  Clock,
  ListVideo,
  ListMusic,
  Plus,
  Settings,
  SunMoon,
  ChevronDown,
} from 'lucide-vue-next';
import { onMounted } from 'vue';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { invoke } from '@tauri-apps/api/core';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

const toggleTheme = () => {
  if (uiStore.activePlugin === 'ui-simple') {
    uiStore.setActivePlugin('ui-default');
  } else {
    uiStore.setActivePlugin('ui-simple');
  }
};

const createPlaylist = async () => {
  const name = prompt("请输入新歌单名称：");
  if (name && name.trim()) {
    try {
      await invoke('library_create_playlist', { name: name.trim() });
      playerStore.fetchPlaylists();
    } catch (e) {
      console.error(e);
      alert("创建失败");
    }
  }
};

onMounted(() => {
  playerStore.fetchPlaylists();
});
</script>

<template>
  <aside class="w-[240px] flex flex-col h-full bg-transparent border-r border-[#e8e6df] py-8 pl-8 pr-4 select-none shrink-0 relative">
    <div data-tauri-drag-region class="absolute top-0 left-0 w-full h-16 z-0"></div>
    
    <!-- Logo & Back Button -->
    <div class="mb-12 relative z-10 flex items-center justify-between">
      <h1 class="font-serif font-bold text-3xl tracking-widest text-black pointer-events-none">LUMO-V</h1>
      <button 
        @click="playerStore.goBack()"
        :disabled="!playerStore.canGoBack"
        class="w-7 h-7 rounded-md transition-all flex items-center justify-center -mr-2"
        :class="playerStore.canGoBack ? 'hover:bg-[#eae8e1] text-black cursor-pointer' : 'text-[#dcdad1] cursor-not-allowed'"
        title="返回"
      >
        <ArrowLeft class="w-4 h-4 stroke-[2]" />
      </button>
    </div>

    <!-- Navigation -->
    <div class="flex-1 overflow-y-auto pr-4 -mr-4 space-y-8 custom-scrollbar">
      <!-- 曲库 -->
      <div>
        <ul class="space-y-5">
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '全部歌曲'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '全部歌曲' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <Music class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">全部歌曲</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '专辑'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '专辑' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <Disc class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">专辑</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '艺人'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '艺人' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <Mic class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">艺人</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 收藏 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-5 uppercase">收藏</h3>
        <ul class="space-y-5">
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '收藏歌曲'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '收藏歌曲' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <Heart class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">收藏歌曲</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '最近播放'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '最近播放' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <Clock class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">最近播放</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '播放队列'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '播放队列' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <ListVideo class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">播放队列</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 歌单 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-5 uppercase">歌单</h3>
        <ul class="space-y-5">
          <li v-for="playlist in playerStore.playlists" :key="playlist.name">
            <a href="#" @click.prevent="playerStore.activeLibraryTab = playlist.name" class="flex items-center gap-4 text-[13px] transition-colors group" :class="playerStore.activeLibraryTab === playlist.name ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <ListMusic class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest flex-1">{{ playlist.name }}</span>
              <span class="text-[10px] text-[#cccccc] group-hover:text-[#888888]">{{ playlist.count }}</span>
            </a>
          </li>
          <li class="pt-2">
            <button @click="createPlaylist" class="flex items-center gap-4 text-[13px] font-medium text-[#777777] hover:text-black transition-colors w-full">
              <Plus class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">新建歌单</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- 来源 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-5 uppercase">来源</h3>
        <ul class="space-y-4">
          <li>
            <a href="#" @click.prevent="playerStore.activeSourceTab = '本地音乐库'" class="flex items-center justify-between text-[13px] transition-colors pr-2" :class="playerStore.activeSourceTab === '本地音乐库' ? 'text-black font-semibold' : 'text-[#777777] font-medium hover:text-black'">
              <span class="tracking-widest">本地音乐库</span>
              <ChevronDown class="w-4 h-4 stroke-[1.5]" />
            </a>
          </li>
        </ul>
      </div>
    </div>

    <!-- Bottom Actions -->
    <div class="mt-8 flex items-center gap-6 text-[#aaaaaa] relative z-10">
      <button 
        @click="playerStore.activeLibraryTab = '设置'" 
        class="transition-colors"
        :class="playerStore.activeLibraryTab === '设置' ? 'text-black' : 'hover:text-black'"
      >
        <Settings class="w-[18px] h-[18px] stroke-[1.5]" />
      </button>
      <button @click="toggleTheme" class="hover:text-black transition-colors" title="切换 UI 主题">
        <SunMoon class="w-[18px] h-[18px] stroke-[1.5]" />
      </button>
    </div>
  </aside>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: transparent;
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background-color: #dcdad1;
}
</style>
