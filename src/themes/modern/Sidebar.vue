<script setup lang="ts">
import {
  Settings, AudioLines, Disc3, User, Heart, Folder, Clock, ListMusic,
  Server, HardDrive, Plus, ArrowLeft
} from 'lucide-vue-next';
import { onMounted } from 'vue';
import { usePlayerStore } from '../../stores/player';
import ThemeControls from '../../components/ThemeControls.vue';

const playerStore = usePlayerStore();

// 侧边栏导航项。
// 注意：id 必须与 MainContent.vue 里的 currentView 匹配（'艺人' 而非 '艺术家'）。
// 此前这里写成了 '艺术家'，导致点击后 MainContent 路由不命中、永远进 TracksView。
const libraryItems = [
  { label: '全部歌曲', id: '全部歌曲', icon: AudioLines },
  { label: '专辑', id: '专辑', icon: Disc3 },
  { label: '艺人', id: '艺人', icon: User },
  { label: '收藏歌曲', id: '收藏歌曲', icon: Heart },
  { label: '文件夹', id: '文件夹', icon: Folder },
  { label: '最近播放', id: '最近播放', icon: Clock },
];

const openPlaylist = (id: number) => {
  // 复用 simple 主题的逻辑：若已在当前歌单详情则刷新，否则切换。
  if (playerStore.activePlaylistId === id) {
    playerStore.refreshCurrentPlaylistTracks(id);
  } else {
    playerStore.activePlaylistId = id;
  }
  playerStore.activeLibraryTab = '歌单详情';
};

const createPlaylist = () => {
  playerStore.isCreatePlaylistModalOpen = true;
};

// 修复 bug：此前 modern 侧边栏从未拉取歌单数据，PLAYLISTS 区块永远为空。
onMounted(() => {
  playerStore.fetchPlaylists();
});
</script>

