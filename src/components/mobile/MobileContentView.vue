<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { Loader2, Disc3, Folder, ListMusic } from 'lucide-vue-next';
import { usePlayerStore, type Album, type Track } from '../../stores/player';
import MobileSongRow from './MobileSongRow.vue';
import MobileAlbumCard from './MobileAlbumCard.vue';
import MobileSettings from './MobileSettings.vue';
import MobileAddLocalSource from './MobileAddLocalSource.vue';
import MobileWebdavSourceEditor from './MobileWebdavSourceEditor.vue';
import MobileArtistDetail from './MobileArtistDetail.vue';
import MobileAlbumDetail from './MobileAlbumDetail.vue';
import MobilePlaylistDetail from './MobilePlaylistDetail.vue';
import MobileSmartPlaylist from './MobileSmartPlaylist.vue';
import HomeView from '../content/HomeView.vue';
import MobileAiRadio from './MobileAiRadio.vue';
import MobileTrackVersions from './MobileTrackVersions.vue';
import ActionSheet, { type ActionItem } from './ActionSheet.vue';
import { useScrollRestore } from '../../composables/useScrollRestore';
import { useVirtualList } from '../../composables/useVirtualList';
import { getArtworkUrl } from '../../utils';

/**
 * 移动端内容区。
 *
 * 根据 playerStore.activeLibraryTab 切换不同视图：
 *
 *   全部歌曲 / 最近播放 / 喜欢的音乐 → 歌曲列表
 *   专辑（无 activeAlbumId）         → 专辑网格
 *   专辑（有 activeAlbumId）         → 专辑详情 (MobileAlbumDetail)
 *   艺术家（无 activeArtistId）      → 艺术家网格
 *   艺术家（有 activeArtistId）      → 艺术家详情 (MobileArtistDetail)
 *   文件夹                           → 文件夹视图
 *   播放列表（无 activePlaylistId）   → 歌单列表
 *   播放列表（有 activePlaylistId）   → 歌单详情 (MobilePlaylistDetail)
 *   智能歌单                         → 智能歌单 (MobileSmartPlaylist)
 *   收藏的专辑                       → 收藏专辑网格
 *   收藏的歌手                       → 收藏歌手网格
 *   设置                             → 设置页
 *   AI 电台                          → AI 电台 (MobileAiRadio)
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
const isSmartPlaylistView = computed(() =>
  playerStore.activeLibraryTab === '智能歌单'
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
const isHomeView = computed(() =>
  playerStore.activeLibraryTab === '首页'
);
const isAiRadioView = computed(() =>
  playerStore.activeLibraryTab === 'AI 电台'
);

const showAddLocalSource = ref(false);
function openAddLocalSource() {
  showAddLocalSource.value = true;
}
function closeAddLocalSource() {
  showAddLocalSource.value = false;
}

const showAddWebdavSource = ref(false);
function openAddWebdavSource() {
  showAddWebdavSource.value = true;
}
function closeAddWebdavSource() {
  showAddWebdavSource.value = false;
}

/* ============ 数据拉取：Tab 切换时加载对应数据 ============ */

function loadForCurrentTab() {
  const tab = playerStore.activeLibraryTab;
  if (tab === '首页' || tab === 'AI 电台') return;
  if (tab === '智能歌单') {
    if (!playerStore.activeSmartPlaylistKind) {
      playerStore.activeSmartPlaylistKind = 'most_played';
    }
    playerStore.loadSmartPlaylist(playerStore.activeSmartPlaylistKind);
    return;
  }
  if (tab === '最近播放') playerStore.fetchRecentlyPlayed();
  else if (tab === '喜欢的音乐') playerStore.fetchFavoriteTracks();
  else if (tab === '专辑') playerStore.fetchAlbums(true);
  else if (tab === '艺术家') playerStore.fetchArtists(true);
  else if (tab === '收藏的专辑') playerStore.fetchFavoriteAlbums();
  else if (tab === '收藏的歌手') playerStore.fetchFavoriteArtists();
  else if (tab === '播放列表') playerStore.fetchPlaylists();
  else playerStore.fetchTracks(true);
}

