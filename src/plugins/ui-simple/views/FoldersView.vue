<script setup lang="ts">
import { onMounted, ref, onUnmounted } from 'vue';
import { usePlayerStore } from '../../../stores/player';
import { Folder, FileAudio, ChevronRight, CornerLeftUp, Play, Plus } from 'lucide-vue-next';

const playerStore = usePlayerStore();

onMounted(() => {
  if (playerStore.sources.length > 0 && !playerStore.activeFolderSourceId) {
    playerStore.fetchFolderContents(playerStore.sources[0].id);
  } else if (playerStore.activeFolderSourceId) {
    playerStore.fetchFolderContents(playerStore.activeFolderSourceId, playerStore.activeFolderPath || undefined);
  } else {
    playerStore.fetchSources().then(() => {
      if (playerStore.sources.length > 0 && !playerStore.activeFolderSourceId) {
        playerStore.fetchFolderContents(playerStore.sources[0].id);
      }
    });
  }
});

const handleSourceChange = (event: Event) => {
  const sourceId = Number((event.target as HTMLSelectElement).value);
  playerStore.fetchFolderContents(sourceId);
};

const goUpLevel = () => {
  if (!playerStore.activeFolderSourceId) return;
  if (playerStore.folderBreadcrumbs.length > 1) {
    const parentPath = playerStore.folderBreadcrumbs[playerStore.folderBreadcrumbs.length - 2];
    playerStore.fetchFolderContents(playerStore.activeFolderSourceId, parentPath);
  } else {
    playerStore.fetchFolderContents(playerStore.activeFolderSourceId);
  }
};

const openFolder = (path: string) => {
  if (playerStore.activeFolderSourceId) {
    playerStore.fetchFolderContents(playerStore.activeFolderSourceId, path);
  }
};

const playTrack = (track: any) => {
  if (!track) return;
  playerStore.queue = [track];
  playerStore.currentIndex = 0;
  playerStore.isPlaying = true;
};

const activeMenuFolderPath = ref<string | null>(null);

const openPlaylistMenu = (path: string) => {
  if (activeMenuFolderPath.value === path) {
    activeMenuFolderPath.value = null;
  } else {
    activeMenuFolderPath.value = path;
  }
};

const addFolderToPlaylist = async (playlistId: number, path: string) => {
  if (!playerStore.activeFolderSourceId) return;
  await playerStore.addFolderToPlaylist(playerStore.activeFolderSourceId, path, playlistId);
  activeMenuFolderPath.value = null;
};

const closeMenu = () => {
  activeMenuFolderPath.value = null;
};

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
</script>

