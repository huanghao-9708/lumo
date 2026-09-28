<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play, Shuffle, Loader2, ListMusic } from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useScrollRestore } from '../../composables/useScrollRestore';
import MobileSongRow from './MobileSongRow.vue';
import ActionSheet, { type ActionItem } from './ActionSheet.vue';

/**
 * 移动端歌单详情页。
 * 独立组件化（从 MobileContentView 解耦并增强）。
 */
const playerStore = usePlayerStore();

const playlistDetail = computed(() => playerStore.currentPlaylistDetails);
const playlistTracks = computed(() => playlistDetail.value?.tracks ?? []);
const isLoadingPlaylist = computed(() => playerStore.activePlaylistId !== null && !playlistDetail.value);

const scrollEl = useScrollRestore(() => `m-playlist-detail:${playerStore.activePlaylistId ?? 0}`);

// 歌单封面：使用歌单中第一首有封面的歌曲封面
const firstCoverId = computed(() => {
  if (playlistDetail.value?.cover_artwork_id) return playlistDetail.value.cover_artwork_id;
  const firstWithCover = playlistTracks.value.find(t => t.cover_artwork_id);
  return firstWithCover?.cover_artwork_id ?? null;
});
const playlistCoverSrc = useArtworkSrc(() => firstCoverId.value);

function playAll() {
  if (playlistTracks.value.length > 0) {
    playerStore.playAll(playlistTracks.value, 0);
  }
}

function shufflePlay() {
  if (playlistTracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * playlistTracks.value.length);
  playerStore.playAll(playlistTracks.value, idx);
}

function playTrack(index: number) {
  playerStore.playAll(playlistTracks.value, index);
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
  const found = playlistTracks.value.find(t => t.id === trackId);
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
    <!-- 加载态 -->
    <div v-if="isLoadingPlaylist" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
      <span class="text-[12px]">加载歌单…</span>
    </div>

    <template v-else-if="playlistDetail">
      <div ref="scrollEl" class="flex-1 overflow-y-auto">
        <!-- 头部信息 -->
        <div class="flex flex-col items-center px-6 pt-6 pb-2">
          <div class="relative w-[50%] max-w-[220px] aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-4 flex items-center justify-center shadow-md">
            <img v-if="playlistCoverSrc" :src="playlistCoverSrc" class="w-full h-full object-cover" alt="cover" />
            <ListMusic v-else class="w-12 h-12 text-text-disabled" aria-hidden="true" />
          </div>

          <h1 class="text-[20px] font-bold text-text-primary tracking-tight leading-tight text-center mb-1">
            {{ playlistDetail.name }}
          </h1>
          <p v-if="playlistDetail.description" class="text-[13px] text-text-secondary text-center mb-2 px-4 max-w-[320px]">
            {{ playlistDetail.description }}
          </p>
          <p class="text-text-muted font-mono uppercase tracking-wider mb-4" style="font-size: var(--text-11);">
            {{ playlistTracks.length }} 首歌曲
          </p>

          <!-- 操作按钮行 -->
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

        <!-- 曲目列表 -->
        <div v-if="playlistTracks.length === 0" class="flex flex-col items-center justify-center py-12 gap-3 text-text-muted">
          <span class="text-[13px]">暂无歌曲，可在曲库中长按歌曲添加到歌单</span>
        </div>
        <div v-else class="py-1 pb-6">
          <MobileSongRow
            v-for="(track, index) in playlistTracks"
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
    </template>

    <!-- ActionSheet 菜单 -->
    <ActionSheet
      :visible="sheetVisible"
      :actions="sheetActions"
      @close="onSheetClose"
    />
  </div>
</template>
