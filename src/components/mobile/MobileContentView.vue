<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { Loader2, Disc3, Music, Shuffle, Play, Heart, ListMusic, Folder } from 'lucide-vue-next';
import { usePlayerStore, type Album, type Track } from '../../stores/player';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import MobileSongRow from './MobileSongRow.vue';
import MobileAlbumCard from './MobileAlbumCard.vue';
import MobileSettings from './MobileSettings.vue';
import MobileArtistDetail from './MobileArtistDetail.vue';
import ActionSheet from './ActionSheet.vue';
import type { ActionItem } from './ActionSheet.vue';

/**
 * 移动端内容区。
 *
 * 根据 playerStore.activeLibraryTab 切换不同视图：
 *
 *   全部歌曲 / 最近播放 / 喜欢的音乐 → 歌曲列表
 *   专辑（无 activeAlbumId）         → 专辑网格
 *   专辑（有 activeAlbumId）         → 专辑详情
 *   艺术家（无 activeArtistId）      → 艺术家网格
 *   艺术家（有 activeArtistId）      → 艺术家详情
 *   文件夹                           → 文件夹视图
 *   播放列表（无 activePlaylistId）   → 歌单列表
 *   播放列表（有 activePlaylistId）   → 歌单详情
 *   收藏的专辑                       → 收藏专辑网格
 *   收藏的歌手                       → 收藏歌手网格
 *   设置                             → 设置页
 */

const playerStore = usePlayerStore();

/* ============ 视图判定 ============ */

const isAlbumGridView = computed(() =>
  playerStore.activeLibraryTab === '专辑' && !playerStore.activeAlbumId
);
const isAlbumDetailView = computed(() =>
  playerStore.activeLibraryTab === '专辑' && !!playerStore.activeAlbumId
);
const isTracksView = computed(() =>
  ['全部歌曲', '最近播放', '喜欢的音乐'].includes(playerStore.activeLibraryTab)
);
const isArtistGridView = computed(() =>
  playerStore.activeLibraryTab === '艺术家' && !playerStore.activeArtistId
);
const isArtistDetailView = computed(() =>
  playerStore.activeLibraryTab === '艺术家' && !!playerStore.activeArtistId
);
const isFolderView = computed(() =>
  playerStore.activeLibraryTab === '文件夹'
);
const isPlaylistListView = computed(() =>
  playerStore.activeLibraryTab === '播放列表' && !playerStore.activePlaylistId
);
const isPlaylistDetailView = computed(() =>
  playerStore.activeLibraryTab === '播放列表' && !!playerStore.activePlaylistId
);
const isFavoriteAlbumsView = computed(() =>
  playerStore.activeLibraryTab === '收藏的专辑'
);
const isFavoriteArtistsView = computed(() =>
  playerStore.activeLibraryTab === '收藏的歌手'
);
const isSettingsView = computed(() =>
  playerStore.activeLibraryTab === '设置'
);

/* ============ 数据拉取：Tab 切换时加载对应数据 ============ */

function loadForCurrentTab() {
  const tab = playerStore.activeLibraryTab;
  if (tab === '最近播放') playerStore.fetchRecentlyPlayed();
  else if (tab === '喜欢的音乐') playerStore.fetchFavoriteTracks();
  else if (tab === '专辑') playerStore.fetchAlbums(true);
  else if (tab === '艺术家') playerStore.fetchArtists(true);
  else if (tab === '收藏的专辑') playerStore.fetchFavoriteAlbums();
  else if (tab === '收藏的歌手') playerStore.fetchFavoriteArtists();
  else if (tab === '播放列表') playerStore.fetchPlaylists();
  else playerStore.fetchTracks(true);
}

watch(() => playerStore.activeLibraryTab, loadForCurrentTab);

onMounted(() => {
  if (playerStore.tracks.length === 0 && playerStore.albums.length === 0) {
    loadForCurrentTab();
  }
});

/* ============ 歌曲列表 ============ */

function isCurrentTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playSong(index: number) {
  playerStore.playTrack(index);
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
  const found = playerStore.tracks.find(t => t.id === trackId);
  if (found) {
    sheetTrack.value = found;
    sheetVisible.value = true;
  }
}

function onSheetClose() {
  sheetVisible.value = false;
  sheetTrack.value = null;
}

/* ============ 专辑网格：无限滚动 ============ */

const albumScrollContainer = ref<HTMLElement | null>(null);
const albumSentinel = ref<HTMLElement | null>(null);
let albumObserver: IntersectionObserver | null = null;

