<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import {
  Search, Play, List, LayoutGrid, MoreHorizontal, Heart, Loader2, Music, CloudOff, CheckSquare,
} from 'lucide-vue-next';

const SKELETON_ROWS = 8;
import { usePlayerStore, type Album } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useVirtualList } from '../../composables/useVirtualList';
import { useBatchSelect } from '../../composables/useBatchSelect';
import BatchActionBar from '../shared/BatchActionBar.vue';
import AlbumGrid from '../content/AlbumGrid.vue';
import AlbumDetail from '../content/AlbumDetail.vue';
import PlaylistDetail from '../content/PlaylistDetail.vue';
import RecentlyPlayed from '../content/RecentlyPlayed.vue';
import FavoritesView from '../content/FavoritesView.vue';
import FavoriteAlbums from '../content/FavoriteAlbums.vue';
import FavoriteArtists from '../content/FavoriteArtists.vue';
import GlobalSearch from '../content/GlobalSearch.vue';
import Settings from '../content/Settings.vue';
import ArtistGrid from '../content/ArtistGrid.vue';
import ArtistDetail from '../content/ArtistDetail.vue';
import FolderView from '../content/FolderView.vue';
import SmartPlaylistView from '../content/SmartPlaylistView.vue';
import HomeView from '../content/HomeView.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 批量选择（本视图一份实例） ============ */
const batch = useBatchSelect();

/* ============ 视图状态 ============ */
const viewMode = ref<'list' | 'grid'>('list');

/* ============ 搜索 ============ */
const searchInput = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;
// 客户端内存过滤的视图：搜索词经 filterQuery prop 作用于视图内部，无需重拉后端
const CLIENT_FILTER_TABS = ['最近播放', '喜欢的音乐'];

function onSearchInput() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    // 艺术家详情页：搜索词经 filterQuery prop 过滤，绝不能落去刷新全局 tracks（防污染）
    if (isArtistDetailView.value) return;
    playerStore.searchQuery = searchInput.value;
    if (CLIENT_FILTER_TABS.includes(playerStore.activeLibraryTab)) return;
    // 根据当前 tab 拉取对应数据（全部歌曲等走后端过滤的视图）
    loadForCurrentTab();
  }, 250);
}
onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); });

/* ============ 标题 & 元信息 ============ */
const pageTitle = computed(() => {
  switch (playerStore.activeLibraryTab) {
    case '首页': return '首页';
    case '最近播放': return '最近播放';
    case '喜欢的音乐': return '我喜欢的音乐';
    case '收藏的专辑': return '收藏的专辑';
    case '收藏的歌手': return '收藏的歌手';
    case '专辑': return '专辑';
    case '艺术家': return '艺术家';
    case '文件夹': return '文件夹';
    case '播放列表': return '播放列表';
    case '智能歌单': return '智能歌单';
    case '设置': return '设置';
    default: return '全部歌曲';
  }
});

const metaText = computed(() => {
  if (playerStore.activeLibraryTab === '专辑') return `${playerStore.albumsTotalCount.toLocaleString()} 张专辑`;
  if (playerStore.activeLibraryTab === '艺术家') return `${playerStore.artistsTotalCount.toLocaleString()} 位艺术家`;
  if (playerStore.activeLibraryTab === '文件夹') return `${playerStore.localSources.length} 个数据源`;
  if (playerStore.activeLibraryTab === '喜欢的音乐') return `${playerStore.libraryCounts.favorite_tracks.toLocaleString()} 首歌曲`;
  if (playerStore.activeLibraryTab === '收藏的专辑') return `${playerStore.libraryCounts.favorite_albums.toLocaleString()} 张专辑`;
  if (playerStore.activeLibraryTab === '收藏的歌手') return `${playerStore.libraryCounts.favorite_artists.toLocaleString()} 位艺术家`;
  return `${playerStore.tracksTotalCount.toLocaleString()} 首歌曲`;
});

/* ============ 视图分支判定 ============ */

// 专辑网格视图（无 activeAlbumId 时）
const isAlbumGridView = computed(() => {
  return playerStore.activeLibraryTab === '专辑' && !playerStore.activeAlbumId;
});

