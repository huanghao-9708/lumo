<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  Folder, ChevronRight, ChevronDown, Play, Loader2, Music, MoreHorizontal, Heart,
} from 'lucide-vue-next';
import { usePlayerStore, type FolderEntry } from '../../stores/player';

const playerStore = usePlayerStore();

const selectedSourceId = ref<number | null>(null);
const expandedPaths = ref<Set<string>>(new Set());
const selectedFolderPath = ref<string>('');
const breadcrumbs = ref<string[]>([]);

const sources = computed(() => playerStore.localSources);
const entries = computed(() => playerStore.currentFolderContents);
const folderTotalCount = computed(() => playerStore.folderTotalCount);

function selectSource(sourceId: number) {
  selectedSourceId.value = sourceId;
  selectedFolderPath.value = '';
  breadcrumbs.value = [];
  expandedPaths.value = new Set();
  playerStore.fetchFolderContents(sourceId, '');
}

function toggleFolder(entry: FolderEntry) {
  if (!selectedSourceId.value) return;
  const path = entry.path;
  const set = new Set(expandedPaths.value);
  if (set.has(path)) {
    set.delete(path);
  } else {
    set.add(path);
    selectedFolderPath.value = path;
    breadcrumbs.value = path.split('\\').filter(Boolean);
    playerStore.fetchFolderContents(selectedSourceId.value, path);
  }
  expandedPaths.value = set;
}

function openFolder(path: string) {
  if (!selectedSourceId.value) return;
  selectedFolderPath.value = path;
  breadcrumbs.value = path.split('\\').filter(Boolean);
  const set = new Set(expandedPaths.value);
  set.add(path);
  expandedPaths.value = set;
  playerStore.fetchFolderContents(selectedSourceId.value, path);
}

function isExpanded(path: string): boolean {
  return expandedPaths.value.has(path);
}

