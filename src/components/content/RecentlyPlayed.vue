<script setup lang="ts">
import { computed } from 'vue';
import { Loader2, Music, ListChecks } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import BatchActionBar from '../shared/BatchActionBar.vue';
import FooterStatus from '../shared/FooterStatus.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const props = defineProps<{
  /** 内容区搜索框传入的过滤词：内存过滤当前列表 */
  filterQuery?: string;
}>();

const playerStore = usePlayerStore();

/** 滚动位置记忆：进详情返回后还原（key 带过滤词，过滤后不串位） */
const scrollEl = useScrollRestore(() => `recently-played:${props.filterQuery ?? ''}`);

/* ============ 批量选择（本视图一份实例） ============ */
const batch = useBatchSelect();
const isAllSelected = computed(() => batch.count > 0 && batch.count === visibleTracks.value.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(visibleTracks.value);
}

const tracks = computed(() => playerStore.tracks);

/** 内存过滤（最近播放一次性全量加载，过滤覆盖完整） */
const visibleTracks = computed(() => {
  const q = (props.filterQuery ?? '').trim().toLowerCase();
  if (!q) return tracks.value;
  return tracks.value.filter(t =>
    t.title.toLowerCase().includes(q) ||
    t.artist.toLowerCase().includes(q) ||
    t.album.toLowerCase().includes(q)
  );
});

/* ============ 统一列解析：最近播放保留播放时间（页面专属列，不被列设置移除） ============ */
const listContext = computed<TrackListContext>(() => ({ extra: ['playedAt'] }));
const { resolvedColumns, menuColumns } = useTrackColumns({
  containerRef: scrollEl,
  context: listContext,
});

const totalDurationText = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const days = Math.floor(totalSec / 86400);
  const hours = Math.floor((totalSec % 86400) / 3600);
  const mins = Math.floor((totalSec % 3600) / 60);
  const parts: string[] = [];
  if (days > 0) parts.push(`${days} 天`);
  if (hours > 0) parts.push(`${hours} 小时`);
  parts.push(`${mins} 分钟`);
  return `总计 ${parts.join(' ')}`;
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
      <TrackListHeader :columns="resolvedColumns" :menu-columns="menuColumns">
        <template #trailing>
          <button
            class="ml-2 w-8 shrink-0 flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
            :class="batch.isActive ? 'text-brand-orange' : ''"
            :title="batch.isActive ? '退出多选' : '多选歌曲'"
            @click="batch.isActive ? batch.exit() : batch.enter()"
          >
            <ListChecks class="w-[14px] h-[14px]" />
          </button>
        </template>
      </TrackListHeader>

      <!-- 加载态 -->
      <div v-if="playerStore.isLoadingTracks && tracks.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Music class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">还没有播放记录</span>
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
          @play="playSong(index)"
          @toggle-select="batch.toggle(track)"
        />
      </div>

      <!-- 批量操作条（多选态） -->
      <BatchActionBar
        v-if="batch.isActive"
        :selected-ids="[...batch.selectedIds]"
        :all-selected="isAllSelected"
        @exit="batch.exit()"
        @toggle-select-all="onToggleSelectAll"
      />

      <!-- Footer -->
    </div>

    <!-- Footer Status（固定在底部） -->
    <FooterStatus v-if="tracks.length > 0" :count="`${tracks.length.toLocaleString()} 首歌曲`" :hint="filterQuery?.trim() ? `过滤后 ${visibleTracks.length} 首` : totalDurationText" />
  </div>
</template>