watch(() => playerStore.activeLibraryTab, () => {
  if (playerStore.isHistoryRestore) {
    playerStore.isHistoryRestore = false;
    return;
  }
  loadForCurrentTab();
});

/* ============ 各列表的滚动位置记忆 ============ */
const tracksScrollEl = useScrollRestore(() => 'm-tracks');
const artistGridScrollEl = useScrollRestore(() => 'm-artist-grid');
const playlistListScrollEl = useScrollRestore(() => 'm-playlist-list');
const folderScrollEl = useScrollRestore(() => 'm-folder');
const favoriteAlbumsScrollEl = useScrollRestore(() => 'm-favorite-albums');
const favoriteArtistsScrollEl = useScrollRestore(() => 'm-favorite-artists');

onMounted(() => {
  if (playerStore.tracks.length === 0 && playerStore.albums.length === 0) {
    loadForCurrentTab();
  }
  if (isAlbumGridView.value) activateAlbumGrid();
  if (isArtistGridView.value) activateArtistGrid();
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
    {
      label: '音频版本与规格',
      onClick: () => {
        versionTargetTrack.value = track;
        showVersionDrawer.value = true;
      },
    },
  ];
});

/* ============ 音频版本与规格抽屉 ============ */
const showVersionDrawer = ref(false);
const versionTargetTrack = ref<Track | null>(null);

function onVersionChanged(newPrimaryFileId: number) {
  if (versionTargetTrack.value) {
    versionTargetTrack.value.primary_file_id = newPrimaryFileId;
  }
  loadForCurrentTab();
}

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

const albumScrollContainer = useScrollRestore(() => 'm-album-grid');
const albumSentinel = ref<HTMLElement | null>(null);
const albumItems = computed(() => playerStore.albums);
const albumGridRowHeight = ref(260);
const { totalHeight: albumGridHeight, offsetY: albumGridOffsetY, visibleItems: visibleAlbums } = useVirtualList({
  containerRef: albumScrollContainer,
  items: albumItems,
  itemHeight: albumGridRowHeight,
  columns: 2,
  scrollOffset: 12,
  buffer: 1,
});
let albumObserver: IntersectionObserver | null = null;
let albumGridResizeObserver: ResizeObserver | null = null;

function updateAlbumGridMetrics() {
  const el = albumScrollContainer.value;
  if (!el) return;
  const contentWidth = Math.max(1, el.clientWidth - 32);
  const cardWidth = (contentWidth - 16) / 2;
  // 封面正方形、两行文字和网格间距。
  albumGridRowHeight.value = Math.ceil(cardWidth + 62);
}

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
    { root: el, rootMargin: '150px' }
  );
  return albumObserver;
}

function activateAlbumGrid() {
  nextTick(() => {
    if (albumScrollContainer.value) {
      if (typeof ResizeObserver !== 'undefined') {
        albumGridResizeObserver ??= new ResizeObserver(updateAlbumGridMetrics);
        albumGridResizeObserver.observe(albumScrollContainer.value);
      }
      updateAlbumGridMetrics();
    }
    const obs = ensureAlbumObserver();
    if (obs && albumSentinel.value) obs.observe(albumSentinel.value);
  });
}

watch(isAlbumGridView, (visible) => {
  if (visible) {
    activateAlbumGrid();
  } else {
    albumGridResizeObserver?.disconnect();
    albumGridResizeObserver = null;
    albumObserver?.disconnect();
    albumObserver = null;
  }
});

onBeforeUnmount(() => {
  albumObserver?.disconnect();
  albumGridResizeObserver?.disconnect();
});

function onAlbumSelect(album: Album) {
  playerStore.activeAlbumId = album.id;
}

/* ============ 艺术家网格 ============ */

