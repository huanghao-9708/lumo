<script setup lang="ts">
import { computed, watch } from 'vue';
import { Play, Shuffle, Loader2, Disc3, Heart, CheckSquare } from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import BatchActionBar from '../shared/BatchActionBar.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const props = defineProps<{
  albumId: number | null;
}>();

const playerStore = usePlayerStore();
// 极简模式：封面头图不挂载（DM-04）
const desktopMode = useDesktopModeStore();
const visualAllowed = computed(() => desktopMode.visualAllowed);

/** 曲目列表滚动位置记忆（同一专辑返回时还原） */
const listScrollEl = useScrollRestore(() => `album-detail:${props.albumId ?? 0}`);

/* ============ 批量选择（本视图一份实例；切换专辑自动退出） ============ */
const batch = useBatchSelect();
watch(() => props.albumId, () => batch.exit());
const isAllSelected = computed(() => batch.count > 0 && batch.count === tracks.value.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(tracks.value);
}

const album = computed(() => playerStore.currentAlbumDetails);

/** 是否已收藏该专辑 */
const isAlbumFavorited = computed(() =>
  props.albumId !== null && playerStore.favoriteAlbums.some(a => a.id === props.albumId)
);

/** 切换专辑收藏 */
function toggleAlbumFav() {
  if (props.albumId !== null) {
    playerStore.toggleFavoriteAlbum(props.albumId, !isAlbumFavorited.value);
  }
}

/** 封面 */
const coverSrc = useArtworkSrc(() => album.value?.cover_artwork_id ?? null);

/** 轨道列表 */
const tracks = computed<Track[]>(() => album.value?.tracks ?? []);
const isLoadingTracks = computed(() => {
  // 刚设了 activeAlbumId 但 watcher 还没拉取完
  return props.albumId !== null && !album.value;
});

/* ============ 统一列解析：专辑详情隐藏专辑列（本页即上下文） ============ */
const listContext = computed<TrackListContext>(() => ({ hidden: ['album'] }));
const { resolvedColumns, menuColumns, trailingExtraWidth } = useTrackColumns({
  containerRef: listScrollEl,
  context: listContext,
});

/** 元数据行文案 */
const trackCount = computed(() => tracks.value.length);
const totalDuration = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
});
const metaText = computed(() => {
  if (!album.value) return '';
  const parts: string[] = [];
  if (album.value.year) parts.push(String(album.value.year));
  parts.push(`${trackCount.value} TRACKS`);
  parts.push(totalDuration.value);
  return parts.join(' · ');
});

/** 当前播放判定 */
function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

/** 双击播放单曲 */
function playTrack(index: number) {
  playerStore.playAll(tracks.value, index);
}

/** 播放全部 */
function playAll() {
  if (tracks.value.length > 0) playerStore.playAll(tracks.value, 0);
}

/** 随机播放 */
function shufflePlay() {
  if (tracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * tracks.value.length);
  playerStore.playAll(tracks.value, idx);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- 无选中时占位 -->
    <div v-if="!albumId" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Disc3 class="w-10 h-10 text-text-disabled" />
      <p class="text-[13px]">选择一张专辑查看详情</p>
    </div>

    <!-- 加载中 -->
    <div v-else-if="isLoadingTracks" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载专辑…</span>
    </div>

    <template v-else-if="album">
      <!-- 专辑头部 -->
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <!-- 封面（1200 基线 152px）；极简模式不挂载（DM-04） -->
          <div v-if="visualAllowed" class="w-[152px] h-[152px] rounded-[12px] overflow-hidden flex-shrink-0 bg-bg-hover flex items-center justify-center shadow-lg ring-1 ring-black/5 dark:ring-white/10">
            <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
            <Disc3 v-else class="w-10 h-10 text-text-disabled" />
          </div>

          <!-- 标题 + 元数据 + 按钮 -->
          <div class="flex-1 min-w-0 pt-2">
            <div class="flex items-center gap-3 mb-1">
              <h1 class="text-(--text-page-title) font-bold text-text-primary tracking-tight leading-tight">{{ album.title }}</h1>
              <button
                class="flex-shrink-0 transition-colors-smooth"
                :class="isAlbumFavorited ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
                @click="toggleAlbumFav"
                :title="isAlbumFavorited ? '取消收藏' : '收藏专辑'"
              >
                <Heart class="w-[22px] h-[22px]" :class="isAlbumFavorited ? 'fill-current' : ''" />
              </button>
            </div>
            <p class="text-[14px] text-text-secondary mb-2">{{ album.artist }}</p>

            <!-- 元数据行（等宽 mono） -->
            <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mb-5">{{ metaText }}</p>

            <!-- 操作按钮 -->
            <div class="flex items-center gap-3">
              <button
                class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
                @click="playAll"
              >
                <Play class="w-[14px] h-[14px] fill-current" />
                播放全部
              </button>
              <button
                class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-primary flex items-center gap-2 hover:bg-list-hover transition-colors-smooth"
                @click="shufflePlay"
              >
                <Shuffle class="w-[14px] h-[14px]" />
                随机播放
              </button>
              <!-- 批量选择入口 -->
              <button
                class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium flex items-center gap-2 transition-colors-smooth"
                :class="batch.isActive ? 'bg-list-selected text-text-primary border-transparent' : 'text-text-primary hover:bg-list-hover'"
                @click="batch.isActive ? batch.exit() : batch.enter()"
              >
                <CheckSquare class="w-[14px] h-[14px]" />
                {{ batch.isActive ? '取消多选' : '多选' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 分割线 -->
      <div class="h-px bg-border-color mx-8"></div>

      <!-- 轨道列表 -->
      <div ref="listScrollEl" class="flex-1 overflow-y-auto px-8">
        <!-- 统一表头（右端含显示列菜单） -->
        <TrackListHeader :columns="resolvedColumns" :menu-columns="menuColumns" />

        <!-- 空列表 -->
        <div v-if="tracks.length === 0" class="flex flex-col items-center justify-center py-16 gap-3 text-text-muted">
          <span class="text-[12px]">该专辑暂无曲目</span>
        </div>

        <!-- 轨道行 -->
        <template v-else>
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
            @play="playTrack(index)"
            @toggle-select="batch.toggle(track)"
          />
        </template>

      </div>

      <!-- 批量操作条（多选态） -->
      <BatchActionBar
        v-if="batch.isActive"
        :selected-ids="[...batch.selectedIds]"
        :all-selected="isAllSelected"
        @exit="batch.exit()"
        @toggle-select-all="onToggleSelectAll"
      />
    </template>
  </div>
</template>
