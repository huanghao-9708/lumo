<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play, Shuffle, Loader2, ListMusic, CheckSquare, Trash2 } from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import BatchActionBar from '../shared/BatchActionBar.vue';
import ConfirmDialog from '../shared/ConfirmDialog.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/** 曲目列表滚动位置记忆（同一歌单返回时还原） */
const listScrollEl = useScrollRestore(() => `playlist-detail:${playerStore.activePlaylistId ?? 0}`);

/* ============ 批量选择（本视图一份实例） ============ */
const batch = useBatchSelect();
const isAllSelected = computed(() => batch.count > 0 && batch.count === tracks.value.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(tracks.value);
}

const detail = computed(() => playerStore.currentPlaylistDetails);

/**
 * 歌单封面 = 歌单内第一首歌曲所属专辑的封面（后端已解析好 artwork id）。
 * 优先用 200x200 缩略图（列表 IPC 内联的 base64，零额外请求）；
 * 缩略图缺失时退回 artwork 协议按需拉取（详情页只有一张，不构成 N+1）。
 */
const artworkSrc = useArtworkSrc(
  () => detail.value?.cover_artwork_id ?? detail.value?.tracks?.[0]?.cover_artwork_id ?? null
);
const coverSrc = computed(() => detail.value?.cover_thumb || artworkSrc.value || '');

/* ============ 统一列解析（容器宽度 + 用户列偏好） ============ */
const listContext = computed<TrackListContext>(() => ({}));
const { resolvedColumns, menuColumns } = useTrackColumns({
  containerRef: listScrollEl,
  context: listContext,
});

/* ============ 删除歌单 ============ */
const showDeleteConfirm = ref(false);
const isDeleting = ref(false);

async function confirmDelete() {
  const playlistId = playerStore.activePlaylistId;
  if (!playlistId) return;
  const name = detail.value?.name ?? '歌单';
  isDeleting.value = true;
  try {
    await playerStore.deletePlaylist(playlistId);
    uiStore.showToast(`已删除歌单「${name}」`);
  } catch (e) {
    console.error('Failed to delete playlist:', e);
    uiStore.showToast('删除失败，请重试');
  } finally {
    isDeleting.value = false;
    showDeleteConfirm.value = false;
  }
}

const tracks = computed<Track[]>(() => detail.value?.tracks ?? []);
const isLoading = computed(() => detail.value?.isLoadingTracks ?? false);
const isLoaded = computed(() => !!detail.value && !detail.value.isLoadingTracks);

const trackCount = computed(() => tracks.value.length);
const totalDuration = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
});
const metaText = computed(() => {
  if (!detail.value) return '';
  return `${trackCount.value} TRACKS · ${totalDuration.value}`;
});

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playAll() {
  if (tracks.value.length > 0) playerStore.playAll(tracks.value, 0);
}

function shufflePlay() {
  if (tracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * tracks.value.length);
  playerStore.playAll(tracks.value, idx);
}

function playTrack(index: number) {
  playerStore.playAll(tracks.value, index);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- 加载中 -->
    <div v-if="isLoading && tracks.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载歌单…</span>
    </div>

    <template v-else-if="isLoaded || tracks.length > 0">
      <!-- 歌单头部 -->
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <!-- 封面：歌单内第一首歌曲的专辑封面（无封面时退回图标占位）；1200 基线 152px -->
          <div class="w-[152px] h-[152px] rounded-[10px] overflow-hidden flex-shrink-0 bg-bg-hover flex items-center justify-center">
            <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
            <ListMusic v-else class="w-12 h-12 text-text-disabled" />
          </div>

          <!-- 标题 + 元数据 + 按钮 -->
          <div class="flex-1 min-w-0 pt-2">
            <h1 class="text-(--text-page-title) font-bold text-text-primary tracking-tight leading-tight mb-1">{{ detail?.name || '歌单' }}</h1>

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

              <!-- 删除歌单（二次确认） -->
              <button
                class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-secondary flex items-center gap-2 transition-colors-smooth hover:bg-list-hover hover:text-status-error hover:border-status-error/40"
                title="删除这个歌单"
                @click="showDeleteConfirm = true"
              >
                <Trash2 class="w-[14px] h-[14px]" />
                删除歌单
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
          <span class="text-[12px]">该歌单暂无曲目</span>
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

  <!-- 删除歌单二次确认 -->
  <ConfirmDialog
    v-if="showDeleteConfirm"
    title="删除歌单"
    :message="`确定删除歌单「${detail?.name ?? '歌单'}」吗？歌单里的歌曲不会从曲库中移除。此操作不可撤销。`"
    confirm-text="删除"
    danger
    :busy="isDeleting"
    @confirm="confirmDelete"
    @cancel="showDeleteConfirm = false"
  />
</template>