const artistSentinel = ref<HTMLElement | null>(null);
const artistItems = computed(() => playerStore.artists);
const artistGridRowHeight = ref(260);
const { totalHeight: artistGridHeight, offsetY: artistGridOffsetY, visibleItems: visibleArtists } = useVirtualList({
  containerRef: artistGridScrollEl,
  items: artistItems,
  itemHeight: artistGridRowHeight,
  columns: 2,
  scrollOffset: 12,
  buffer: 1,
});
let artistObserver: IntersectionObserver | null = null;
let artistResizeObserver: ResizeObserver | null = null;

function updateArtistGridMetrics() {
  const el = artistGridScrollEl.value;
  if (el) artistGridRowHeight.value = Math.ceil((Math.max(1, el.clientWidth - 32) - 16) / 2 + 62);
}

function activateArtistGrid() {
  void nextTick(() => {
    const el = artistGridScrollEl.value;
    if (!el) return;
    updateArtistGridMetrics();
    if (typeof ResizeObserver !== 'undefined') {
      artistResizeObserver ??= new ResizeObserver(updateArtistGridMetrics);
      artistResizeObserver.observe(el);
    }
    artistObserver ??= new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting && !playerStore.isLoadingArtists && playerStore.hasMoreArtists) {
        void playerStore.fetchArtists(false);
      }
    }, { root: el, rootMargin: '150px' });
    if (artistSentinel.value) artistObserver.observe(artistSentinel.value);
  });
}

watch(isArtistGridView, (visible) => {
  if (visible) activateArtistGrid();
  else {
    artistObserver?.disconnect();
    artistResizeObserver?.disconnect();
    artistObserver = artistResizeObserver = null;
  }
});
onBeforeUnmount(() => {
  artistObserver?.disconnect();
  artistResizeObserver?.disconnect();
});

function selectArtist(artistId: number) {
  playerStore.navigateToArtist(artistId);
}

/* ============ 歌单操作 ============ */

