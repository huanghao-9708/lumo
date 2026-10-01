<script setup lang="ts">
import { computed, watch } from 'vue';
import { Loader2, Zap } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import BatchActionBar from '../shared/BatchActionBar.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const playerStore = usePlayerStore();

/** 滚动位置记忆：每种智能歌单各记一份 */
const scrollEl = useScrollRestore(() => `smart-playlist:${playerStore.activeSmartPlaylistKind ?? ''}`);

/* ============ 批量选择（本视图一份实例） ============ */
const batch = useBatchSelect();
watch(() => playerStore.activeSmartPlaylistKind, () => batch.exit());
const isAllSelected = computed(() => tracks.value.length > 0 && tracks.value.every(track => batch.isSelected(track.id)));
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(tracks.value);
}

const tracks = computed(() => playerStore.smartPlaylistTracks);

const title = computed(() => {
  switch (playerStore.activeSmartPlaylistKind) {
    case 'most_played': return '播放最多';
    case 'recently_added': return '最近添加';
    case 'recently_played': return '最近播放';
    case 'never_played': return '未曾播放';
    default: return '智能歌单';
  }
});

/* ============ 统一列解析（容器宽度 + 用户列偏好） ============ */
const listContext = computed<TrackListContext>(() => ({ batchEntry: true }));
const { resolvedColumns, menuColumns, trailingExtraWidth } = useTrackColumns({
  containerRef: scrollEl,
  context: listContext,
});

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playSong(index: number) {
  playerStore.playQueue(tracks.value, index);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div ref="scrollEl" class="flex-1 overflow-y-auto px-8">
      <!-- 头部标识 -->
      <div class="py-6 flex items-center gap-4">
        <div class="w-16 h-16 rounded-xl bg-gradient-to-br from-brand-orange to-red-500 flex items-center justify-center shadow-lg">
          <Zap class="w-8 h-8 text-white fill-white" />
        </div>
        <div>
          <h1 class="text-(--text-page-title) font-bold tracking-tight text-text-primary">{{ title }}</h1>
          <p class="text-sm text-text-secondary mt-1">Smart Playlist</p>
        </div>
      </div>

      <!-- 统一表头（右端含批量选择入口与显示列菜单） -->
      <TrackListHeader
          :columns="resolvedColumns"
          :menu-columns="menuColumns"
          show-batch-entry
          :batch-active="batch.isActive"
          @toggle-batch="batch.isActive ? batch.exit() : batch.enter()"
        />

      <!-- 加载态 -->
      <div v-if="playerStore.isLoadingSmartPlaylist && tracks.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Zap class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">此智能歌单目前为空</span>
      </div>

      <!-- 列表 -->
      <div v-else>
        <TrackRow
          v-for="(track, index) in tracks"
          :key="track.id"
          :track="track"
          :columns="resolvedColumns"
          :index="index"
          :playing="isPlayingTrack(track.id)"
          :is-playing-now="playerStore.isPlaying"
          :batch-mode="batch.isActive"
          :selected="batch.isSelected(track.id)"
          :trailing-width="trailingExtraWidth"
          @play="playSong(index)"
          @toggle-select="batch.toggle(track)"
        />
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
</template>
