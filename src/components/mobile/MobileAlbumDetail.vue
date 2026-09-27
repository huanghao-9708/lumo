<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play, Shuffle, Loader2, Heart, Disc3 } from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { useScrollRestore } from '../../composables/useScrollRestore';
import MobileSongRow from './MobileSongRow.vue';
import ActionSheet, { type ActionItem } from './ActionSheet.vue';

/**
 * 移动端专辑详情页。
 * 独立组件化（从 MobileContentView 解耦）。
 */
const playerStore = usePlayerStore();

const album = computed(() => playerStore.currentAlbumDetails);
const albumTracks = computed(() => album.value?.tracks ?? []);
const isLoadingAlbum = computed(() => playerStore.activeAlbumId !== null && !album.value);

const scrollEl = useScrollRestore(() => `m-album-detail:${playerStore.activeAlbumId ?? 0}`);
const albumCoverSrc = useArtworkSrc(() => album.value?.cover_artwork_id ?? null);

function albumMetaText(): string {
  if (!album.value) return '';
  const parts: string[] = [];
  if (album.value.year) parts.push(String(album.value.year));
  parts.push(`${albumTracks.value.length} 首歌曲`);
  const totalSec = albumTracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const m = Math.floor(totalSec / 60);
  if (m > 0) parts.push(`${m} 分钟`);
  return parts.join(' · ');
}

const isAlbumFav = computed(() =>
  playerStore.favoriteAlbums.some(a => a.id === playerStore.activeAlbumId)
);

function toggleAlbumFav() {
  if (playerStore.activeAlbumId !== null) {
    playerStore.toggleFavoriteAlbum(playerStore.activeAlbumId, !isAlbumFav.value);
  }
}

function playAlbumAll() {
  if (albumTracks.value.length > 0) {
    playerStore.playAll(albumTracks.value, 0);
  }
}

function shuffleAlbum() {
  if (albumTracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * albumTracks.value.length);
  playerStore.playAll(albumTracks.value, idx);
}

function playAlbumTrack(index: number) {
  playerStore.playAll(albumTracks.value, index);
}

function isAlbumTrackPlaying(trackId: number): boolean {
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
    ...(track.artistId ? [{
      label: '查看艺术家',
      onClick: () => {
        playerStore.navigateToArtist(track.artistId!);
      },
    }] : []),
  ];
});

function onTrackLongPress(trackId: number) {
  const found = albumTracks.value.find(t => t.id === trackId);
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
    <div v-if="isLoadingAlbum" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
      <span class="text-[12px]">加载专辑…</span>
    </div>

    <template v-else-if="album">
      <div ref="scrollEl" class="flex-1 overflow-y-auto">
        <!-- 专辑头部 -->
        <div class="flex flex-col items-center px-6 pt-6 pb-2">
          <div class="relative w-[60%] max-w-[260px] aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-4 shadow-md">
            <img v-if="albumCoverSrc" :src="albumCoverSrc" class="w-full h-full object-cover" alt="cover" />
            <Disc3 v-else class="w-10 h-10 text-text-disabled absolute inset-0 m-auto" aria-hidden="true" />
          </div>
          <h1 class="text-[20px] font-bold text-text-primary tracking-tight leading-tight text-center mb-1">
            {{ album.title }}
          </h1>
          <p class="text-[14px] text-text-secondary mb-1">{{ album.artist }}</p>
          <p class="text-[11px] font-mono uppercase tracking-wider text-text-muted mb-4">
            {{ albumMetaText() }}
          </p>
          <div class="flex items-center gap-3 w-full max-w-[280px]">
            <button
              class="flex-1 h-11 rounded-full bg-text-primary text-bg-canvas text-[14px] font-medium flex items-center justify-center gap-2 active:opacity-80 transition-opacity"
              @click="playAlbumAll"
            >
              <Play class="w-[16px] h-[16px] fill-current" aria-hidden="true" />
              播放全部
            </button>
            <button
              class="flex-1 h-11 rounded-full border border-border-solid text-[14px] font-medium text-text-primary flex items-center justify-center gap-2 active:bg-list-hover transition-colors-smooth"
              @click="shuffleAlbum"
            >
              <Shuffle class="w-[16px] h-[16px]" aria-hidden="true" />
              随机播放
            </button>
            <button
              class="w-11 h-11 rounded-full flex items-center justify-center border border-border-solid transition-colors-smooth active:bg-list-hover flex-shrink-0"
              :class="isAlbumFav ? 'text-brand-orange' : 'text-text-muted'"
              :aria-label="isAlbumFav ? '取消收藏' : '收藏专辑'"
              @click="toggleAlbumFav"
            >
              <Heart class="w-[20px] h-[20px]" :class="isAlbumFav ? 'fill-current' : ''" aria-hidden="true" />
            </button>
          </div>
        </div>

        <div class="h-px bg-border-color mx-4 mt-4"></div>

        <!-- 曲目列表 -->
        <div v-if="albumTracks.length === 0" class="flex flex-col items-center justify-center py-12 gap-3 text-text-muted">
          <span class="text-[13px]">该专辑暂无曲目</span>
        </div>
        <div v-else class="py-1 pb-6">
          <MobileSongRow
            v-for="(track, index) in albumTracks"
            :key="track.id"
            :track="track"
            :index="index"
            :is-playing="playerStore.isPlaying"
            :is-current="isAlbumTrackPlaying(track.id)"
            @play="playAlbumTrack"
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
