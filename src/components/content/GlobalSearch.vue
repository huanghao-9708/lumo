<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue';
import {
  Play, Loader2, Heart, MoreHorizontal, Music, Search, Disc3, User,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { libraryGetTracks, libraryGetAlbums, libraryGetArtists } from '../../api/library';
import type { TrackDTO, AlbumDTO, ArtistDTO } from '../../api/types';

const playerStore = usePlayerStore();

const activeTab = ref<'tracks' | 'albums' | 'artists'>('tracks');
const isSearching = ref(false);
const trackResults = ref<TrackDTO[]>([]);
const albumResults = ref<AlbumDTO[]>([]);
const artistResults = ref<ArtistDTO[]>([]);

let searchTimer: ReturnType<typeof setTimeout> | null = null;

watch(() => playerStore.globalSearchQuery, async (query) => {
  if (searchTimer) clearTimeout(searchTimer);
  if (!query.trim()) {
    trackResults.value = [];
    albumResults.value = [];
    artistResults.value = [];
    isSearching.value = false;
    return;
  }
  isSearching.value = true;
  searchTimer = setTimeout(async () => {
    try {
      const q = query.trim();
      const [tracks, albums, artists] = await Promise.all([
        libraryGetTracks(20, 0, q),
        libraryGetAlbums(12, 0, q),
        libraryGetArtists(12, 0, q),
      ]);
      trackResults.value = tracks;
      albumResults.value = albums;
      artistResults.value = artists;
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      isSearching.value = false;
    }
  }, 300);
});

onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); });

const tabCounts = computed(() => ({
  tracks: trackResults.value.length,
  albums: albumResults.value.length,
  artists: artistResults.value.length,
}));

const totalCount = computed(() =>
  trackResults.value.length + albumResults.value.length + artistResults.value.length
);

function playTrack(index: number, dto: TrackDTO) {
  const sec = Math.floor((dto.duration_ms || 0) / 1000);
  playerStore.playAll([{
    id: dto.id,
    title: dto.title,
    artist: dto.artist_name || '未知艺人',
    album: dto.album_title || '未知专辑',
    duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
    durationSec: sec,
    format: dto.format ? dto.format.toUpperCase() : 'UNKNOWN',
    coverColor: '',
    cover_artwork_id: dto.cover_artwork_id,
    isFavorite: dto.is_favorite || false,
    primary_file_id: dto.media_file_id,
    fileSize: dto.file_size ?? null,
  }], index);
}

function selectAlbum(album: AlbumDTO) {
  playerStore.activeAlbumId = album.id;
  playerStore.activeLibraryTab = '专辑';
  playerStore.globalSearchQuery = '';
}