function isSelected(path: string): boolean {
  return selectedFolderPath.value === path;
}

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playTrack(index: number) {
  const audioTracks = entries.value
    .filter(e => !e.is_dir && e.track)
    .map(e => e.track!);
  if (audioTracks.length > 0) {
    playerStore.playAll(audioTracks, index);
  }
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

const audioEntries = computed(() => entries.value.filter(e => !e.is_dir && e.track));
const isRootEmpty = computed(() => !selectedSourceId.value);

watch(sources, (list) => {
  if (list.length > 0 && !selectedSourceId.value) {
    selectSource(list[0].id);
  }
}, { immediate: true });
</script>

<template>
  <div class="flex-1 flex bg-bg-content overflow-hidden select-none min-w-0">

    <div class="w-[280px] flex-shrink-0 border-r border-border-color flex flex-col overflow-hidden">
      <div class="px-5 pt-6 pb-3 text-[10px] font-semibold text-text-muted uppercase tracking-widest flex-shrink-0">文件浏览器</div>

      <div class="px-4 pb-2 flex-shrink-0">
        <select
          v-model="selectedSourceId"
          @change="selectedSourceId && selectSource(selectedSourceId)"
          class="w-full h-[32px] px-2 text-[12px] bg-bg-canvas border border-border-color rounded-[6px] text-text-primary"
        >
          <option v-for="s in sources" :key="s.id" :value="s.id">{{ s.name }}</option>
        </select>
      </div>

      <div class="flex-1 overflow-y-auto px-2 pb-4">
        <div v-if="isRootEmpty" class="flex flex-col items-center justify-center py-16 text-text-muted">
          <p class="text-[12px]">暂无数据源</p>
        </div>

        <div v-else class="space-y-[2px]">
          <template v-for="entry in entries" :key="entry.path">
            <div v-if="entry.is_dir">
              <div
                class="flex items-center px-2 py-[6px] rounded-[6px] cursor-pointer transition-colors-smooth group"
                :class="isSelected(entry.path) ? 'bg-list-selected' : 'hover:bg-list-hover'"
                @click="toggleFolder(entry)"
              >
                <ChevronRight class="w-[14px] h-[14px] text-text-muted flex-shrink-0 transition-transform" :class="isExpanded(entry.path) ? 'rotate-90' : ''" />
                <Folder class="w-[16px] h-[16px] mx-2 text-text-muted flex-shrink-0" />
                <span class="text-[13px] text-text-primary truncate flex-1">{{ entry.name }}</span>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>

    <div class="flex-1 flex flex-col overflow-hidden">
      <div v-if="!selectedSourceId" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
        <Folder class="w-10 h-10 text-text-disabled" />
        <p class="text-[13px]">选择一个文件夹查看内容</p>
      </div>

      <template v-else>
        <div class="px-6 pt-4 pb-2 flex items-center gap-1 text-[11px] text-text-muted font-mono flex-shrink-0">
          <button class="hover:text-text-primary transition-colors-smooth" @click="selectSource(selectedSourceId!)">{{ sources.find(s => s.id === selectedSourceId)?.name }}</button>
          <template v-for="(crumb, i) in breadcrumbs" :key="i">
            <span class="mx-1">/</span>
            <button
              class="hover:text-text-primary transition-colors-smooth"
              @click="openFolder(breadcrumbs.slice(0, i + 1).join('\\'))"
            >{{ crumb }}</button>
          </template>
        </div>

        <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider px-6 py-2 border-b border-border-color flex-shrink-0">
          <div class="w-10 text-center shrink-0">#</div>
          <div class="w-8 shrink-0"></div>
          <div class="flex-[2] min-w-0 pl-1">标题</div>
          <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
          <div class="w-[70px] text-right shrink-0 hidden md:block">大小</div>
          <div class="w-[56px] text-right shrink-0 hidden md:block">时长</div>
          <div class="w-8 shrink-0"></div>
        </div>

        <div class="flex-1 overflow-y-auto px-6">
          <div v-if="playerStore.isLoadingFolderContents && audioEntries.length === 0" class="flex items-center justify-center py-16">
            <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
          </div>

          <div v-else-if="audioEntries.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <Music class="w-6 h-6 text-text-disabled mb-2" />
            <span class="text-[12px]">该文件夹没有可播放的歌曲</span>
          </div>

          <div v-else>
            <div
              v-for="(entry, index) in audioEntries"
              :key="entry.track!.id"
              class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
              style="height: 40px;"
              :class="{ 'playing-row bg-list-selected': isPlayingTrack(entry.track!.id) }"
              @dblclick="playTrack(index)"
            >
              <div class="w-10 text-center shrink-0 text-[12px] font-mono">
                <span v-if="isPlayingTrack(entry.track!.id)" class="text-brand-orange inline-flex items-center justify-center">
                  <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
                  <Play v-else class="w-[12px] h-[12px] fill-current" />
                </span>
                <template v-else>
                  <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
                  <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
                </template>
              </div>

              <div class="w-8 shrink-0 flex items-center justify-center">
                <Heart
                  v-if="entry.track!.isFavorite"
                  class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                  @click="toggleFav(entry.track!.id, $event)"
                />
                <Heart
                  v-else
                  class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                  @click="toggleFav(entry.track!.id, $event)"
                />
              </div>

              <div class="flex-[2] min-w-0 pl-1">
                <span class="text-[13px] truncate block" :class="isPlayingTrack(entry.track!.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
                  {{ entry.track!.title }}
                </span>
              </div>

              <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ entry.track!.artist }}</div>

              <div class="w-[70px] text-right shrink-0 hidden md:block text-[12px] font-mono text-text-muted tabular-nums">--</div>

              <div class="w-[56px] text-right shrink-0 hidden md:block text-[12px] font-mono text-text-muted tabular-nums">{{ entry.track!.duration }}</div>

              <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                <MoreHorizontal class="w-4 h-4 text-text-muted" />
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
