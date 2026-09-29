<script setup lang="ts">
import { computed } from 'vue';
import { Loader2, Heart } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import BatchActionBar from '../shared/BatchActionBar.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const props = defineProps<{
  /** 内容区搜索框传入的过滤词：内存过滤当前列表（收藏全量在内存，过滤覆盖完整） */
  filterQuery?: string;
}>();

const playerStore = usePlayerStore();

/** 滚动位置记忆：进详情返回后还原（key 带过滤词，过滤后不串位） */
const scrollEl = useScrollRestore(() => `favorites:${props.filterQuery ?? ''}`);

/* ============ 批量选择（本视图一份实例；hide-favorite-action 避免"把喜欢的加进喜欢"） ============ */
const batch = useBatchSelect();
const isAllSelected = computed(() => batch.count > 0 && batch.count === visibleTracks.value.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(visibleTracks.value);
}

const tracks = computed(() => playerStore.tracks);

/** 内存过滤 */
const visibleTracks = computed(() => {
  const q = (props.filterQuery ?? '').trim().toLowerCase();
  if (!q) return tracks.value;
  return tracks.value.filter(t =>
    t.title.toLowerCase().includes(q) ||
    t.artist.toLowerCase().includes(q) ||
    t.album.toLowerCase().includes(q)
  );
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
  // 播放过滤后的列表（与所见一致）
  playerStore.playQueue(visibleTracks.value, index);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div ref="scrollEl" class="flex-1 overflow-y-auto px-8">
      <!-- 统一表头（右端含批量选择入口与显示列菜单） -->
      <TrackListHeader
          :columns="resolvedColumns"
          :menu-columns="menuColumns"
          show-batch-entry
          :batch-active="batch.isActive"
          @toggle-batch="batch.isActive ? batch.exit() : batch.enter()"
        />

      <!-- 加载态 -->
      <div v-if="playerStore.isLoadingTracks && tracks.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Heart class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">还没有收藏的歌曲</span>
        <p class="text-[11px] text-text-muted/70">在歌曲上点击心形图标即可收藏</p>
      </div>

      <!-- 过滤无结果 -->
      <div v-else-if="visibleTracks.length === 0" class="flex flex-col items-center justify-center py-20 text-text-muted">
        <span class="text-[12px]">没有匹配的歌曲</span>
      </div>

      <!-- 列表 -->
      <div v-else>
        <TrackRow
          v-for="(track, index) in visibleTracks"
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

      <!-- 批量操作条（多选态；hide-favorite-action 避免自我包含） -->
      <BatchActionBar
        v-if="batch.isActive"
        :selected-ids="[...batch.selectedIds]"
        :all-selected="isAllSelected"
        hide-favorite-action
        @exit="batch.exit()"
        @toggle-select-all="onToggleSelectAll"
      />

      <!-- Footer -->
    </div>

  </div>
</template>
