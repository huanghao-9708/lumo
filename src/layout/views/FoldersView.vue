<script setup lang="ts">
import { onMounted, ref, onUnmounted, computed } from 'vue';
import { usePlayerStore } from '../../stores/player';
import { useVirtualList } from '../../composables/useVirtualList';
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

// ============ 虚拟滚动 + 增量加载 ============
// 行高：py-3 (12px*2) + 内容高度 40px ≈ 64px。实测若略有偏差可微调。
const ROW_HEIGHT = 64;
const scrollContainer = ref<HTMLElement | null>(null);

// 把"返回上一级"和真实条目合并为一个虚拟列表数据源，让虚拟化统一管理。
// 第一项（index 0）固定为"返回上一级"按钮（仅当有面包屑时存在）。
interface VRow {
  kind: 'back' | 'entry';
  entry?: import('../../stores/player').FolderEntry;
}

const hasBackButton = computed(() => playerStore.folderBreadcrumbs.length > 0);

const virtualRows = computed<VRow[]>(() => {
  const rows: VRow[] = [];
  if (hasBackButton.value) rows.push({ kind: 'back' });
  for (const e of playerStore.currentFolderContents) {
    rows.push({ kind: 'entry', entry: e });
  }
  return rows;
});

const { totalHeight, offsetY, visibleItems } = useVirtualList<VRow>({
  containerRef: scrollContainer,
  items: virtualRows,
  itemHeight: ROW_HEIGHT,
  buffer: 8,
  columns: 1,
});

