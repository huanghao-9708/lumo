<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount, onMounted, nextTick } from 'vue';
import { Loader2, Search, Music, User } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { libraryGetTracks, libraryGetAlbums, libraryGetArtists } from '../../api/library';
import type { TrackDTO, AlbumDTO, ArtistDTO } from '../../api/types';
import MobileSongRow from './MobileSongRow.vue';
import MobileAlbumCard from './MobileAlbumCard.vue';

/**
 * 移动端搜索页。
 *
 * 独立搜索输入框 + 防抖 300ms + 分段结果显示：
 *   - 歌曲（MobileSongRow，单击播放）
 *   - 专辑（MobileAlbumCard 2 列网格，单击进详情）
 *   - 艺术家（头像行，单击进详情）
 */

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 搜索输入 ============ */

const searchInput = ref<HTMLInputElement | null>(null);
const query = ref('');
const isSearching = ref(false);

const trackResults = ref<TrackDTO[]>([]);
const albumResults = ref<AlbumDTO[]>([]);
const artistResults = ref<ArtistDTO[]>([]);

let searchTimer: ReturnType<typeof setTimeout> | null = null;

async function doSearch(q: string) {
  if (!q.trim()) {
    trackResults.value = [];
    albumResults.value = [];
    artistResults.value = [];
    isSearching.value = false;
    return;
  }
  isSearching.value = true;
  try {
    const trimmed = q.trim();
    const [tracks, albums, artistResult] = await Promise.all([
      libraryGetTracks(20, 0, trimmed),
      libraryGetAlbums(12, 0, trimmed),
      libraryGetArtists(12, 0, trimmed),
    ]);
    trackResults.value = tracks;
    albumResults.value = albums;
    artistResults.value = artistResult.artists;
  } catch (e) {
    console.error('Search failed:', e);
  } finally {
    isSearching.value = false;
  }
}

watch(query, (val) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => doSearch(val), 300);
});

onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); });

/* ============ 自动聚焦 ============ */

onMounted(() => {
  nextTick(() => searchInput.value?.focus());
});

/* ============ 结果统计 ============ */

const hasQuery = computed(() => query.value.trim().length > 0);
const anyResults = computed(() =>
  trackResults.value.length > 0 ||
  albumResults.value.length > 0 ||
  artistResults.value.length > 0
);

const totalResults = computed(() =>
  trackResults.value.length + albumResults.value.length + artistResults.value.length
);

/* ============ 歌曲操作 ============ */