function selectArtist(artist: ArtistDTO) {
  playerStore.activeArtistId = artist.id;
  playerStore.activeLibraryTab = '艺术家';
  playerStore.globalSearchQuery = '';
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- Header -->
    <div class="px-8 pt-6 pb-0 flex-shrink-0">
      <div class="flex items-end justify-between mb-2">
        <div>
          <h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none mb-2">搜索</h1>
          <p class="text-[12px] text-text-muted font-mono">
            <template v-if="isSearching">搜索中…</template>
            <template v-else-if="totalCount > 0">找到 {{ totalCount }} 个结果</template>
            <template v-else-if="playerStore.globalSearchQuery.trim()">没有结果</template>
            <template v-else>输入关键词开始搜索</template>
          </p>
        </div>
      </div>
    </div>

    <!-- 搜索提示（无输入时） -->
    <div v-if="!playerStore.globalSearchQuery.trim()" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Search class="w-10 h-10 text-text-disabled" />
      <p class="text-[13px]">在 TopBar 搜索框输入关键词</p>
      <p class="text-[11px] text-text-muted/70">搜索歌曲、艺术家、专辑…</p>
    </div>

    <template v-else>
      <!-- Tab Bar -->
      <div class="flex items-center gap-8 px-8 pt-4 pb-0 flex-shrink-0">
        <button
          v-for="tab in (['tracks', 'albums', 'artists'] as const)"
          :key="tab"
          class="text-[13px] pb-2 border-b-2 transition-colors-smooth"
          :class="activeTab === tab ? 'text-text-primary border-brand-orange font-medium' : 'text-text-muted border-transparent hover:text-text-primary'"
          @click="activeTab = tab"
        >
          {{ { tracks: '歌曲', albums: '专辑', artists: '艺术家' }[tab] }}
          <span class="text-[11px] ml-1 text-text-muted">({{ tabCounts[tab] }})</span>
        </button>
      </div>
      <div class="h-px bg-border-color mx-8"></div>

      <div class="flex-1 overflow-y-auto px-8">
        <!-- Loading -->
        <div v-if="isSearching" class="flex items-center justify-center py-16">
          <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
        </div>

        <!-- 歌曲 Tab -->
        <template v-else-if="activeTab === 'tracks'">
          <div v-if="trackResults.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <Music class="w-8 h-8 text-text-disabled mb-2" />
            <span class="text-[12px]">没有找到歌曲</span>
          </div>
          <div v-else>
            <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
              <div class="w-10 text-center shrink-0">#</div>
              <div class="flex-[2] min-w-0 pl-1">标题</div>
              <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
              <div class="flex-[1.5] min-w-0 hidden md:block">专辑</div>
              <div class="w-[56px] text-right shrink-0 hidden lg:block">时长</div>
            </div>
            <div
              v-for="(t, i) in trackResults"
              :key="t.id"
              class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer"
              style="height: 40px;"
              @dblclick="playTrack(i, t)"
            >
              <div class="w-10 text-center shrink-0 text-[12px] font-mono">
                <Play class="w-[12px] h-[12px] fill-current mx-auto text-text-muted group-hover:text-text-secondary" />
              </div>
              <div class="flex-[2] min-w-0 pl-1">
                <span class="text-[13px] truncate block text-text-primary font-medium">{{ t.title }}</span>
              </div>
              <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ t.artist_name }}</div>
              <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate">{{ t.album_title }}</div>
              <div class="w-[56px] text-right shrink-0 hidden lg:block text-[12px] font-mono text-text-muted tabular-nums">
                {{ Math.floor((t.duration_ms || 0) / 60000) }}:{{ String(Math.floor(((t.duration_ms || 0) % 60000) / 1000)).padStart(2, '0') }}
              </div>
            </div>
          </div>
        </template>

        <!-- 专辑 Tab -->
        <template v-else-if="activeTab === 'albums'">
          <div v-if="albumResults.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <Disc3 class="w-8 h-8 text-text-disabled mb-2" />
            <span class="text-[12px]">没有找到专辑</span>
          </div>
          <div v-else class="grid gap-6 pt-4 pb-4" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))">
            <div
              v-for="album in albumResults"
              :key="album.id"
              class="group cursor-pointer"
              @click="selectAlbum(album)"
            >
              <div class="w-full aspect-square rounded-[10px] mb-3 overflow-hidden bg-bg-hover flex items-center justify-center">
                <Disc3 class="w-10 h-10 text-text-disabled" />
              </div>
              <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ album.title }}</p>
              <p class="text-[13px] text-text-secondary truncate">{{ album.artist_name }}<span v-if="album.release_year"> · {{ album.release_year }}</span></p>
            </div>
          </div>
        </template>

        <!-- 艺术家 Tab -->
        <template v-else-if="activeTab === 'artists'">
          <div v-if="artistResults.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <User class="w-8 h-8 text-text-disabled mb-2" />
            <span class="text-[12px]">没有找到艺术家</span>
          </div>
          <div v-else class="grid gap-6 pt-4 pb-4" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))">
            <div
              v-for="artist in artistResults"
              :key="artist.id"
              class="group cursor-pointer"
              @click="selectArtist(artist)"
            >
              <div class="w-full aspect-square rounded-[10px] mb-3 overflow-hidden bg-bg-hover flex items-center justify-center">
                <User class="w-10 h-10 text-text-disabled" />
              </div>
              <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ artist.name }}</p>
              <p class="text-[13px] text-text-secondary truncate">{{ artist.track_count }} 首歌曲</p>
            </div>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>
