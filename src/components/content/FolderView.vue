<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue';
import {
  Folder, ChevronRight, ChevronDown, Loader2, Music, Plus
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import type { DirectoryNodeDTO } from '../../api/types';
import { libraryGetFolderChildren } from '../../api/library';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import BatchActionBar from '../shared/BatchActionBar.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 批量选择（本视图一份实例；切换文件夹自动退出） ============ */
const batch = useBatchSelect();
watch(() => playerStore.selectedTreePath, () => batch.exit());
const isAllSelected = computed(() => playerStore.folderTracks.length > 0 && playerStore.folderTracks.every(track => batch.isSelected(track.id)));
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(playerStore.folderTracks);
}

// Keep navigation in the store, but release loaded directory objects on unmount.
const selectedSourceId = computed({
  get: () => playerStore.folderBrowseContext.sourceId,
  set: (id: number | null) => { playerStore.folderBrowseContext.sourceId = id; },
});
watch(selectedSourceId, () => batch.exit());
const loadedChildren = ref<Record<string, DirectoryNodeDTO[]>>({});
const expandedPaths = computed({
  get: () => playerStore.folderBrowseContext.expandedPaths,
  set: (paths: Record<string, boolean>) => { playerStore.folderBrowseContext.expandedPaths = paths; },
});
const loadingPaths = ref<Record<string, boolean>>({});
const pickerDirPath = ref<string | null>(null);
let treeRevision = 0;
let disposed = false;
onBeforeUnmount(() => { disposed = true; treeRevision++; });

/** 滚动位置记忆：目录树 / 曲目列表各记一份（曲目列表按目录分别记） */
const treeScrollEl = useScrollRestore(() => `folder-tree:${selectedSourceId.value ?? 0}`);
const tracksScrollEl = useScrollRestore(
  () => `folder-tracks:${selectedSourceId.value ?? 0}:${playerStore.selectedTreePath}`
);

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
  const sourceId = selectedSourceId.value;
  if (sourceId == null || disposed || playerStore.pendingBrowseRestore) return;
  if (loadedChildren.value[parentPath] || loadingPaths.value[parentPath]) return;
  const revision = treeRevision;
  const isCurrent = () => !disposed && revision === treeRevision && selectedSourceId.value === sourceId;
  loadingPaths.value = { ...loadingPaths.value, [parentPath]: true };
  try {
    const res = await libraryGetFolderChildren(sourceId, parentPath || undefined);
    if (isCurrent()) loadedChildren.value = { ...loadedChildren.value, [parentPath]: res.children };
  } catch (error) {
    if (isCurrent()) uiStore.showToast('目录加载失败，请重新展开目录', 'error');
  } finally {
    if (isCurrent()) {
      const nextLoading = { ...loadingPaths.value };
      delete nextLoading[parentPath];
      loadingPaths.value = nextLoading;
    }
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

/* ============ 统一列解析：文件夹视图钉住「大小」列（文件夹浏览的业务字段） ============ */
const listContext = computed<TrackListContext>(() => ({ pinned: ['fileSize'], batchEntry: true }));
const { resolvedColumns, menuColumns, trailingExtraWidth } = useTrackColumns({
  containerRef: tracksScrollEl,
  context: listContext,
});

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playTrack(index: number) {
  if (playerStore.folderTracks.length > 0) {
    playerStore.playAll(playerStore.folderTracks, index);
  }
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

watch(selectedSourceId, (id, oldId) => {
  if (id !== oldId) {
    treeRevision++;
    loadedChildren.value = {};
    loadingPaths.value = {};
    if (playerStore.pendingBrowseRestore) return;
    expandedPaths.value = {};
    playerStore.clearFolderTrackSelection();
    if (id != null) void loadChildrenForPath('');
  }
}, { flush: 'sync' });

watch([sources, () => playerStore.pendingBrowseRestore], ([list, pending]) => {
  if (pending || disposed) return;
  if (!list.some(source => source.id === selectedSourceId.value)) selectedSourceId.value = list[0]?.id ?? null;
  if (selectedSourceId.value == null) return;
  void loadChildrenForPath('');
  for (const [path, expanded] of Object.entries(expandedPaths.value)) {
    if (expanded) void loadChildrenForPath(path);
  }
}, { immediate: true });

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

      <div ref="treeScrollEl" class="flex-1 overflow-y-auto px-2 pb-4 space-y-[2px]">
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

      <!-- 统一表头（右端含批量选择入口与显示列菜单） -->
      <div class="px-6 flex-shrink-0">
        <TrackListHeader
          :columns="resolvedColumns"
          :menu-columns="menuColumns"
          show-batch-entry
          :batch-active="batch.isActive"
          @toggle-batch="batch.isActive ? batch.exit() : batch.enter()"
        />
      </div>

      <!-- Track rows -->
      <div
        ref="tracksScrollEl"
        class="flex-1 overflow-y-auto px-6"
        @scroll="(e) => {
          const el = e.target as HTMLElement;
          if (el.scrollTop + el.clientHeight >= el.scrollHeight - 400) {
            if (!playerStore.pendingBrowseRestore && selectedSourceId != null && playerStore.selectedTreePath != null && !playerStore.isLoadingFolderTracks && playerStore.hasMoreFolderTracks) {
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
          <TrackRow
            v-for="(track, index) in playerStore.folderTracks"
            :key="track.id"
            :track="track"
            :columns="resolvedColumns"
            :index="index"
            :playing="isPlayingTrack(track.id)"
            :is-playing-now="playerStore.isPlaying"
            :batch-mode="batch.isActive"
            :selected="batch.isSelected(track.id)"
            :trailing-width="trailingExtraWidth"
            @play="playTrack(index)"
            @toggle-select="batch.toggle(track)"
          />

          <div v-if="playerStore.isLoadingFolderTracks && playerStore.folderTracks.length > 0" class="flex items-center justify-center py-4">
            <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
          </div>
        </div>

      </div>

      <!-- 批量操作条独立于滚动区，长列表中始终可见 -->
      <BatchActionBar
        v-if="batch.isActive"
        :selected-ids="[...batch.selectedIds]"
        :all-selected="isAllSelected"
        @exit="batch.exit()"
        @toggle-select-all="onToggleSelectAll"
      />
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