// 专辑详情视图（有 activeAlbumId 时）
const isAlbumDetailView = computed(() => {
  return playerStore.activeLibraryTab === '专辑' && !!playerStore.activeAlbumId;
});

// 歌单详情视图
const isPlaylistDetailView = computed(() => {
  return playerStore.activeLibraryTab === '播放列表' && !!playerStore.activePlaylistId;
});

const isRecentlyPlayedView = computed(() => {
  return playerStore.activeLibraryTab === '最近播放';
});

const isGlobalSearchActive = computed(() => {
  return playerStore.globalSearchQuery.trim().length > 0;
});

// 轨道表格视图
const isTracksView = computed(() => {
  return ['全部歌曲', '播放列表'].includes(playerStore.activeLibraryTab);
});

const isArtistGridView = computed(() => {
  return playerStore.activeLibraryTab === '艺术家' && !playerStore.activeArtistId;
});
const isArtistDetailView = computed(() => {
  return playerStore.activeLibraryTab === '艺术家' && !!playerStore.activeArtistId;
});
const isFolderView = computed(() => {
  return playerStore.activeLibraryTab === '文件夹';
});

const isFavoriteTracksView = computed(() => {
  return playerStore.activeLibraryTab === '喜欢的音乐';
});

const isFavoriteAlbumsView = computed(() => {
  return playerStore.activeLibraryTab === '收藏的专辑';
});

const isFavoriteArtistsView = computed(() => {
  return playerStore.activeLibraryTab === '收藏的歌手';
});

/* ============ 轨道列表相关 ============ */
function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

/* ============ 可播性（离线降级） ============ */
// 列表变化或可播性失效（扫描/同步恢复）时批量拉取；离线时 Remote 未缓存与 Unavailable 的行置灰
watch(
  [() => playerStore.tracks.map(t => t.id), () => playerStore.playabilityEpoch],
  ([ids]) => { if (ids.length > 0) playerStore.ensurePlayability(ids); },
  { immediate: true },
);

function isTrackGreyed(trackId: number): boolean {
  return playerStore.isTrackUnplayable(trackId);
}

function isOfflineRemote(trackId: number): boolean {
  return !uiStore.isOnline && playerStore.getPlayability(trackId) === 'remote';
}

function playSong(index: number) {
  playerStore.playTrack(index);
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

/* ============ 批量选择（全选以已加载列表为准） ============ */
const isAllSelected = computed(() => batch.count > 0 && batch.count === playerStore.tracks.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(playerStore.tracks);
}

/* ============ 虚拟列表 ============ */
const ROW_HEIGHT = 40;
const scrollContainer = ref<HTMLElement | null>(null);
const { totalHeight, offsetY, visibleItems } = useVirtualList({
  containerRef: scrollContainer,
  items: computed(() => playerStore.tracks) as any,
  itemHeight: ROW_HEIGHT,
  buffer: 8,
});

/* ============ 无限加载更多 ============ */
function onListScroll() {
  const el = scrollContainer.value;
  if (!el) return;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 400) {
    if (playerStore.hasMoreTracks && !playerStore.isLoadingTracks) {
      playerStore.fetchTracks();
    }
  }
}

/* ============ 专辑选择 / 返回 ============ */
function onAlbumSelect(album: Album) {
  playerStore.activeAlbumId = album.id;
}

/* ============ Tab 切换时重新拉取数据 ============ */
function loadForCurrentTab() {
  const tab = playerStore.activeLibraryTab;
  if (tab === '首页') return; // 首页自管数据
  if (tab === '最近播放') playerStore.fetchRecentlyPlayed();
  else if (tab === '喜欢的音乐') playerStore.fetchFavoriteTracks();
  else if (tab === '收藏的专辑') playerStore.fetchFavoriteAlbums();
  else if (tab === '收藏的歌手') playerStore.fetchFavoriteArtists();
  else if (tab === '专辑') playerStore.fetchAlbums(true);
  else if (tab === '播放列表' && playerStore.activePlaylistId) return;
  else if (tab === '艺术家' && playerStore.activeArtistId) return; // 详情数据由 watch(activeArtistId) 加载
  else if (tab === '艺术家') playerStore.fetchArtists(true); // 艺术家网格：按关键词刷新网格
  else playerStore.fetchTracks(true);
}
watch(() => playerStore.activeLibraryTab, () => {
  // 切换视图即切换列表：清掉上个视图的过滤词，避免残留污染（P0 级体验修正）
  if (searchInput.value || playerStore.searchQuery) {
    searchInput.value = '';
    playerStore.searchQuery = '';
  }
  // 切换 tab 自动退出多选（本视图跨 全部歌曲/播放列表 常驻，须显式退出）
  batch.exit();
  loadForCurrentTab();
});
// 播放列表详情切换也退出多选，避免选择集跨歌单串扰
watch(() => playerStore.activePlaylistId, () => batch.exit());

