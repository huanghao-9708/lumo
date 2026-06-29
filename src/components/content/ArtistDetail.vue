<script setup lang="ts">
import { ref, computed } from 'vue';
import {
  Play, Shuffle, User, Loader2, Heart, MoreHorizontal, Clock, Disc3,
} from 'lucide-vue-next';
import { usePlayerStore, type Album, type Track } from '../../stores/player';

const props = defineProps<{
  artistId: number | null;
}>();

const playerStore = usePlayerStore();

const detail = computed(() => playerStore.currentArtistDetails);
const activeSubTab = ref<'tracks' | 'albums'>('tracks');

const albumGrid = computed<Album[]>(() => {
  const d = detail.value;
  if (!d || !d.albums) return [];
  return d.albums.map((a: any) => ({
    id: a.id,
    title: a.title,
    artist: a.artist_name || '未知艺人',
    coverColor: a.coverColor || 'from-gray-500 to-gray-700',
    cover_artwork_id: a.cover_artwork_id ?? null,
    cover_thumb: a.cover_thumb || null,
    artist_name: a.artist_name,
    track_count: a.track_count,
  }));
});

function playAll() {
  const d = detail.value;
  if (d && d.tracks && d.tracks.length > 0) {
    playerStore.playAll(d.tracks, 0);
  }
}

function shufflePlay() {
  const d = detail.value;
  if (d && d.tracks && d.tracks.length > 0) {
    const idx = Math.floor(Math.random() * d.tracks.length);
    playerStore.playAll(d.tracks, idx);
  }
}

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playTrack(index: number) {
  const d = detail.value;
  if (d && d.tracks) playerStore.playAll(d.tracks, index);
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

function selectAlbum(albumId: number) {
  playerStore.activeLibraryTab = '专辑';
  playerStore.activeAlbumId = albumId;
}

function getColorClass(color: string): string {
  return color || 'from-gray-500 to-gray-700';
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <div v-if="!artistId" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <User class="w-10 h-10 text-text-disabled" />
      <p class="text-[13px]">选择一位艺术家查看详情</p>
    </div>

    <div v-else-if="!detail" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载艺术家…</span>
    </div>

    <template v-else-if="detail">
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <div
            class="w-[180px] h-[180px] rounded-[10px] overflow-hidden flex-shrink-0 flex items-center justify-center bg-gradient-to-br"
            :class="getColorClass(detail.avatarColor)"
          >
            <User class="w-[48px] h-[48px] text-white/60" />
          </div>

          <div class="flex-1 min-w-0 pt-2">
            <h1 class="text-[28px] font-bold text-text-primary tracking-tight leading-tight mb-1">{{ detail.name }}</h1>

            <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mb-5">
              {{ detail.stats?.track_count ?? 0 }} TRACKS · {{ detail.stats?.album_count ?? 0 }} ALBUMS
            </p>

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
            </div>
          </div>
        </div>
      </div>

      <div class="h-px bg-border-color mx-8"></div>

      <div class="flex items-center gap-8 px-8 pt-4 pb-0 flex-shrink-0">
        <button
          class="text-[13px] pb-2 border-b-2 transition-colors-smooth"
          :class="activeSubTab === 'tracks' ? 'text-text-primary border-brand-orange font-medium' : 'text-text-muted border-transparent hover:text-text-primary'"
          @click="activeSubTab = 'tracks'"
        >全部歌曲</button>
        <button
          class="text-[13px] pb-2 border-b-2 transition-colors-smooth"
          :class="activeSubTab === 'albums' ? 'text-text-primary border-brand-orange font-medium' : 'text-text-muted border-transparent hover:text-text-primary'"
          @click="activeSubTab = 'albums'"
        >全部专辑</button>
      </div>

      <div class="h-px bg-border-color mx-8"></div>

      <div class="flex-1 overflow-y-auto px-8">

        <template v-if="activeSubTab === 'tracks'">
          <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
            <div class="w-10 text-center shrink-0">#</div>
            <div class="w-8 shrink-0"></div>
            <div class="flex-[2] min-w-0 pl-1">标题</div>
            <div class="flex-[1.5] min-w-0 hidden sm:block">专辑</div>
            <div class="w-[56px] text-right shrink-0">
              <Clock class="w-[12px] h-[12px] inline-block" />
            </div>
            <div class="w-8 shrink-0"></div>
          </div>

          <div v-if="detail.isLoadingTracks && detail.tracks?.length === 0" class="flex items-center justify-center py-16">
            <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
          </div>

          <div v-else-if="!detail.tracks || detail.tracks.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <span class="text-[12px]">暂无歌曲</span>
          </div>

          <div v-else>
            <div
              v-for="(track, index) in detail.tracks"
              :key="track.id"
              class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
              style="height: 40px;"
              :class="{ 'playing-row bg-list-selected': isPlayingTrack(track.id) }"
              @dblclick="playTrack(index)"
            >
              <div class="w-10 text-center shrink-0 text-[12px] font-mono">
                <span v-if="isPlayingTrack(track.id)" class="text-brand-orange inline-flex items-center justify-center">
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
                  v-if="track.isFavorite"
                  class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                  @click="toggleFav(track.id, $event)"
                />
                <Heart
                  v-else
                  class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                  @click="toggleFav(track.id, $event)"
                />
              </div>

              <div class="flex-[2] min-w-0 pl-1">
                <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
                  {{ track.title }}
                </span>
              </div>

              <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ track.album }}</div>

              <div class="w-[56px] text-right shrink-0 text-[12px] font-mono text-text-muted tabular-nums">{{ track.duration }}</div>

              <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                <MoreHorizontal class="w-4 h-4 text-text-muted" />
              </div>
            </div>
          </div>
        </template>

        <template v-if="activeSubTab === 'albums'">
          <div
            class="grid gap-6 pt-4 pb-4"
            style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))"
          >
            <div
              v-for="album in albumGrid"
              :key="album.id"
              class="group cursor-pointer"
              @click="selectAlbum(album.id)"
            >
          <div
            class="relative w-full aspect-square rounded-[10px] mb-3 overflow-hidden flex-shrink-0 bg-gradient-to-br flex items-center justify-center"
            :class="getColorClass(album.coverColor)"
          >
            <img v-if="album.cover_thumb" :src="album.cover_thumb" class="w-full h-full object-cover" alt="cover" />
            <Disc3 v-else class="w-10 h-10 text-white/60" />
            <div class="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors-smooth rounded-[10px] pointer-events-none"></div>
              </div>

              <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ album.title }}</p>
              <p class="text-[13px] text-text-secondary truncate">{{ album.track_count ?? 0 }} 首歌曲</p>
            </div>
          </div>

          <div v-if="!albumGrid || albumGrid.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <span class="text-[12px]">暂无专辑</span>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>
