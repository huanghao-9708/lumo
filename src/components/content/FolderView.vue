<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  Folder, ChevronRight, ChevronDown, Play, Loader2, Music, Heart, Plus
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import type { DirectoryNodeDTO } from '../../api/types';
import { libraryGetFolderChildren } from '../../api/library';

const playerStore = usePlayerStore();

const selectedSourceId = ref<number | null>(null);
const loadedChildren = ref<Record<string, DirectoryNodeDTO[]>>({});
const expandedPaths = ref<Record<string, boolean>>({});
const loadingPaths = ref<Record<string, boolean>>({});
const pickerDirPath = ref<string | null>(null);

const sources = computed(() => playerStore.localSources);

const visibleTree = computed(() => {
  type FlatNode = DirectoryNodeDTO & { depth: number; isExpanded: boolean; isLoading: boolean };
  const result: FlatNode[] = [];

  function walk(parentPath: string, depth: number) {
    const children = loadedChildren.value[parentPath];
    if (!children) return;
    for (const child of children) {
      const fullPath = child.path;
      const isExpanded = !!expandedPaths.value[fullPath];
      result.push({
        ...child,
        depth,
        isExpanded,
        isLoading: !!loadingPaths.value[fullPath],
      });
      if (isExpanded) {
        walk(fullPath, depth + 1);
      }
    }
  }

  walk('', 0);
  return result;
});

async function loadChildrenForPath(parentPath: string) {
  if (!selectedSourceId.value) return;
  if (loadedChildren.value[parentPath]) return;
    loadingPaths.value = { ...loadingPaths.value, [parentPath]: true };
  try {
    const res = await libraryGetFolderChildren(selectedSourceId.value, parentPath || undefined);
    loadedChildren.value = { ...loadedChildren.value, [parentPath]: res.children };
  } finally {
    const nextLoading = { ...loadingPaths.value };
    delete nextLoading[parentPath];
    loadingPaths.value = nextLoading;
  }
}

function toggleNode(node: DirectoryNodeDTO & { depth: number; isExpanded: boolean; isLoading: boolean }) {
  const fullPath = node.path;
  if (expandedPaths.value[fullPath]) {
    const next = { ...expandedPaths.value };
    delete next[fullPath];
    expandedPaths.value = next;
  } else {
    expandedPaths.value = { ...expandedPaths.value, [fullPath]: true };
    if (!loadedChildren.value[fullPath]) {
      loadChildrenForPath(fullPath);
    }
    selectPath(fullPath);
  }
}

function selectPath(fullPath: string) {
  if (!selectedSourceId.value) return;
  playerStore.fetchFolderTracks(selectedSourceId.value, fullPath, true);
}


function breadcrumbClick(parts: string[]) {
  if (!selectedSourceId.value) return;
  const path = parts.join('\\');
  selectPath(path);
}