onMounted(() => {
  // 仅在还没有数据时首次拉取，避免覆盖 restoreSession 的状态
  if (playerStore.tracks.length === 0 && playerStore.albums.length === 0) {
    loadForCurrentTab();
  }
  // 拉取收藏列表，用于详情页收藏按钮状态
  playerStore.fetchFavoriteAlbums();
  playerStore.fetchFavoriteArtists();
});
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- ============ 全局搜索视图（覆盖所有其他视图） ============ -->
    <template v-if="isGlobalSearchActive">
      <GlobalSearch />
    </template>

    <!-- ============ 专辑详情视图（独占整个 Content Area） ============ -->
    <template v-else-if="isAlbumDetailView">
      <AlbumDetail
        :album-id="playerStore.activeAlbumId"
      />
    </template>

    <!-- ============ 歌单详情视图 ============ -->
    <PlaylistDetail v-else-if="isPlaylistDetailView" />

    <!-- ============ 设置页 ============ -->
    <Settings v-else-if="playerStore.activeLibraryTab === '设置'" />

    <!-- ============ 智能歌单 ============ -->
    <SmartPlaylistView v-else-if="playerStore.activeLibraryTab === '智能歌单'" />

    <!-- ============ 首页（自管数据，无共享 Header/Toolbar） ============ -->
    <HomeView v-else-if="playerStore.activeLibraryTab === '首页'" />

    <!-- ============ 其他视图（共享 Header + Toolbar） ============ -->
    <template v-else>

      <!-- Header -->
      <div class="px-8 pt-6 pb-0 flex-shrink-0" data-tauri-drag-region>
        <div class="flex items-end justify-between mb-2">
          <div>
            <!-- LDL Page Title = 42px -->
            <h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none mb-2">{{ pageTitle }}</h1>
            <p class="text-[12px] text-text-muted leading-relaxed font-mono">{{ metaText }}</p>
          </div>
          <div class="relative w-[240px]">
            <Search class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              v-model="searchInput"
              @input="onSearchInput"
              type="text"
              placeholder="搜索歌曲、艺术家、专辑…"
              class="w-full h-[32px] pl-8 pr-3 text-[12px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:border-brand-orange/50"
            />
          </div>
        </div>
      </div>

      <!-- Page Toolbar（仅轨道视图显示；修复非轨道视图残留一条空 padding 行的旧布局问题） -->
      <div v-if="isTracksView" class="px-8 py-3 flex items-center justify-end flex-shrink-0">
        <div class="flex items-center gap-2">
          <!-- 批量选择入口 -->
          <button
            class="h-7 px-3 rounded-[6px] text-[12px] border border-border-color transition-colors-smooth"
            :class="batch.isActive ? 'bg-list-selected text-text-primary border-transparent' : 'text-text-secondary hover:bg-list-hover'"
            :title="batch.isActive ? '退出多选' : '多选歌曲'"
            @click="batch.isActive ? batch.exit() : batch.enter()"
          >
            <span class="flex items-center gap-1.5">
              <CheckSquare class="w-3.5 h-3.5" />
              {{ batch.isActive ? '取消多选' : '多选' }}
            </span>
          </button>

          <!-- 视图切换 -->
          <div class="flex items-center gap-0 bg-bg-canvas border border-border-color rounded-[8px] p-[2px]">
            <button
              class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
              :class="viewMode === 'list' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
              @click="viewMode = 'list'"
              title="列表视图"
            >
              <List class="w-[14px] h-[14px]" />
            </button>
            <button
              class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
              :class="viewMode === 'grid' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
              @click="viewMode = 'grid'"
              title="网格视图"
            >
              <LayoutGrid class="w-[14px] h-[14px]" />
            </button>
          </div>
        </div>
      </div>

      <!-- ============ 专辑网格视图 ============ -->
      <AlbumGrid
        v-if="isAlbumGridView"
        @select="onAlbumSelect"
      />

      <!-- ============ 轨道表格视图 ============ -->
      <template v-else-if="isTracksView">
        <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8" @scroll="onListScroll">

          <!-- 表头（sticky） -->
          <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
            <div class="w-10 text-center shrink-0">#</div>
            <div class="w-8 shrink-0"></div>
            <div class="flex-[2] min-w-0 pl-1">标题</div>
            <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
            <div class="flex-[1.5] min-w-0 hidden md:block">专辑</div>
            <div class="w-[56px] text-right shrink-0 hidden lg:block">时长</div>
            <div class="w-[50px] text-center shrink-0 hidden lg:block">格式</div>
            <div class="w-8 shrink-0"></div>
          </div>

          <!-- 加载态（首次）骨架屏 -->
          <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length === 0" class="py-2">
            <div
              v-for="i in SKELETON_ROWS"
              :key="'skel-' + i"
              class="flex items-center animate-pulse"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <div class="w-10 text-center shrink-0 flex justify-center">
                <div class="w-4 h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="w-8 shrink-0 flex justify-center">
                <div class="w-3.5 h-3.5 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="flex-[2] min-w-0 pl-1">
                <div class="w-[60%] max-w-[200px] h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="flex-[1.5] min-w-0 hidden sm:block">
                <div class="w-[70%] max-w-[140px] h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="flex-[1.5] min-w-0 hidden md:block">
                <div class="w-[65%] max-w-[150px] h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="w-[56px] shrink-0 hidden lg:block flex justify-end">
                <div class="w-8 h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="w-[50px] shrink-0 hidden lg:block flex justify-center">
                <div class="w-6 h-3 rounded-[3px] skeleton-bg"></div>
              </div>
              <div class="w-8 shrink-0"></div>
            </div>
          </div>

          <!-- 空态 -->
          <div v-else-if="playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <Music class="w-8 h-8 text-text-disabled" />
            <span class="text-[12px]">没有找到歌曲</span>
          </div>

          <!-- 错误态 -->
          <div v-else-if="playerStore.isErrorTracks" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <span class="text-[12px]">加载失败，请稍后重试</span>
          </div>

          <!-- 虚拟列表 -->
          <div v-else :style="{ height: totalHeight + 'px', position: 'relative' }">
            <div :style="{ transform: `translateY(${offsetY}px)` }">
              <div
                v-for="{ index, data: song } in visibleItems"
                :key="song.id"
                class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
                :style="{ height: ROW_HEIGHT + 'px' }"
                :class="{
                  'playing-row bg-list-selected': isPlayingTrack(song.id),
                  'bg-list-selected/60': batch.isActive && batch.isSelected(song.id),
                }"
                @click="batch.isActive && batch.toggle(song)"
                @dblclick="!batch.isActive && playSong(index)"
              >
                <!-- 序号 / 复选框 / 播放图标 -->
                <div class="w-10 text-center shrink-0 text-[12px] font-mono">
                  <!-- 多选态：序号列换成复选框 -->
                  <span
                    v-if="batch.isActive"
                    class="inline-flex items-center justify-center"
                    @click.stop="batch.toggle(song)"
                  >
                    <span
                      class="w-[14px] h-[14px] rounded-[3px] border flex items-center justify-center transition-colors-smooth"
                      :class="batch.isSelected(song.id) ? 'bg-brand-orange border-brand-orange' : 'border-border-solid'"
                    >
                      <CheckSquare v-if="batch.isSelected(song.id)" class="w-[10px] h-[10px] text-white" />
                    </span>
                  </span>
                  <template v-else>
                    <span v-if="isPlayingTrack(song.id)" class="text-brand-orange inline-flex items-center justify-center">
                      <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
                      <Play v-else class="w-[12px] h-[12px] fill-current" />
                    </span>
                    <template v-else>
                      <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
                      <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
                    </template>
                  </template>
                </div>

                <!-- 收藏（多选态下改为切换选择） -->
                <div class="w-8 shrink-0 flex items-center justify-center">
                  <template v-if="batch.isActive">
                    <span
                      class="w-[14px] h-[14px] rounded-[3px] border flex items-center justify-center transition-colors-smooth"
                      :class="batch.isSelected(song.id) ? 'bg-brand-orange border-brand-orange' : 'border-border-solid opacity-0 group-hover:opacity-100'"
                      @click.stop="batch.toggle(song)"
                    >
                      <CheckSquare v-if="batch.isSelected(song.id)" class="w-[10px] h-[10px] text-white" />
                    </span>
                  </template>
                  <template v-else>
                    <Heart
                      v-if="song.isFavorite"
                      class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                      @click="toggleFav(song.id, $event)"
                    />
                    <Heart
                      v-else
                      class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                      @click="toggleFav(song.id, $event)"
                    />
                  </template>
                </div>

                <!-- 标题 -->
                <div class="flex-[2] min-w-0 pl-1">
                  <span class="text-[13px] truncate block" :class="isPlayingTrack(song.id) ? 'text-brand-orange font-semibold' : isTrackGreyed(song.id) ? 'text-text-disabled' : 'text-text-primary font-medium'">
                    {{ song.title }}
                  </span>
                </div>

                <!-- 艺术家（多选态下改为切换选择） -->
                <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] truncate" :class="isTrackGreyed(song.id) ? 'text-text-disabled' : 'text-text-secondary'">
                  <span class="hover:underline cursor-pointer" @click.stop="batch.isActive ? batch.toggle(song) : playerStore.navigateToArtist(song.artistId)">{{ song.artist }}</span>
                </div>

                <!-- 专辑（非斜体；多选态下改为切换选择） -->
                <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] truncate" :class="isTrackGreyed(song.id) ? 'text-text-disabled' : 'text-text-secondary'">
                  <span class="hover:underline cursor-pointer" @click.stop="batch.isActive ? batch.toggle(song) : playerStore.navigateToAlbum(song.albumId)">{{ song.album }}</span>
                </div>

                <!-- 时长 -->
                <div class="w-[56px] text-right shrink-0 hidden lg:block text-[12px] font-mono text-text-muted tabular-nums">{{ song.duration }}</div>

                <!-- 格式 -->
                <div class="w-[50px] text-center shrink-0 hidden lg:block">
                  <span class="text-[10px] font-mono uppercase" :class="isTrackGreyed(song.id) ? 'text-text-disabled' : 'text-text-muted'">{{ song.format }}</span>
                </div>

                <!-- 离线不可播标记（纯云端未缓存 / 本地文件丢失） -->
                <div v-if="isTrackGreyed(song.id)" class="w-8 shrink-0 flex items-center justify-center" :title="isOfflineRemote(song.id) ? '离线且未缓存' : '文件不可用'">
                  <CloudOff class="w-3.5 h-3.5 text-text-disabled" />
                </div>

                <!-- more -->
                <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity" :class="{ 'hidden': isTrackGreyed(song.id) }">
                  <MoreHorizontal class="w-4 h-4 text-text-muted" />
                </div>
              </div>
            </div>
          </div>

          <!-- 增量加载指示 -->
          <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length > 0" class="flex items-center justify-center py-4 text-text-muted">
            <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
            <span class="text-[11px]">加载更多…</span>
          </div>

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

      <!-- ============ 艺术家网格视图 ============ -->
      <ArtistGrid v-if="isArtistGridView" />

      <!-- ============ 艺术家详情视图 ============ -->
      <ArtistDetail v-if="isArtistDetailView" :artist-id="playerStore.activeArtistId" :filter-query="searchInput" />

      <!-- ============ 文件夹视图 ============ -->
      <FolderView v-if="isFolderView" />

      <!-- ============ 最近播放视图 ============ -->
      <RecentlyPlayed v-if="isRecentlyPlayedView" :filter-query="searchInput" />

      <!-- ============ 喜欢的音乐视图 ============ -->
      <FavoritesView v-if="isFavoriteTracksView" :filter-query="searchInput" />

      <!-- ============ 收藏的专辑视图 ============ -->
      <FavoriteAlbums v-if="isFavoriteAlbumsView" />

      <!-- ============ 收藏的歌手视图 ============ -->
      <FavoriteArtists v-if="isFavoriteArtistsView" />

    </template>
  </div>
</template>