function selectPlaylist(id: number) {
  playerStore.openPlaylist(id);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden relative" style="min-height: 0;">

    <!-- ===== 首页（自管数据，统计卡 + 排行榜） ===== -->
    <HomeView v-if="isHomeView" />

    <!-- ===== AI 电台 ===== -->
    <MobileAiRadio v-else-if="isAiRadioView" />

    <!-- ===== 智能歌单 ===== -->
    <MobileSmartPlaylist v-else-if="isSmartPlaylistView" />

    <!-- ===== 专辑详情 ===== -->
    <MobileAlbumDetail v-else-if="isAlbumDetailView" />

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
      <div v-else class="relative" :style="{ height: `${albumGridHeight}px` }">
        <div
          class="absolute inset-x-0 top-0 grid grid-cols-2 gap-4"
          :style="{ transform: `translateY(${albumGridOffsetY}px)`, gridAutoRows: `${albumGridRowHeight - 16}px` }"
        >
          <MobileAlbumCard
            v-for="{ data: album } in visibleAlbums"
            :key="album.id"
            :album="album"
            @select="onAlbumSelect"
          />
        </div>
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
    <div v-else-if="isTracksView" ref="tracksScrollEl" class="flex-1 overflow-y-auto">
      <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
        <span class="text-[12px]">加载中…</span>
      </div>
      <div v-else-if="playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-16 px-6 text-center text-text-muted space-y-3">
        <div class="w-14 h-14 rounded-full bg-bg-hover flex items-center justify-center text-text-disabled mb-1">
          <Disc3 class="w-7 h-7" aria-hidden="true" />
        </div>
        <p class="text-[16px] font-semibold text-text-primary">曲库暂无音乐</p>
        <p class="text-[13px] text-text-muted max-w-[260px] leading-relaxed">
          添加本地音乐文件夹或 WebDAV 网盘，随时随地畅享无损音乐。
        </p>
        <button
          v-if="playerStore.sources.length === 0"
          class="mt-2 h-10 px-5 rounded-[8px] bg-brand-orange text-white text-[14px] font-medium active:opacity-85 transition-opacity"
          @click="openAddLocalSource"
        >
          添加音乐目录
        </button>
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
    <div v-else-if="isArtistGridView" ref="artistGridScrollEl" class="flex-1 overflow-y-auto px-4 pt-3">
      <div v-if="playerStore.artists.length > 0" class="relative" :style="{ height: `${artistGridHeight}px` }">
        <div class="absolute inset-x-0 top-0 grid grid-cols-2 gap-4" :style="{
          transform: `translateY(${artistGridOffsetY}px)`, gridAutoRows: `${artistGridRowHeight - 16}px`,
        }">
        <div v-for="{ data: artist } in visibleArtists" :key="artist.id" class="cursor-pointer min-w-0" @click="selectArtist(artist.id)">
          <div class="w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-2 flex items-center justify-center">
            <img v-if="artist.avatar_artwork_id" :src="getArtworkUrl(artist.avatar_artwork_id)" :alt="artist.name" loading="lazy" class="w-full h-full object-cover" />
            <div v-else class="w-full h-full bg-gradient-to-br from-warm-400 to-warm-600 flex items-center justify-center">
              <span class="text-white/70 text-[28px] font-bold">{{ artist.name.charAt(0) }}</span>
            </div>
          </div>
          <p class="text-[15px] font-medium text-text-primary truncate leading-tight">{{ artist.name }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
        </div>
      </div>
      <div v-else class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <span class="text-[13px]">还没有扫描到艺术家</span>
      </div>
      <div ref="artistSentinel" class="h-px" />
      <div v-if="playerStore.isLoadingArtists" class="flex items-center justify-center py-4 text-text-muted">
        <Loader2 class="w-3.5 h-3.5 animate-spin" aria-label="加载更多艺人" />
      </div>
    </div>

    <!-- ===== 艺术家详情 ===== -->
    <MobileArtistDetail v-else-if="isArtistDetailView" />

    <!-- ===== 歌单列表 ===== -->
    <div v-else-if="isPlaylistListView" ref="playlistListScrollEl" class="flex-1 overflow-y-auto px-4 pt-3">
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
    <MobilePlaylistDetail v-else-if="isPlaylistDetailView" />

    <!-- ===== 文件夹视图（简化） ===== -->
    <div v-else-if="isFolderView" ref="folderScrollEl" class="flex-1 overflow-y-auto px-4 pt-3">
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
    <div v-else-if="isFavoriteAlbumsView" ref="favoriteAlbumsScrollEl" class="flex-1 overflow-y-auto px-4 pt-3">
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
    <div v-else-if="isFavoriteArtistsView" ref="favoriteArtistsScrollEl" class="flex-1 overflow-y-auto px-4 pt-3">
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
            <div class="w-full h-full bg-gradient-to-br from-warm-400 to-warm-600 flex items-center justify-center">
              <span class="text-white/70 text-[28px] font-bold">{{ artist.name.charAt(0) }}</span>
            </div>
          </div>
          <p class="text-[15px] font-medium text-text-primary truncate leading-tight">{{ artist.name }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
      </div>
    </div>

    <!-- ===== 设置 ===== -->
    <MobileSettings
      v-else-if="isSettingsView"
      @add-local="openAddLocalSource"
      @add-webdav="openAddWebdavSource"
    />

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

    <!-- ===== 歌曲版本与规格抽屉 ===== -->
    <MobileTrackVersions
      :visible="showVersionDrawer"
      :track="versionTargetTrack"
      @close="showVersionDrawer = false"
      @version-changed="onVersionChanged"
    />

    <!-- ===== 添加本地来源 ===== -->
    <div
      v-if="showAddLocalSource"
      class="absolute inset-0 z-[60] bg-bg-canvas"
    >
      <MobileAddLocalSource @close="closeAddLocalSource" />
    </div>

    <!-- ===== 添加 WebDAV 来源 ===== -->
    <div
      v-if="showAddWebdavSource"
      class="absolute inset-0 z-[60] bg-bg-canvas"
    >
      <MobileWebdavSourceEditor @close="closeAddWebdavSource" />
    </div>

  </div>
</template>
