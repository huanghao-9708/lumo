<script setup lang="ts">
import { 
  Settings, AudioLines, Disc3, User, Heart, Folder, Clock, ListMusic,
  Server, HardDrive, Plus
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

// Icons based on the image
const libraryItems = [
  { label: '全部歌曲', id: '全部歌曲', icon: AudioLines, count: '12,483' },
  { label: '专辑', id: '专辑', icon: Disc3, count: '1,246' },
  { label: '艺术家', id: '艺人', icon: User, count: '643' },
  { label: '作曲家', id: '作曲家', icon: Heart, count: '128' }, // Assuming heart is used for something else in the image, but let's stick to standard
  { label: '文件夹', id: '文件夹', icon: Folder, count: '892' },
  { label: '最近播放', id: '最近播放', icon: Clock, count: '200' },
  { label: '播放列表', id: '播放列表', icon: ListMusic, count: '28' },
];

</script>

<template>
  <aside class="w-full h-full flex flex-col font-sans text-sm bg-bg-panel/50 pt-8 pb-4">
    <!-- Header / Logo -->
    <div class="px-8 mb-8">
      <h1 class="text-2xl font-bold tracking-[0.2em] text-text-main">LUMO</h1>
      <p class="text-[9px] text-text-muted mt-2 tracking-widest uppercase">LOCAL MUSIC SYSTEM</p>
      <p class="text-[9px] text-text-muted mt-1 tracking-widest">V1.0.0</p>
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
              <span class="text-xs font-mono" :class="playerStore.activeLibraryTab === item.id ? 'text-text-main' : 'text-text-muted'">{{ item.count }}</span>
              
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
              @click.prevent="playerStore.activeLibraryTab = '歌单详情'; playerStore.activePlaylistId = playlist.id"
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
            <button class="flex items-center px-4 py-2 w-full text-left text-text-main hover:bg-bg-active/30 rounded-lg transition-colors group">
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

    <!-- Footer Settings -->
    <div class="px-8 mt-4 flex items-center gap-4">
      <button 
        @click="playerStore.activeLibraryTab = '设置'" 
        class="hover:opacity-60 transition-opacity"
        :class="playerStore.activeLibraryTab === '设置' ? 'text-accent' : 'text-text-main'"
      >
        <Settings class="w-4 h-4 stroke-[1.5]" />
      </button>
      <button 
        @click="uiStore.toggleDarkMode" 
        class="hover:opacity-60 transition-opacity text-text-main"
      >
        <div class="w-4 h-4 border-[1.5px] border-current rounded-full relative">
           <!-- Sun icon mock -->
           <div class="absolute -top-1 left-1.5 w-0.5 h-1 bg-current"></div>
           <div class="absolute -bottom-1 left-1.5 w-0.5 h-1 bg-current"></div>
           <div class="absolute -left-1 top-1.5 w-1 h-0.5 bg-current"></div>
           <div class="absolute -right-1 top-1.5 w-1 h-0.5 bg-current"></div>
        </div>
      </button>
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