function ensureAlbumObserver() {
  if (albumObserver) return albumObserver;
  const el = albumScrollContainer.value;
  if (!el) return null;
  albumObserver = new IntersectionObserver(
    (entries) => {
      if (
        entries[0]?.isIntersecting &&
        !playerStore.isLoadingAlbums &&
        playerStore.hasMoreAlbums
      ) {
        playerStore.fetchAlbums(false);
      }
    },
    { root: el, rootMargin: '400px' }
  );
  return albumObserver;
}

/* 视图切换到专辑网格时（重新）observe sentinel */
watch(isAlbumGridView, (visible) => {
  if (visible) {
    nextTick(() => {
      const obs = ensureAlbumObserver();
      if (obs && albumSentinel.value) {
        obs.observe(albumSentinel.value);
      }
    });
  } else {
    if (albumSentinel.value && albumObserver) {
      albumObserver.unobserve(albumSentinel.value);
    }
  }
});

onBeforeUnmount(() => {
  albumObserver?.disconnect();
});

function onAlbumSelect(album: Album) {
  playerStore.activeAlbumId = album.id;
}

/* ============ 专辑详情 ============ */

const album = computed(() => playerStore.currentAlbumDetails);
const albumTracks = computed(() => album.value?.tracks ?? []);
const isLoadingAlbum = computed(() =>
  playerStore.activeAlbumId !== null && !album.value
);

function albumMetaText(): string {
  if (!album.value) return '';
  const parts: string[] = [];
  if (album.value.year) parts.push(String(album.value.year));
  parts.push(`${albumTracks.value.length} TRACKS`);
  const totalSec = albumTracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const m = Math.floor(totalSec / 60);
  parts.push(`${m} 分钟`);
  return parts.join(' · ');
}

function albumIsFav(): boolean {
  return playerStore.favoriteAlbums.some(a => a.id === playerStore.activeAlbumId);
}

function toggleAlbumFav() {
  if (playerStore.activeAlbumId !== null) {
    playerStore.toggleFavoriteAlbum(playerStore.activeAlbumId, !albumIsFav());
  }
}