function playTrack(index: number, _dto: TrackDTO) {
  const dto = trackResults.value[index];
  if (!dto) return;
  const sec = Math.floor((dto.duration_ms || 0) / 1000);
  playerStore.playAll([{
    id: dto.id,
    title: dto.title,
    artist: dto.artist_name || '未知艺人',
    album: dto.album_title || '未知专辑',
    duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
    durationSec: sec,
    format: dto.format ? dto.format.toUpperCase() : 'UNKNOWN',
    artistId: dto.artist_id ?? null,
    albumId: dto.album_id ?? null,
    coverColor: '',
    cover_artwork_id: dto.cover_artwork_id,
    isFavorite: dto.is_favorite || false,
    primary_file_id: dto.media_file_id,
    fileSize: dto.file_size ?? null,
    sourceKind: (dto.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
  }], 0);
}

/* ============ DTO → 前端模型转换（用于 MobileSongRow 和 MobileAlbumCard） ============ */

function dtoToTrack(dto: TrackDTO) {
  const sec = Math.floor((dto.duration_ms || 0) / 1000);
  return {
    id: dto.id,
    title: dto.title,
    artist: dto.artist_name || '未知艺人',
    album: dto.album_title || '未知专辑',
    duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
    durationSec: sec,
    format: dto.format ? dto.format.toUpperCase() : 'UNKNOWN',
    artistId: dto.artist_id ?? null,
    albumId: dto.album_id ?? null,
    coverColor: '',
    cover_artwork_id: dto.cover_artwork_id,
    isFavorite: dto.is_favorite || false,
    primary_file_id: dto.media_file_id,
    fileSize: dto.file_size ?? null,
    sourceKind: (dto.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
  };
}

/* ============ 专辑/艺术家选择 → 进详情 ============ */

function selectAlbum(dto: AlbumDTO) {
  playerStore.activeAlbumId = dto.id;
  playerStore.activeLibraryTab = '专辑';
  uiStore.setMobileTab('library');
}

function selectArtist(dto: ArtistDTO) {
  playerStore.activeArtistId = dto.id;
  playerStore.activeLibraryTab = '艺术家';
  uiStore.setMobileTab('library');
}

/* ============ 播放判定 ============ */

function isTrackPlaying(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

/* ============ 清除搜索 ============ */

function clearSearch() {
  query.value = '';
  searchInput.value?.focus();
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden">

    <!-- 搜索输入框 -->
    <div class="flex items-center gap-3 px-4 py-3 flex-shrink-0">
      <div class="flex-1 relative">
        <Search class="w-[18px] h-[18px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" aria-hidden="true" />
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="搜索歌曲、专辑、艺术家…"
          class="w-full h-11 pl-9 pr-10 text-[15px] bg-bg-canvas border border-border-color rounded-[10px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:border-brand-orange/50"
          enterkeyhint="search"
        />
        <button
          v-if="query"
          class="absolute right-2 top-1/2 -translate-y-1/2 w-7 h-7 rounded-full flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
          aria-label="清除搜索"
          @click="clearSearch"
        >
          <span class="text-[14px] leading-none">&times;</span>
        </button>
      </div>
    </div>

    <!-- Divider -->
    <div class="h-px bg-border-color mx-4"></div>

    <!-- 内容区 -->
    <div class="flex-1 overflow-y-auto">

      <!-- 无输入提示 -->
      <div v-if="!hasQuery" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Search class="w-10 h-10 text-text-disabled" aria-hidden="true" />
        <p class="text-[14px]">输入关键词搜索</p>
        <p class="text-[12px] text-text-muted/70">搜索歌曲、艺术家、专辑</p>
      </div>

      <!-- 搜索中 -->
      <div v-else-if="isSearching" class="flex items-center justify-center py-20">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
      </div>

      <!-- 无结果 -->
      <div v-else-if="!anyResults" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Music class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <p class="text-[14px]">没有找到结果</p>
      </div>

      <!-- 结果列表 -->
      <div v-else class="px-4 py-2">

        <!-- 歌曲 Section -->
        <template v-if="trackResults.length > 0">
          <h2
            class="text-text-muted font-semibold uppercase tracking-widest sticky top-0 bg-bg-content z-10 py-2 mb-1"
            style="font-size: var(--text-10);"
          >
            歌曲 ({{ trackResults.length }})
          </h2>
          <MobileSongRow
            v-for="(dto, i) in trackResults"
            :key="'t-' + dto.id"
            :track="dtoToTrack(dto)"
            :index="i"
            :is-playing="playerStore.isPlaying"
            :is-current="isTrackPlaying(dto.id)"
            @play="(idx: number) => playTrack(idx, trackResults[idx])"
            @toggle-fav="(trackId: number) => playerStore.toggleFavorite(trackId)"
          />
        </template>

        <!-- 专辑 Section -->
        <template v-if="albumResults.length > 0">
          <h2
            class="text-text-muted font-semibold uppercase tracking-widest sticky top-0 bg-bg-content z-10 py-2 mb-1"
            style="font-size: var(--text-10);"
          >
            专辑 ({{ albumResults.length }})
          </h2>
          <div class="grid gap-4 pb-4" style="grid-template-columns: repeat(2, 1fr);">
            <MobileAlbumCard
              v-for="dto in albumResults"
              :key="'a-' + dto.id"
              :album="{
                id: dto.id,
                title: dto.title,
                artist: dto.artist_name || '未知艺人',
                year: dto.release_year || 0,
                coverColor: '',
                cover_artwork_id: dto.cover_artwork_id,
                cover_thumb: null,
              }"
              @select="selectAlbum(dto)"
            />
          </div>
        </template>

        <!-- 艺术家 Section -->
        <template v-if="artistResults.length > 0">
          <h2
            class="text-text-muted font-semibold uppercase tracking-widest sticky top-0 bg-bg-content z-10 py-2 mb-1"
            style="font-size: var(--text-10);"
          >
            艺术家 ({{ artistResults.length }})
          </h2>
          <div class="pb-4">
            <div
              v-for="dto in artistResults"
              :key="'ar-' + dto.id"
              class="flex items-center gap-3 cursor-pointer active:bg-list-hover transition-colors-smooth"
              style="height: var(--touch-row);"
              @click="selectArtist(dto)"
            >
              <!-- 头像占位 -->
              <div
                class="w-10 h-10 rounded-full bg-gradient-to-br from-warm-400 to-warm-600 flex items-center justify-center flex-shrink-0"
              >
                <User class="w-5 h-5 text-white/60" aria-hidden="true" />
              </div>
              <div class="flex-1 min-w-0">
                <p class="text-text-primary font-medium truncate" style="font-size: var(--text-mobile-body);">
                  {{ dto.name }}
                </p>
                <p class="text-text-muted truncate" style="font-size: var(--text-13);">
                  {{ dto.track_count }} 首歌曲
                </p>
              </div>
              <span class="text-text-muted" style="font-size: var(--text-20);">&rsaquo;</span>
            </div>
          </div>
        </template>

      </div>
    </div>

    <!-- 底部状态 -->
    <div
      v-if="hasQuery && !isSearching"
      class="px-4 py-2 border-t border-border-color flex-shrink-0"
    >
      <p class="text-text-muted font-mono" style="font-size: var(--text-11);">
        共找到 {{ totalResults.toLocaleString() }} 个结果
      </p>
    </div>

  </div>
</template>