<template>
  <div class="h-full flex flex-col pt-4 overflow-hidden">
    <!-- Source Selector & Breadcrumbs -->
    <div class="mb-6 shrink-0 flex flex-col gap-4">
      <div class="flex items-center gap-4">
        <span class="text-xs tracking-widest text-[#a0a0a0] font-bold uppercase">来源:</span>
        <select 
          class="bg-transparent border border-[#dcdad1] text-xs py-1 px-2 rounded outline-none focus:border-black cursor-pointer"
          :value="playerStore.activeFolderSourceId || ''"
          @change="handleSourceChange"
        >
          <option v-for="source in playerStore.sources" :key="source.id" :value="source.id">
            {{ source.name }} ({{ source.path || source.username || '远程源' }})
          </option>
        </select>
      </div>

      <div class="flex items-center gap-2 text-sm text-[#777] overflow-x-auto custom-scrollbar pb-2 whitespace-nowrap">
        <button 
          @click="() => playerStore.activeFolderSourceId && playerStore.fetchFolderContents(playerStore.activeFolderSourceId!)"
          class="hover:text-black transition-colors font-medium cursor-pointer shrink-0"
        >
          根目录
        </button>
        <template v-for="(path, index) in playerStore.folderBreadcrumbs" :key="path">
          <ChevronRight class="w-4 h-4 text-[#dcdad1] shrink-0" />
          <button 
            @click="playerStore.fetchFolderContents(playerStore.activeFolderSourceId!, path)"
            class="hover:text-black transition-colors shrink-0"
            :class="index === playerStore.folderBreadcrumbs.length - 1 ? 'text-black font-semibold' : ''"
          >
            {{ path.split(/\\|\//).pop() || path }}
          </button>
        </template>
      </div>
    </div>

    <!-- Contents List -->
    <div class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-32">
      <div v-if="playerStore.isFetchingFolder" class="py-12 text-center text-[#a0a0a0] text-sm tracking-widest animate-pulse">
        读取文件夹中...
      </div>
      
      <div v-else-if="playerStore.currentFolderContents.length === 0" class="py-12 text-center text-[#a0a0a0] text-sm tracking-widest">
        此文件夹为空
      </div>

      <ul v-else class="space-y-1">
        <!-- 上一级按钮 -->
        <li v-if="playerStore.folderBreadcrumbs.length > 0">
          <button 
            @click="goUpLevel"
            class="w-full flex items-center gap-4 py-3 px-4 rounded-lg hover:bg-black/[0.03] transition-colors group text-left"
          >
            <div class="w-10 h-10 flex items-center justify-center bg-[#f0eee9] rounded-md shrink-0">
              <CornerLeftUp class="w-5 h-5 text-[#888] group-hover:text-black transition-colors" />
            </div>
            <span class="text-sm font-medium tracking-wide">返回上一级</span>
          </button>
        </li>

        <li v-for="item in playerStore.currentFolderContents" :key="item.path">
          <!-- 文件夹 -->
          <div 
            v-if="item.is_dir"
            class="w-full flex items-center justify-between py-3 px-4 rounded-lg hover:bg-black/[0.03] transition-colors group relative"
          >
            <button @click="openFolder(item.path)" class="flex items-center gap-4 flex-1 text-left">
              <div class="w-10 h-10 flex items-center justify-center bg-[#f0eee9] rounded-md shrink-0">
                <Folder class="w-5 h-5 text-[#888] group-hover:text-black transition-colors" />
              </div>
              <span class="text-sm font-medium tracking-wide truncate flex-1">{{ item.name }}</span>
            </button>
            
            <div class="relative shrink-0 ml-4 flex items-center">
              <button @click.stop="openPlaylistMenu(item.path)" class="text-[#ccc] opacity-0 group-hover:opacity-100 hover:text-black transition-opacity p-2" title="添加文件夹下所有歌曲至歌单">
                <Plus class="w-4 h-4 stroke-[1.5]" />
              </button>
              
              <!-- Playlist menu -->
              <div v-if="activeMenuFolderPath === item.path" class="absolute right-0 top-full mt-1 bg-white border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-[#a0a0a0] whitespace-nowrap">暂无自建歌单</div>
                <button 
                  v-for="pl in playerStore.playlists" 
                  :key="pl.id"
                  @click.stop="addFolderToPlaylist(pl.id, item.path)"
                  class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-black hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
                >
                  {{ pl.name }}
                </button>
              </div>
            </div>
          </div>

          <!-- 音频文件 -->
          <div 
            v-else
            class="flex items-center justify-between py-3 px-4 rounded-lg hover:bg-black/[0.03] transition-colors group"
          >
            <div class="flex items-center gap-4 flex-1 min-w-0">
              <div 
                class="w-10 h-10 rounded-md overflow-hidden bg-[#eae8e1] shrink-0 relative cursor-pointer flex items-center justify-center"
                @click="item.track ? playTrack(item.track) : null"
              >
                <FileAudio class="w-5 h-5 text-[#888]" />
                
                <div class="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                  <Play class="w-4 h-4 text-white fill-white" />
                </div>
              </div>

              <div class="flex-1 min-w-0 flex flex-col justify-center">
                <h4 class="text-sm font-medium text-black truncate cursor-pointer hover:underline" @click="item.track ? playTrack(item.track) : null">
                  {{ item.track?.title || item.name }}
                </h4>
                <p class="text-xs text-[#888888] truncate mt-0.5">
                  {{ item.track?.artist || '未索引文件' }}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-6 shrink-0 text-xs text-[#a0a0a0] tracking-widest pl-4">
              <span v-if="item.track?.format" class="font-medium bg-[#eae8e1] px-2 py-0.5 rounded">{{ item.track.format }}</span>
              <span class="w-12 text-right">{{ item.track?.duration || '--:--' }}</span>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: #e5e3db;
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background-color: #dcdad1;
}
</style>
