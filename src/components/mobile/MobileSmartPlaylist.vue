<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play, Shuffle, Loader2, Zap } from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useScrollRestore } from '../../composables/useScrollRestore';
import MobileSongRow from './MobileSongRow.vue';
import ActionSheet, { type ActionItem } from './ActionSheet.vue';

/**
 * 移动端智能歌单页面（播放最多 / 最近添加 / 最近播放 / 未曾播放）。
 */
const playerStore = usePlayerStore();

const tracks = computed(() => playerStore.smartPlaylistTracks);
const isLoading = computed(() => playerStore.isLoadingSmartPlaylist);

const title = computed(() => {
  switch (playerStore.activeSmartPlaylistKind) {
    case 'most_played': return '播放最多';
    case 'recently_added': return '最近添加';
    case 'recently_played': return '最近播放';
    case 'never_played': return '未曾播放';
    default: return '智能歌单';
  }
});

const subtitle = computed(() => {
  switch (playerStore.activeSmartPlaylistKind) {
    case 'most_played': return '听歌频次最高的 Top 100 曲目';
    case 'recently_added': return '最新加入曲库的歌曲';
    case 'recently_played': return '按播放时间倒序排列';
    case 'never_played': return '曲库中尚未听过的宝藏歌曲';
    default: return '基于播放数据自动生成';
  }
});

const scrollEl = useScrollRestore(() => `m-smart-playlist:${playerStore.activeSmartPlaylistKind ?? ''}`);

function playAll() {
  if (tracks.value.length > 0) {
    playerStore.playQueue(tracks.value, 0);
  }
}

function shufflePlay() {
  if (tracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * tracks.value.length);
  playerStore.playQueue(tracks.value, idx);
}

function playTrack(index: number) {
  playerStore.playQueue(tracks.value, index);
}

function isCurrentTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function toggleFav(trackId: number) {
  playerStore.toggleFavorite(trackId);
}

/* ============ ActionSheet：长按菜单 ============ */
const sheetVisible = ref(false);
const sheetTrack = ref<Track | null>(null);

const sheetActions = computed<ActionItem[]>(() => {
  const track = sheetTrack.value;
  if (!track) return [];
  return [
    {
      label: track.isFavorite ? '取消收藏' : '收藏',
      onClick: () => playerStore.toggleFavorite(track.id),
    },
    ...(track.albumId ? [{
      label: '查看专辑',
      onClick: () => {
        playerStore.activeAlbumId = track.albumId;
        playerStore.activeLibraryTab = '专辑';
      },
    }] : []),
    ...(track.artistId ? [{
      label: '查看艺术家',
      onClick: () => {
        playerStore.activeArtistId = track.artistId;
        playerStore.activeLibraryTab = '艺术家';
      },
    }] : []),
  ];
});

function onTrackLongPress(trackId: number) {
  const found = tracks.value.find(t => t.id === trackId);
  if (found) {
    sheetTrack.value = found;
    sheetVisible.value = true;
  }
}

function onSheetClose() {
  sheetVisible.value = false;
  sheetTrack.value = null;
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden">
    <!-- 加载中 -->
    <div v-if="isLoading && tracks.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
      <span class="text-[12px]">加载智能歌单…</span>
    </div>

    <!-- 空列表 -->
    <div v-else-if="tracks.length === 0" class="flex-1 flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
      <Zap class="w-8 h-8 text-text-disabled" aria-hidden="true" />
      <span class="text-[13px]">暂无歌曲数据</span>
    </div>

    <!-- 列表展示 -->
    <div v-else ref="scrollEl" class="flex-1 overflow-y-auto">
      <div class="flex flex-col items-center px-6 pt-6 pb-2">
        <div class="w-16 h-16 rounded-[14px] bg-gradient-to-br from-brand-orange to-red-500 flex items-center justify-center shadow-lg mb-3">
          <Zap class="w-8 h-8 text-white fill-white" />
        </div>
        <h1 class="text-[20px] font-bold text-text-primary tracking-tight leading-tight text-center mb-1">
          {{ title }}
        </h1>
        <p class="text-[13px] text-text-secondary text-center mb-1">
          {{ subtitle }}
        </p>
        <p class="text-[11px] font-mono uppercase tracking-wider text-text-muted mb-4">
          {{ tracks.length }} 首歌曲
        </p>

        <!-- 播放控制行 -->
        <div class="flex items-center gap-3 w-full max-w-[280px]">
          <button
            class="flex-1 h-11 rounded-full bg-text-primary text-bg-canvas text-[14px] font-medium flex items-center justify-center gap-2 active:opacity-80 transition-opacity"
            @click="playAll"
          >
            <Play class="w-[16px] h-[16px] fill-current" aria-hidden="true" />
            播放全部
          </button>
          <button
            class="flex-1 h-11 rounded-full border border-border-solid text-[14px] font-medium text-text-primary flex items-center justify-center gap-2 active:bg-list-hover transition-colors-smooth"
            @click="shufflePlay"
          >
            <Shuffle class="w-[16px] h-[16px]" aria-hidden="true" />
            随机播放
          </button>
        </div>
      </div>

      <div class="h-px bg-border-color mx-4 mt-4"></div>

      <div class="py-1 pb-6">
        <MobileSongRow
          v-for="(track, index) in tracks"
          :key="track.id"
          :track="track"
          :index="index"
          :is-playing="playerStore.isPlaying"
          :is-current="isCurrentTrack(track.id)"
          @play="playTrack"
          @toggle-fav="toggleFav"
          @long-press="onTrackLongPress"
        />
      </div>
    </div>

    <!-- ActionSheet 菜单 -->
    <ActionSheet
      :visible="sheetVisible"
      :actions="sheetActions"
      @close="onSheetClose"
    />
  </div>
</template>