// 滚动监听：除了驱动虚拟列表（在 composable 内部已处理），还用来触发增量加载
const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  // 接近底部时拉取下一页
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 300) {
    if (playerStore.hasMoreFolderEntries && !playerStore.isFetchingFolder) {
      playerStore.fetchMoreFolderEntries();
    }
  }
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
        <span class="text-xs tracking-widest text-text-muted font-bold uppercase">来源:</span>
        <select
          class="bg-transparent border border-border-color  text-xs py-1 px-2 rounded outline-none focus:border-black cursor-pointer"
          :value="playerStore.activeFolderSourceId || ''"
          @change="handleSourceChange"
        >
          <option v-for="source in playerStore.sources" :key="source.id" :value="source.id">
            {{ source.name }} ({{ source.path || source.username || '远程源' }})
          </option>
        </select>
      </div>

      <div class="flex items-center gap-2 text-sm text-text-muted overflow-x-auto custom-scrollbar pb-2 whitespace-nowrap">
        <button
          @click="() => playerStore.activeFolderSourceId && playerStore.fetchFolderContents(playerStore.activeFolderSourceId!)"
          class="hover:text-accent  transition-colors font-medium cursor-pointer shrink-0"
        >
          根目录
        </button>
        <template v-for="(path, index) in playerStore.folderBreadcrumbs" :key="path">
          <ChevronRight class="w-4 h-4 text-[#dcdad1] shrink-0" />
          <button
            @click="playerStore.fetchFolderContents(playerStore.activeFolderSourceId!, path)"
            class="hover:text-accent  transition-colors shrink-0"
            :class="index === playerStore.folderBreadcrumbs.length - 1 ? 'text-accent font-semibold' : ''"
          >
            {{ path.split(/\\|\//).pop() || path }}
          </button>
        </template>
      </div>
    </div>

    <!-- Contents List -->
    <div
      ref="scrollContainer"
      class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-32"
      @scroll="handleScroll"
    >
      <div v-if="playerStore.isFetchingFolder && playerStore.currentFolderContents.length === 0" class="py-12 text-center text-text-muted text-sm tracking-widest animate-pulse">
        读取文件夹中...
      </div>

      <div v-else-if="playerStore.currentFolderContents.length === 0 && !hasBackButton" class="py-12 text-center text-text-muted text-sm tracking-widest">
        此文件夹为空
      </div>

      <!--
        虚拟滚动容器：
        - 外层 div 用 totalHeight 撑出完整滚动条
        - 内层 div 用 transform: translateY 定位到当前可视窗口
        - 只渲染 visibleItems（约 20-30 行），不管总条目数多大
      -->
      <ul
        v-else
        class="relative"
        :style="{ height: totalHeight + 'px' }"
      >
        <div
          class="absolute top-0 left-0 right-0 space-y-1 will-change-transform"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <template v-for="item in visibleItems" :key="item.data.kind === 'back' ? '__back__' : item.data.entry!.path">
            <!-- 返回上一级 -->
            <li v-if="item.data.kind === 'back'" :style="{ height: ROW_HEIGHT + 'px' }">
              <button
                @click="goUpLevel"
                class="w-full flex items-center gap-4 py-3 px-4 rounded-lg hover:bg-black/[0.03] transition-colors group text-left"
              >
                <div class="w-10 h-10 flex items-center justify-center bg-[#f0eee9] rounded-md shrink-0">
                  <CornerLeftUp class="w-5 h-5 text-text-muted  group-hover:text-accent  transition-colors" />
                </div>
                <span class="text-sm font-medium tracking-wide">返回上一级</span>
              </button>
            </li>

            <!-- 真实条目 -->
            <li v-else :style="{ height: ROW_HEIGHT + 'px' }">
              <!-- 文件夹 -->
              <div
                v-if="item.data.entry!.is_dir"
                class="w-full flex items-center justify-between py-3 px-4 rounded-lg hover:bg-black/[0.03] transition-colors group relative"
              >
                <button @click="openFolder(item.data.entry!.path)" class="flex items-center gap-4 flex-1 text-left">
                  <div class="w-10 h-10 flex items-center justify-center bg-[#f0eee9] rounded-md shrink-0">
                    <Folder class="w-5 h-5 text-text-muted  group-hover:text-accent  transition-colors" />
                  </div>
                  <span class="text-sm font-medium tracking-wide truncate flex-1">{{ item.data.entry!.name }}</span>
                </button>

                <div class="relative shrink-0 ml-4 flex items-center">
                  <button @click.stop="openPlaylistMenu(item.data.entry!.path)" class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent  transition-opacity p-2" title="添加文件夹下所有歌曲至歌单">
                    <Plus class="w-4 h-4 stroke-[1.5]" />
                  </button>

                  <!-- Playlist menu -->
                  <div v-if="activeMenuFolderPath === item.data.entry!.path" class="absolute right-0 top-full mt-1 bg-bg-base border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                    <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-text-muted whitespace-nowrap">暂无自建歌单</div>
                    <button
                      v-for="pl in playerStore.playlists"
                      :key="pl.id"
                      @click.stop="addFolderToPlaylist(pl.id, item.data.entry!.path)"
                      class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-accent  hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
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
                    class="w-10 h-10 rounded-md overflow-hidden bg-bg-panel  shrink-0 relative cursor-pointer flex items-center justify-center"
                    @click="item.data.entry!.track ? playTrack(item.data.entry!.track) : null"
                  >
                    <FileAudio class="w-5 h-5 text-text-muted " />

                    <div class="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                      <Play class="w-4 h-4 text-white fill-white" />
                    </div>
                  </div>

                  <div class="flex-1 min-w-0 flex flex-col justify-center">
                    <h4 class="text-sm font-medium text-accent truncate cursor-pointer hover:underline" @click="item.data.entry!.track ? playTrack(item.data.entry!.track) : null">
                      {{ item.data.entry!.track?.title || item.data.entry!.name }}
                    </h4>
                    <p class="text-xs text-[#888888] truncate mt-0.5">
                      {{ item.data.entry!.track?.artist || '未索引文件' }}
                    </p>
                  </div>
                </div>

                <div class="flex items-center gap-6 shrink-0 text-xs text-text-muted tracking-widest pl-4">
                  <span v-if="item.data.entry!.track?.format" class="font-medium bg-bg-panel  px-2 py-0.5 rounded">{{ item.data.entry!.track.format }}</span>
                  <span class="w-12 text-right">{{ item.data.entry!.track?.duration || '--:--' }}</span>
                </div>
              </div>
            </li>
          </template>
        </div>
      </ul>

      <!-- 增量加载指示器 -->
      <div v-if="playerStore.isFetchingFolder && playerStore.currentFolderContents.length > 0" class="py-4 text-center text-text-muted text-xs tracking-widest animate-pulse">
        加载更多...
      </div>
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
