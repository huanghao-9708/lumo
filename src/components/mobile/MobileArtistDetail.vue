<script setup lang="ts">
import { computed } from 'vue';
import { Play, Shuffle, Loader2, Heart, User } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import MobileSongRow from './MobileSongRow.vue';

/**
 * 移动端艺术家详情页。
 *
 * 布局（单列纵向）：
 *   - 圆形头像（渐变背景 + 首字母）
 *   - 艺术家名称
 *   - 歌曲数
 *   - Play All / Shuffle / Favorite 按钮
 *   - Divider
 *   - 曲目列表（MobileSongRow）
 */

const playerStore = usePlayerStore();

const detail = computed(() => playerStore.currentArtistDetails);
const tracks = computed(() => detail.value?.tracks ?? []);

const isLoading = computed(() => !detail.value);

const isArtistFav = computed(() =>
  playerStore.favoriteArtists.some(a => a.id === playerStore.activeArtistId)
);

function toggleArtistFav() {
  if (playerStore.activeArtistId !== null) {
    playerStore.toggleFavoriteArtist(playerStore.activeArtistId, !isArtistFav.value);
  }
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

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function toggleFav(trackId: number) {
  playerStore.toggleFavorite(trackId);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden">

    <!-- 加载中 -->
    <div v-if="isLoading" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
      <span class="text-[12px]">加载艺术家…</span>
    </div>

    <template v-else-if="detail">
      <div class="flex-1 overflow-y-auto">
        <!-- 头像 + 信息 -->
        <div class="flex flex-col items-center px-6 pt-6 pb-2">
          <div class="w-[120px] h-[120px] rounded-full overflow-hidden bg-gradient-to-br from-gray-400 to-gray-600 flex items-center justify-center mb-4 flex-shrink-0">
            <User class="w-12 h-12 text-white/60" aria-hidden="true" />
          </div>

          <h1 class="text-[20px] font-bold text-text-primary tracking-tight leading-tight text-center mb-1">
            {{ detail.name }}
          </h1>
          <p class="text-text-muted font-mono uppercase tracking-wider mb-4" style="font-size: var(--text-11);">
            {{ detail.track_count ?? tracks.length }} 首歌曲
          </p>

          <!-- 操作按钮 -->
          <div class="flex items-center gap-3 w-full max-w-[280px]">
            <button class="flex-1 h-11 rounded-full bg-text-primary text-bg-canvas text-[14px] font-medium flex items-center justify-center gap-2 active:opacity-80 transition-opacity" @click="playAll">
              <Play class="w-[16px] h-[16px] fill-current" aria-hidden="true" />
              播放全部
            </button>
            <button class="flex-1 h-11 rounded-full border border-border-solid text-[14px] font-medium text-text-primary flex items-center justify-center gap-2 active:bg-list-hover transition-colors-smooth" @click="shufflePlay">
              <Shuffle class="w-[16px] h-[16px]" aria-hidden="true" />
              随机播放
            </button>
            <button class="w-11 h-11 rounded-full flex items-center justify-center border border-border-solid transition-colors-smooth active:bg-list-hover flex-shrink-0" :class="isArtistFav ? 'text-brand-orange' : 'text-text-muted'" :aria-label="isArtistFav ? '取消收藏' : '收藏'" @click="toggleArtistFav">
              <Heart class="w-[20px] h-[20px]" :class="isArtistFav ? 'fill-current' : ''" aria-hidden="true" />
            </button>
          </div>
        </div>

        <!-- Divider -->
        <div class="h-px bg-border-color mx-4 mt-4"></div>

        <!-- 曲目列表 -->
        <div v-if="tracks.length === 0" class="flex flex-col items-center justify-center py-12 gap-3 text-text-muted">
          <span class="text-[13px]">暂无曲目</span>
        </div>
        <div v-else class="py-1">
          <MobileSongRow
            v-for="(track, index) in tracks"
            :key="track.id"
            :track="track"
            :index="index"
            :is-playing="playerStore.isPlaying"
            :is-current="isPlayingTrack(track.id)"
            @play="playTrack($event)"
            @toggle-fav="toggleFav"
          />
        </div>
      </div>
    </template>

  </div>
</template>