function formatFileSize(bytes: number | null): string {
  if (bytes == null) return '--';
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
}

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playTrack(index: number) {
  if (playerStore.folderTracks.length > 0) {
    playerStore.playAll(playerStore.folderTracks, index);
  }
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

function openPlaylistPicker(dirPath: string, e: Event) {
  e.stopPropagation();
  pickerDirPath.value = dirPath;
}

async function addDirToPlaylist(playlistId: number) {
  if (!selectedSourceId.value || !pickerDirPath.value) return;
  const ok = await playerStore.addFolderToPlaylist(selectedSourceId.value, pickerDirPath.value, playlistId);
  if (ok) {
    pickerDirPath.value = null;
  }
}

watch(sources, (list) => {
  if (list.length > 0 && !selectedSourceId.value) {
    selectedSourceId.value = list[0].id;
    loadedChildren.value = {};
    expandedPaths.value = {};
    loadChildrenForPath('');
  }
}, { immediate: true });

watch(selectedSourceId, (id) => {
  if (id != null) {
    loadedChildren.value = {};
    expandedPaths.value = {};
    playerStore.folderTracks = [];
    loadChildrenForPath('');
  }
});

const sourceName = computed(() =>
  sources.value.find(s => s.id === selectedSourceId.value)?.name ?? ''
);

const currentBreadcrumb = computed(() => {
  const sp = playerStore.selectedTreePath;
  if (!sp) return [];
  return sp.split('\\').filter(Boolean);
});
</script>

<template>
  <div class="flex-1 flex bg-bg-content overflow-hidden select-none min-w-0">

    <!-- Left: Folder Tree -->
    <div class="w-[280px] flex-shrink-0 border-r border-border-color flex flex-col overflow-hidden">
      <div class="px-5 pt-6 pb-3 text-[10px] font-semibold text-text-muted uppercase tracking-widest flex-shrink-0">文件浏览器</div>

      <div class="px-4 pb-2 flex-shrink-0">
        <select
          v-model="selectedSourceId"
          class="w-full h-[32px] px-2 text-[12px] bg-bg-canvas border border-border-color rounded-[6px]"
        >
          <option v-for="s in sources" :key="s.id" :value="s.id">{{ s.name }}</option>
        </select>
      </div>

      <div class="flex-1 overflow-y-auto px-2 pb-4 space-y-[2px]">
        <div v-if="sources.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
          <p class="text-[12px]">暂无数据源</p>
        </div>

        <template v-for="node in visibleTree" :key="node.path">
          <div
            class="flex items-center gap-1 rounded-[6px] cursor-pointer transition-colors-smooth h-[32px] text-[13px]"
            :class="playerStore.selectedTreePath === node.path ? 'bg-list-selected' : 'hover:bg-list-hover'"
            :style="{ paddingLeft: (node.depth * 16 + 8) + 'px' }"
            @click="toggleNode(node)"
          >
            <div class="w-4 flex items-center justify-center flex-shrink-0">
              <div v-if="node.isLoading" class="w-3.5 h-3.5 rounded-full border-2 border-text-muted border-t-transparent animate-spin"></div>
              <ChevronRight v-else-if="node.has_subdirs && !node.isExpanded" class="w-3.5 h-3.5 text-text-muted flex-shrink-0" />
              <ChevronDown v-else-if="node.has_subdirs && node.isExpanded" class="w-3.5 h-3.5 text-text-muted flex-shrink-0" />
            </div>
            <Folder class="w-4 h-4 text-text-muted flex-shrink-0" />
            <span class="truncate flex-1 text-text-primary">{{ node.name }}</span>
            <span class="text-text-muted text-[11px] tabular-nums mr-1">({{ node.audio_count }})</span>
            <button
              class="w-5 h-5 flex items-center justify-center hover:bg-list-hover rounded flex-shrink-0 text-text-muted transition-colors-smooth"
              @click.stop="openPlaylistPicker(node.path, $event)"
            >
              <Plus class="w-3.5 h-3.5" />
            </button>
          </div>
        </template>

        <div v-if="visibleTree.length === 0 && !loadingPaths[''] && sources.length > 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
          <Music class="w-5 h-5 mb-2" />
          <span class="text-[12px]">未找到音乐文件</span>
        </div>
      </div>
    </div>

    <!-- Right: Track List -->
    <div class="flex-1 flex flex-col overflow-hidden">

      <!-- Breadcrumb -->
      <div class="px-6 pt-4 pb-2 flex items-center gap-1 text-[11px] text-text-muted font-mono flex-shrink-0" v-if="selectedSourceId">
        <button class="hover:text-text-primary transition-colors-smooth" @click="selectPath('')">
          {{ sourceName }}
        </button>
        <template v-for="(crumb, i) in currentBreadcrumb" :key="i">
          <span class="mx-1">/</span>
          <button
            class="hover:text-text-primary transition-colors-smooth"
            @click="breadcrumbClick(currentBreadcrumb.slice(0, i + 1))"
          >{{ crumb }}</button>
        </template>
      </div>

      <!-- Track header -->
      <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider px-6 py-2 border-b border-border-color flex-shrink-0">
        <div class="w-10 text-center shrink-0">#</div>
        <div class="w-8 shrink-0"></div>
        <div class="flex-[2] min-w-0 pl-1">标题</div>
        <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
        <div class="w-[70px] text-right shrink-0 hidden md:block">大小</div>
        <div class="w-[56px] text-right shrink-0 hidden md:block">时长</div>
        <div class="w-8 shrink-0"></div>
      </div>

      <!-- Track rows -->
      <div
        class="flex-1 overflow-y-auto px-6"
        @scroll="(e) => {
          const el = e.target as HTMLElement;
          if (el.scrollTop + el.clientHeight >= el.scrollHeight - 400) {
            if (selectedSourceId && playerStore.selectedTreePath && !playerStore.isLoadingFolderTracks && playerStore.hasMoreFolderTracks) {
              playerStore.fetchMoreFolderTracks(selectedSourceId, playerStore.selectedTreePath);
            }
          }
        }"
      >
        <div v-if="playerStore.isLoadingFolderTracks && playerStore.folderTracks.length === 0" class="flex items-center justify-center py-16">
          <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
        </div>

        <div v-else-if="playerStore.folderTracks.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
          <Music class="w-6 h-6 text-text-disabled mb-2" />
          <span class="text-[12px]">该文件夹没有可播放的歌曲</span>
        </div>

        <div v-else>
          <div
            v-for="(track, index) in playerStore.folderTracks"
            :key="track.id"
            :class="{
              'playing-row bg-list-selected': isPlayingTrack(track.id),
              'hover:bg-list-hover': !isPlayingTrack(track.id)
            }"
            class="flex items-center transition-colors-smooth group cursor-pointer relative"
            style="height: 40px;"
            @dblclick="playTrack(index)"
          >
            <div class="w-10 text-center shrink-0 text-[12px] font-mono">
              <span v-if="isPlayingTrack(track.id)" class="text-brand-orange inline-flex items-center justify-center">
                <Play class="w-[12px] h-[12px] fill-current" />
              </span>
              <template v-else>
                <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
                <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
              </template>
            </div>

            <div class="w-8 shrink-0 flex items-center justify-center">
              <Heart
                v-if="track.isFavorite"
                class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                @click="toggleFav(track.id, $event)"
              />
              <Heart
                v-else
                class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                @click="toggleFav(track.id, $event)"
              />
            </div>

            <div class="flex-[2] min-w-0 pl-1">
              <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
                {{ track.title }}
              </span>
            </div>

            <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ track.artist }}</div>

            <div class="w-[70px] text-right shrink-0 hidden md:block text-[12px] font-mono text-text-muted tabular-nums">{{ formatFileSize(track.fileSize) }}</div>

            <div class="w-[56px] text-right shrink-0 hidden md:block text-[12px] font-mono text-text-muted tabular-nums">{{ track.duration }}</div>
          </div>

          <div v-if="playerStore.isLoadingFolderTracks && playerStore.folderTracks.length > 0" class="flex items-center justify-center py-4">
            <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
          </div>
        </div>
      </div>
    </div>

    <!-- Playlist Picker Popup -->
    <div
      v-if="pickerDirPath"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      @click.self="pickerDirPath = null"
    >
      <div class="bg-bg-canvas rounded-[10px] shadow-lg border border-border-color p-4 w-[260px]">
        <p class="text-[13px] text-text-primary font-medium mb-3">添加到歌单</p>
        <div class="space-y-1 max-h-[200px] overflow-y-auto">
          <button
            v-for="pl in playerStore.playlists"
            :key="pl.id"
            class="w-full text-left px-3 py-2 rounded-[6px] text-[13px] hover:bg-list-hover transition-colors-smooth"
            @click="addDirToPlaylist(pl.id)"
          >{{ pl.name }}</button>
        </div>
        <div v-if="playerStore.playlists.length === 0" class="text-[12px] text-text-muted text-center py-3">暂无歌单</div>
        <button
          class="mt-3 w-full text-center text-[12px] text-text-muted hover:text-text-primary transition-colors-smooth py-1"
          @click="pickerDirPath = null"
        >取消</button>
      </div>
    </div>
  </div>
</template>