function playAlbumAll() {
  if (albumTracks.value.length > 0) playerStore.playAll(albumTracks.value, 0);
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

const albumCoverSrc = useArtworkSrc(() => album.value?.cover_artwork_id ?? null);

/* ============ 艺术家网格 ============ */

function selectArtist(artistId: number) {
  playerStore.activeArtistId = artistId;
}

/* ============ 歌单操作 ============ */

function selectPlaylist(id: number) {
  playerStore.activePlaylistId = id;
  playerStore.refreshCurrentPlaylistTracks(id);
}

/* ============ 歌单详情 ============ */

const playlistDetail = computed(() => playerStore.currentPlaylistDetails);
const playlistTracks = computed(() => playlistDetail.value?.tracks ?? []);

function onPlaylistTrackLongPress(trackId: number) {
  const found = playlistTracks.value.find(t => t.id === trackId);
  if (found) {
    sheetTrack.value = found;
    sheetVisible.value = true;
  }
}

function onAlbumTrackLongPress(trackId: number) {
  const found = albumTracks.value.find(t => t.id === trackId);
  if (found) {
    sheetTrack.value = found;
    sheetVisible.value = true;
  }
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden" style="min-height: 0;">

    <!-- ===== 专辑详情 ===== -->
    <template v-if="isAlbumDetailView">
      <div v-if="isLoadingAlbum" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
        <span class="text-[12px]">加载专辑…</span>
      </div>

      <template v-else-if="album">
        <div class="flex-1 overflow-y-auto">
          <div class="flex flex-col items-center px-6 pt-6 pb-2">
            <div class="relative w-[60%] max-w-[260px] aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-4">
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
              <button class="flex-1 h-11 rounded-full bg-text-primary text-bg-canvas text-[14px] font-medium flex items-center justify-center gap-2 active:opacity-80 transition-opacity" @click="playAlbumAll">
                <Play class="w-[16px] h-[16px] fill-current" aria-hidden="true" />
                播放全部
              </button>
              <button class="flex-1 h-11 rounded-full border border-border-solid text-[14px] font-medium text-text-primary flex items-center justify-center gap-2 active:bg-list-hover transition-colors-smooth" @click="shuffleAlbum">
                <Shuffle class="w-[16px] h-[16px]" aria-hidden="true" />
                随机播放
              </button>
              <button class="w-11 h-11 rounded-full flex items-center justify-center border border-border-solid transition-colors-smooth active:bg-list-hover flex-shrink-0" :class="albumIsFav() ? 'text-brand-orange' : 'text-text-muted'" :aria-label="albumIsFav() ? '取消收藏' : '收藏专辑'" @click="toggleAlbumFav">
                <Heart class="w-[20px] h-[20px]" :class="albumIsFav() ? 'fill-current' : ''" aria-hidden="true" />
              </button>
            </div>
          </div>
          <div class="h-px bg-border-color mx-4 mt-4"></div>
          <div v-if="albumTracks.length === 0" class="flex flex-col items-center justify-center py-12 gap-3 text-text-muted">
            <span class="text-[13px]">该专辑暂无曲目</span>
          </div>
          <div v-else class="py-1">
            <MobileSongRow
              v-for="(track, index) in albumTracks"
              :key="track.id"
              :track="track"
              :index="index"
              :is-playing="playerStore.isPlaying"
              :is-current="isAlbumTrackPlaying(track.id)"
              @play="playAlbumTrack($event)"
              @toggle-fav="toggleFav"
              @long-press="onAlbumTrackLongPress"
            />
          </div>
        </div>
      </template>
    </template>

    <!-- ===== 专辑网格 ===== -->
    <div v-else-if="isAlbumGridView" ref="albumScrollContainer" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.isLoadingAlbums && playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
        <span class="text-[12px]">加载专辑…</span>
      </div>
      <div v-else-if="playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">没有找到专辑</span>
      </div>
      <div v-else class="grid gap-4 pb-6" style="grid-template-columns: repeat(2, 1fr);">
        <MobileAlbumCard
          v-for="album in playerStore.albums"
          :key="album.id"
          :album="album"
          @select="onAlbumSelect"
        />
      </div>
      <div ref="albumSentinel" class="h-px" />
      <div v-if="playerStore.isLoadingAlbums && playerStore.albums.length > 0" class="flex items-center justify-center py-4 text-text-muted">
        <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" aria-hidden="true" />
        <span class="text-[11px]">加载更多…</span>
      </div>
      <div v-if="!playerStore.hasMoreAlbums && playerStore.albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
        <span class="text-[11px]">已显示全部 {{ playerStore.albumsTotalCount.toLocaleString() }} 张专辑</span>
      </div>
    </div>

    <!-- ===== 歌曲列表 ===== -->
    <div v-else-if="isTracksView" class="flex-1 overflow-y-auto">
      <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
        <span class="text-[12px]">加载中…</span>
      </div>
      <div v-else-if="playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Music class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">没有找到歌曲</span>
      </div>
      <div v-else class="py-1">
        <MobileSongRow
          v-for="(track, index) in playerStore.tracks"
          :key="track.id"
          :track="track"
          :index="index"
          :is-playing="playerStore.isPlaying"
          :is-current="isCurrentTrack(track.id)"
          @play="playSong"
          @toggle-fav="toggleFav"
          @long-press="onTrackLongPress"
        />
      </div>
      <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length > 0" class="flex items-center justify-center py-4 text-text-muted">
        <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" aria-hidden="true" />
        <span class="text-[11px]">加载更多…</span>
      </div>
    </div>

    <!-- ===== 艺术家网格 ===== -->
    <div v-else-if="isArtistGridView" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.artists.length > 0" class="grid gap-4 pb-4" style="grid-template-columns: repeat(2, 1fr);">
        <div v-for="artist in playerStore.artists" :key="artist.id" class="cursor-pointer min-w-0" @click="selectArtist(artist.id)">
          <div class="w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-2 flex items-center justify-center">
            <div class="w-full h-full bg-gradient-to-br from-gray-400 to-gray-600 flex items-center justify-center">
              <span class="text-white/70 text-[28px] font-bold">{{ artist.name.charAt(0) }}</span>
            </div>
          </div>
          <p class="text-[15px] font-medium text-text-primary truncate leading-tight">{{ artist.name }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
      </div>
      <div v-else class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <span class="text-[13px]">还没有扫描到艺术家</span>
      </div>
    </div>

    <!-- ===== 艺术家详情 ===== -->
    <MobileArtistDetail v-else-if="isArtistDetailView" />

    <!-- ===== 歌单列表 ===== -->
    <div v-else-if="isPlaylistListView" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.playlists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <ListMusic class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">暂无歌单</span>
      </div>
      <div v-else class="pb-4">
        <div
          v-for="pl in playerStore.playlists"
          :key="pl.id"
          class="flex items-center gap-3 cursor-pointer active:bg-list-hover transition-colors-smooth"
          style="height: var(--touch-row);"
          @click="selectPlaylist(pl.id)"
        >
          <div class="w-10 h-10 rounded-[8px] bg-bg-hover flex items-center justify-center flex-shrink-0">
            <ListMusic class="w-5 h-5 text-text-muted" aria-hidden="true" />
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-text-primary font-medium truncate" style="font-size: var(--text-mobile-body);">{{ pl.name }}</p>
            <p class="text-text-muted font-mono truncate" style="font-size: var(--text-12);">{{ pl.count }} 首</p>
          </div>
          <span class="text-text-muted" style="font-size: var(--text-20);">&rsaquo;</span>
        </div>
      </div>
    </div>

    <!-- ===== 歌单详情 ===== -->
    <div v-else-if="isPlaylistDetailView" class="flex-1 flex flex-col">
      <div v-if="!playlistDetail" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
        <span class="text-[12px]">加载歌单…</span>
      </div>
      <template v-else>
        <div class="text-center px-6 pt-4 pb-2">
          <h1 class="text-[20px] font-bold text-text-primary">{{ playlistDetail.name }}</h1>
          <p class="text-text-muted font-mono uppercase tracking-wider" style="font-size: var(--text-11);">
            {{ playlistTracks.length }} TRACKS
          </p>
        </div>
        <div class="h-px bg-border-color mx-4 mt-2"></div>
        <div class="flex-1 overflow-y-auto py-1">
          <MobileSongRow
            v-for="(track, index) in playlistTracks"
            :key="track.id"
            :track="track"
            :index="index"
            :is-playing="playerStore.isPlaying"
            :is-current="isCurrentTrack(track.id)"
            @play="(i: number) => playerStore.playAll(playlistTracks, i)"
            @toggle-fav="toggleFav"
            @long-press="onPlaylistTrackLongPress"
          />
        </div>
      </template>
    </div>

    <!-- ===== 文件夹视图（简化） ===== -->
    <div v-else-if="isFolderView" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.localSources.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Folder class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">暂无文件夹数据源</span>
      </div>
      <div v-else class="pb-4">
        <div
          v-for="src in playerStore.localSources"
          :key="src.id"
          class="flex items-center gap-3 active:bg-list-hover transition-colors-smooth rounded-[8px] px-2"
          style="height: var(--touch-row);"
        >
          <Folder class="w-[20px] h-[20px] text-text-muted flex-shrink-0" aria-hidden="true" />
          <div class="flex-1 min-w-0">
            <p class="text-text-primary font-medium truncate" style="font-size: var(--text-mobile-body);">{{ src.name }}</p>
            <p class="text-text-muted font-mono truncate" style="font-size: var(--text-11);">{{ src.path }}</p>
          </div>
        </div>
      </div>
    </div>

    <!-- ===== 收藏的专辑 ===== -->
    <div v-else-if="isFavoriteAlbumsView" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.favoriteAlbums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">暂无收藏的专辑</span>
      </div>
      <div v-else class="grid gap-4 pb-4" style="grid-template-columns: repeat(2, 1fr);">
        <MobileAlbumCard
          v-for="album in playerStore.favoriteAlbums"
          :key="'fa-' + album.id"
          :album="album"
          @select="onAlbumSelect"
        />
      </div>
    </div>

    <!-- ===== 收藏的歌手 ===== -->
    <div v-else-if="isFavoriteArtistsView" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.favoriteArtists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <span class="text-[13px]">暂无收藏的歌手</span>
      </div>
      <div v-else class="grid gap-4 pb-4" style="grid-template-columns: repeat(2, 1fr);">
        <div
          v-for="artist in playerStore.favoriteArtists"
          :key="'fav-' + artist.id"
          class="cursor-pointer min-w-0"
          @click="selectArtist(artist.id)"
        >
          <div class="w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-2 flex items-center justify-center">
            <div class="w-full h-full bg-gradient-to-br from-gray-400 to-gray-600 flex items-center justify-center">
              <span class="text-white/70 text-[28px] font-bold">{{ artist.name.charAt(0) }}</span>
            </div>
          </div>
          <p class="text-[15px] font-medium text-text-primary truncate leading-tight">{{ artist.name }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
      </div>
    </div>

    <!-- ===== 设置 ===== -->
    <MobileSettings v-else-if="isSettingsView" />

    <!-- ===== 其他视图占位 ===== -->
    <div v-else class="flex-1 flex flex-col items-center justify-center gap-4 text-center px-8">
      <Disc3 class="w-10 h-10 text-text-disabled" aria-hidden="true" />
      <p class="text-[15px] text-text-secondary font-medium">{{ playerStore.activeLibraryTab }}</p>
      <p class="text-[13px] text-text-muted">即将支持</p>
    </div>

    <!-- ===== ActionSheet（长按菜单） ===== -->
    <ActionSheet
      :visible="sheetVisible"
      :actions="sheetActions"
      @close="onSheetClose"
    />

  </div>
</template>