<template>
  <aside class="w-full h-full flex flex-col font-sans text-sm bg-bg-panel/50 pt-8 pb-4">
    <!-- Header / Logo -->
    <div class="px-8 mb-8 flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold tracking-[0.2em] text-text-main">LUMO</h1>
        <p class="text-[9px] text-text-muted mt-2 tracking-widest uppercase">LOCAL MUSIC SYSTEM</p>
        <p class="text-[9px] text-text-muted mt-1 tracking-widest">V1.0.0</p>
      </div>
      <button
        @click="playerStore.goBack()"
        :disabled="!playerStore.canGoBack"
        class="w-7 h-7 rounded-md transition-all flex items-center justify-center"
        :class="playerStore.canGoBack ? 'hover:bg-bg-active text-accent cursor-pointer' : 'text-text-muted/40 cursor-not-allowed'"
        title="返回"
      >
        <ArrowLeft class="w-4 h-4 stroke-[2]" />
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-4 custom-scrollbar">

      <!-- LIBRARY -->
      <div class="mb-8">
        <h2 class="px-4 mb-3 text-[10px] text-text-muted font-bold tracking-widest uppercase">LIBRARY</h2>
        <ul class="flex flex-col gap-0.5">
          <li v-for="item in libraryItems" :key="item.id">
            <a
              href="#"
              @click.prevent="playerStore.activeLibraryTab = item.id as any"
              class="group flex items-center px-4 py-2 rounded-lg transition-colors cursor-pointer relative"
              :class="playerStore.activeLibraryTab === item.id
                ? 'bg-bg-active/60 font-bold text-text-main'
                : 'text-text-main hover:bg-bg-active/30'"
            >
              <component :is="item.icon" class="w-4 h-4 mr-4 stroke-[1.5]" :class="playerStore.activeLibraryTab === item.id ? 'text-text-main' : 'text-text-muted'" />
              <span class="flex-1">{{ item.label }}</span>

              <!-- Active Dot Indicator -->
              <div v-if="playerStore.activeLibraryTab === item.id" class="w-1.5 h-1.5 rounded-full bg-accent absolute right-2"></div>
            </a>
          </li>
        </ul>
      </div>

      <!-- PLAYLISTS -->
      <div class="mb-8">
        <h2 class="px-4 mb-3 text-[10px] text-text-muted font-bold tracking-widest uppercase">PLAYLISTS</h2>
        <ul class="flex flex-col gap-0.5">
          <li v-for="playlist in playerStore.playlists" :key="playlist.id">
            <a
              href="#"
              @click.prevent="openPlaylist(playlist.id)"
              class="group flex items-center px-4 py-2 rounded-lg transition-colors cursor-pointer relative"
              :class="playerStore.activeLibraryTab === '歌单详情' && playerStore.activePlaylistId === playlist.id
                ? 'bg-bg-active/60 font-bold text-text-main'
                : 'text-text-main hover:bg-bg-active/30'"
            >
              <ListMusic class="w-4 h-4 mr-4 stroke-[1.5] text-text-muted" />
              <span class="flex-1 truncate">{{ playlist.name }}</span>
              <span class="text-xs font-mono text-text-muted">{{ playlist.count }}</span>
              <div v-if="playerStore.activeLibraryTab === '歌单详情' && playerStore.activePlaylistId === playlist.id" class="w-1.5 h-1.5 rounded-full bg-accent absolute right-2"></div>
            </a>
          </li>
          <li>
            <button @click="createPlaylist" class="flex items-center px-4 py-2 w-full text-left text-text-main hover:bg-bg-active/30 rounded-lg transition-colors group">
              <Plus class="w-4 h-4 mr-4 stroke-[1.5] text-text-muted group-hover:text-text-main" />
              <span class="flex-1">新建播放列表</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- SOURCES -->
      <div class="mb-8">
        <h2 class="px-4 mb-3 text-[10px] text-text-muted font-bold tracking-widest uppercase">SOURCES</h2>
        <ul class="flex flex-col gap-0.5">
          <li>
            <a
              href="#"
              @click.prevent="playerStore.activeSourceTab = '本地音乐库'"
              class="group flex items-center px-4 py-2 rounded-lg transition-colors cursor-pointer relative"
              :class="playerStore.activeSourceTab === '本地音乐库'
                ? 'bg-bg-active/60 font-bold text-text-main'
                : 'text-text-main hover:bg-bg-active/30'"
            >
              <div class="w-4 h-4 rounded-full bg-text-main mr-4 flex-shrink-0 border-2 border-bg-base shadow-sm"></div>
              <span class="flex-1">本地音乐</span>
              <div v-if="playerStore.activeSourceTab === '本地音乐库'" class="w-1.5 h-1.5 rounded-full bg-accent absolute right-2"></div>
            </a>
          </li>
          <li v-for="source in playerStore.sources.filter(s => s.kind !== 'local')" :key="source.id">
            <a href="#" class="flex items-center px-4 py-2 rounded-lg transition-colors cursor-pointer text-text-main hover:bg-bg-active/30">
              <Server v-if="source.kind === 'webdav'" class="w-4 h-4 mr-4 stroke-[1.5] text-text-muted" />
              <HardDrive v-else class="w-4 h-4 mr-4 stroke-[1.5] text-text-muted" />
              <span class="flex-1 truncate">{{ source.name }}</span>
            </a>
          </li>
        </ul>
      </div>

    </div>

    <!-- Footer: 设置 + 主题切换 + 夜间模式（统一放在设置按钮旁） -->
    <div class="px-8 mt-4 flex items-center gap-4 text-text-main">
      <button
        @click="playerStore.activeLibraryTab = '设置'"
        class="hover:opacity-60 transition-opacity"
        :class="playerStore.activeLibraryTab === '设置' ? 'text-accent' : 'text-text-main'"
        title="设置"
      >
        <Settings class="w-[18px] h-[18px] stroke-[1.5]" />
      </button>
      <ThemeControls :size="18" />
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
  border-radius: 4px;
}
</style>
