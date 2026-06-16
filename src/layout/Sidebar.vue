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
  Trash2,
  Folder,
} from 'lucide-vue-next';
import { onMounted } from 'vue';
import { usePlayerStore } from '../stores/player';
import { useUiStore } from '../stores/ui';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

const toggleTheme = () => {
  uiStore.toggleDarkMode();
};

const confirmDeletePlaylist = async (playlist: any) => {
  if (confirm(`确定要删除歌单“${playlist.name}”吗？`)) {
    try {
      await playerStore.deletePlaylist(playlist.id);
    } catch (e) {
      alert("删除歌单失败");
    }
  }
};

const createPlaylist = () => {
  playerStore.isCreatePlaylistModalOpen = true;
};

const openPlaylist = (id: number) => {
  if (playerStore.activePlaylistId === id) {
    playerStore.refreshCurrentPlaylistTracks(id);
  } else {
    playerStore.activePlaylistId = id;
  }
  playerStore.activeLibraryTab = '歌单详情';
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
      <div class="flex items-center gap-2">
        <img src="/lumo_log.ico" class="w-8 h-8 object-contain drop-shadow-sm" />
        <h1 class="font-serif font-bold text-3xl tracking-widest text-accent pointer-events-none">LUMO</h1>
      </div>
      <button 
        @click="playerStore.goBack()"
        :disabled="!playerStore.canGoBack"
        class="w-7 h-7 rounded-md transition-all flex items-center justify-center -mr-2"
        :class="playerStore.canGoBack ? 'hover:bg-bg-panel   text-accent cursor-pointer' : 'text-[#dcdad1] cursor-not-allowed'"
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
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '全部歌曲'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '全部歌曲' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Music class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">全部歌曲</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '专辑'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '专辑' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Disc class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">专辑</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '艺人'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '艺人' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Mic class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">艺人</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '文件夹'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '文件夹' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Folder class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">文件夹</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 收藏 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-text-muted mb-5 uppercase">收藏</h3>
        <ul class="space-y-5">
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '收藏歌曲'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '收藏歌曲' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Heart class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">收藏歌曲</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '最近播放'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '最近播放' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <Clock class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">最近播放</span>
            </a>
          </li>
          <li>
            <a href="#" @click.prevent="playerStore.activeLibraryTab = '播放队列'" class="flex items-center gap-4 text-[13px] transition-colors" :class="playerStore.activeLibraryTab === '播放队列' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
              <ListVideo class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">播放队列</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- 歌单 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-text-muted mb-5 uppercase">歌单</h3>
        <ul class="space-y-5">
          <li v-for="playlist in playerStore.playlists" :key="playlist.id">
            <div class="flex items-center justify-between group py-1 pr-2 rounded-md transition-colors hover:bg-black/[0.03]">
              <a href="#" @click.prevent="openPlaylist(playlist.id)" class="flex items-center gap-4 text-[13px] transition-colors flex-1 min-w-0" :class="playerStore.activeLibraryTab === '歌单详情' && playerStore.activePlaylistId === playlist.id ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
                <ListMusic class="w-4 h-4 stroke-[1.5] shrink-0" />
                <span class="tracking-widest truncate">{{ playlist.name }}</span>
              </a>
              <div class="flex items-center gap-2 shrink-0">
                <span class="text-[10px] text-[#cccccc] group-hover:hidden">{{ playlist.count }}</span>
                <button 
                  @click.stop="confirmDeletePlaylist(playlist)" 
                  class="hidden group-hover:flex text-text-muted hover:text-[#d25050] transition-colors"
                  title="删除歌单"
                >
                  <Trash2 class="w-3.5 h-3.5 stroke-[1.5]" />
                </button>
              </div>
            </div>
          </li>
          <li class="pt-2">
            <button @click="createPlaylist" class="flex items-center gap-4 text-[13px] font-medium text-text-muted hover:text-accent  transition-colors w-full">
              <Plus class="w-4 h-4 stroke-[1.5]" />
              <span class="tracking-widest">新建歌单</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- 来源 -->
      <div>
        <h3 class="text-[10px] font-bold tracking-[0.2em] text-text-muted mb-5 uppercase">来源</h3>
        <ul class="space-y-4">
          <li>
            <a href="#" @click.prevent="playerStore.activeSourceTab = '本地音乐库'" class="flex items-center justify-between text-[13px] transition-colors pr-2" :class="playerStore.activeSourceTab === '本地音乐库' ? 'text-accent font-semibold' : 'text-text-muted font-medium hover:text-accent '">
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
        :class="playerStore.activeLibraryTab === '设置' ? 'text-accent' : 'hover:text-accent '"
      >
        <Settings class="w-[18px] h-[18px] stroke-[1.5]" />
      </button>
      <button @click="toggleTheme" class="hover:text-accent  transition-colors" title="切换 UI 主题">
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
