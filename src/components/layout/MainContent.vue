<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import {
  Search, List, LayoutGrid, Loader2, Music, CheckSquare, Filter,
} from 'lucide-vue-next';

const SKELETON_ROWS = 8;
import { usePlayerStore, ALBUM_MIN_TRACK_COUNT, ARTIST_MIN_TRACK_COUNT, type Album, type Track } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useVirtualList } from '../../composables/useVirtualList';
import { useBatchSelect } from '../../composables/useBatchSelect';
import { useScrollRestore } from '../../composables/useScrollRestore';
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
import AiPlaylistView from '../content/AiPlaylistView.vue';
import MinimalEntityList from '../content/MinimalEntityList.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import { TRACK_ROW_HEIGHT, columnCellStyle } from '../shared/trackList/columns';
import { useDesktopModeStore } from '../../stores/desktopMode';
import { libraryGetTrackIds } from '../../api/library';

const playerStore = usePlayerStore();
const uiStore = useUiStore();
// 极简体验（DM-04）：false 时封面网格替换为文字列表
const desktopMode = useDesktopModeStore();
const visualAllowed = computed(() => desktopMode.visualAllowed);

/* ============ 极简文字列表数据（DM-04：与网格同一份 store 数据，不复制加载逻辑） ============ */
const minimalAlbumItems = computed(() =>
  playerStore.albums.map(a => ({
    id: a.id,
    primary: a.title,
    secondary: a.artist,
    hint: a.track_count != null ? `${a.track_count} 首` : undefined,
  })),
);
const minimalArtistItems = computed(() =>
  playerStore.artists.map(a => ({
    id: a.id,
    primary: a.name,
    hint: (a.track_count ?? a.trackCount) != null ? `${a.track_count ?? a.trackCount} 首` : undefined,
  })),
);
const minimalFavoriteAlbumItems = computed(() =>
  playerStore.favoriteAlbums.map(a => ({
    id: a.id,
    primary: a.title,
    secondary: a.artist,
    hint: a.track_count != null ? `${a.track_count} 首` : undefined,
  })),
);
const minimalFavoriteArtistItems = computed(() =>
  playerStore.favoriteArtists.map(a => ({
    id: a.id,
    primary: a.name,
    hint: (a.track_count ?? a.trackCount) != null ? `${a.track_count ?? a.trackCount} 首` : undefined,
  })),
);

function onMinimalArtistSelect(id: number) {
  playerStore.activeArtistId = id;
}
function onMinimalFavoriteAlbumSelect(id: number) {
  playerStore.activeAlbumId = id;
}
function onMinimalFavoriteArtistSelect(id: number) {
  playerStore.activeArtistId = id;
}

/* ============ 统一歌曲列表列配置（容器宽度驱动，见 trackList/columns.ts） ============ */
const trackColumnsContext = computed(() => ({}));

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
    case 'AI 电台': return 'AI 电台';
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
  if (playerStore.activeLibraryTab === '播放列表' && !playerStore.activePlaylistId) {
    // 播放列表（未选中具体歌单）= 当前正在播放的队列
    return `${playerStore.queue.length.toLocaleString()} 首歌曲 · 当前播放队列`;
  }
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

/**
 * 「播放列表」一级入口（未选中具体歌单）展示的是**当前播放队列**，
 * 与右侧面板的「播放列表」标签同一份数据，只是铺满内容区。
 */
const isQueueView = computed(() => {
  return playerStore.activeLibraryTab === '播放列表' && !playerStore.activePlaylistId;
});

/**
 * 队列视图的搜索过滤结果 + 对应的原始队列下标（两个数组同序）。
 * 过滤后行号与播放位置仍要指向队列里的真实位置，所以要带回原始下标。
 */
const queueFilter = computed(() => {
  const q = playerStore.searchQuery.trim().toLowerCase();
  const list: Track[] = [];
  const idx: number[] = [];
  playerStore.queue.forEach((t, i) => {
    if (!q || t.title.toLowerCase().includes(q) || t.artist.toLowerCase().includes(q) || t.album.toLowerCase().includes(q)) {
      list.push(t);
      idx.push(i);
    }
  });
  return { list, idx };
});

/** 渲染行下标 → 原始队列下标 */
function queueOriginalIndex(renderedIndex: number): number {
  return queueFilter.value.idx[renderedIndex] ?? renderedIndex;
}

/** 表格视图实际渲染的数据源：播放列表页 = 播放队列（可过滤），其余 = 曲库列表 */
const displayTracks = computed(() => isQueueView.value ? queueFilter.value.list : playerStore.tracks);

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
  [() => displayTracks.value.map(t => t.id), () => playerStore.playabilityEpoch],
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
  if (isQueueView.value) {
    // 队列行：按原始队列下标继续播（过滤后下标会错位，必须映射回去）
    playerStore.playQueue(playerStore.queue, queueOriginalIndex(index));
    return;
  }
  playerStore.playTrack(index);
}

/* ============ 批量选择（全选以已加载列表为准；筛选后=完整结果集，DM-05 验收 4） ============ */
const isAllSelected = computed(() => {
  if (batch.resultWide) return true;
  return batch.count > 0 && batch.count === displayTracks.value.length;
});
const isSelectAllBusy = ref(false);
async function onToggleSelectAll() {
  if (isAllSelected.value) {
    batch.selectNone();
    return;
  }
  const q = playerStore.searchQuery.trim();
  if (q && !isQueueView.value) {
    // 筛选后全选：从后端拉取完整结果集 ID（低频显式动作），不缩为已加载窗口
    if (isSelectAllBusy.value) return;
    isSelectAllBusy.value = true;
    try {
      batch.selectAllIds(await libraryGetTrackIds(q));
    } catch {
      // 可理解回退（验收 5）：完整结果集获取失败时退回已加载窗口并提示
      batch.selectAll(displayTracks.value);
      uiStore.showToast('完整结果集获取失败，已选中已加载部分', 'error');
    } finally {
      isSelectAllBusy.value = false;
    }
    return;
  }
  batch.selectAll(displayTracks.value);
}

/* ============ 虚拟列表 ============ */
// 行高常量与 TrackRow 共用同一来源（虚拟列表占位高度计算依赖固定行高）
const ROW_HEIGHT = TRACK_ROW_HEIGHT;
// 滚动位置记忆：全部歌曲 / 播放列表（队列）各记一份
const scrollContainer = useScrollRestore(
  () => `tracks:${playerStore.activeLibraryTab}:${playerStore.searchQuery}`
);
const { totalHeight, offsetY, visibleItems } = useVirtualList({
  containerRef: scrollContainer,
  items: displayTracks as any,
  itemHeight: ROW_HEIGHT,
  buffer: 8,
});

/* ============ 统一列解析（容器宽度 + 用户列偏好） ============ */
const { resolvedColumns, menuColumns, trailingExtraWidth } = useTrackColumns({
  containerRef: scrollContainer,
  context: trackColumnsContext,
});

/* ============ 无限加载更多 ============ */
function onListScroll() {
  const el = scrollContainer.value;
  if (!el) return;
  // 队列是内存数据，没有分页
  if (isQueueView.value) return;
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
  if (tab === 'AI 电台') return; // AI 电台自管数据
  if (tab === '播放列表') return; // 未选歌单时展示播放队列（内存），详情数据由 store watcher 负责
  if (tab === '最近播放') playerStore.fetchRecentlyPlayed();
  else if (tab === '喜欢的音乐') playerStore.fetchFavoriteTracks();
  else if (tab === '收藏的专辑') playerStore.fetchFavoriteAlbums();
  else if (tab === '收藏的歌手') playerStore.fetchFavoriteArtists();
  else if (tab === '专辑') playerStore.fetchAlbums(true);
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
  // 历史前进/后退回到本页：数据仍在内存，重拉会把分页与滚动高度清掉，跳过这次加载
  if (playerStore.isHistoryRestore) {
    playerStore.isHistoryRestore = false;
    return;
  }
  loadForCurrentTab();
});
// 播放列表详情切换也退出多选，避免选择集跨歌单串扰
watch(() => playerStore.activePlaylistId, () => batch.exit());

onMounted(() => {
  // 迷你返回后的浏览恢复（DM-05）：锚点窗口播种替代默认加载
  if (playerStore.pendingBrowseRestore) {
    void playerStore.applyBrowseRestore();
  } else if (playerStore.tracks.length === 0 && playerStore.albums.length === 0) {
    // 仅在还没有数据时首次拉取，避免覆盖 restoreSession 的状态
    loadForCurrentTab();
  }
  // 恢复搜索框文本（快照恢复把 searchQuery 写回了 store）
  if (playerStore.searchQuery) searchInput.value = playerStore.searchQuery;
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

    <!-- ============ AI 电台（自管数据） ============ -->
    <AiPlaylistView v-else-if="playerStore.activeLibraryTab === 'AI 电台'" />

    <!-- ============ 其他视图（共享 Header + Toolbar） ============ -->
    <template v-else>

      <!-- Header -->
      <div class="px-8 pt-6 pb-0 flex-shrink-0">
        <div class="flex items-end justify-between mb-2">
          <div>
            <!-- LDL v2 页面标题 28px（1200×720 基线） -->
            <h1 class="text-(--text-page-title) font-bold text-text-primary tracking-tight leading-none mb-2">{{ pageTitle }}</h1>
            <p class="text-[12px] text-text-muted leading-relaxed font-mono">{{ metaText }}</p>
          </div>
          <div class="flex items-center gap-2 flex-shrink-0">
            <!-- 专辑/艺人网格过滤开关：隐藏低曲目数条目（选择持久化在 store） -->
            <button
              v-if="isAlbumGridView"
              class="h-[32px] px-3 rounded-[8px] text-[12px] border transition-colors-smooth whitespace-nowrap"
              :class="playerStore.hideSmallAlbums ? 'bg-list-selected text-text-primary border-transparent' : 'text-text-secondary border-border-color hover:bg-list-hover'"
              :title="`不加载曲目数少于 ${ALBUM_MIN_TRACK_COUNT} 首的专辑（过滤单曲/零星收录）`"
              @click="playerStore.toggleHideSmallAlbums()"
            >
              <span class="flex items-center gap-1.5">
                <Filter class="w-3.5 h-3.5" />
                隐藏少于{{ ALBUM_MIN_TRACK_COUNT }}首的专辑
              </span>
            </button>
            <button
              v-if="isArtistGridView"
              class="h-[32px] px-3 rounded-[8px] text-[12px] border transition-colors-smooth whitespace-nowrap"
              :class="playerStore.hideMinorArtists ? 'bg-list-selected text-text-primary border-transparent' : 'text-text-secondary border-border-color hover:bg-list-hover'"
              :title="`不加载曲目数少于 ${ARTIST_MIN_TRACK_COUNT} 首的艺术家（过滤零星合作艺人）`"
              @click="playerStore.toggleHideMinorArtists()"
            >
              <span class="flex items-center gap-1.5">
                <Filter class="w-3.5 h-3.5" />
                隐藏少于{{ ARTIST_MIN_TRACK_COUNT }}首的艺术家
              </span>
            </button>
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

      <!-- ============ 专辑网格视图（极简换文字列表，数据/点击行为不变） ============ -->
      <AlbumGrid
        v-if="isAlbumGridView && visualAllowed"
        @select="onAlbumSelect"
      />
      <MinimalEntityList
        v-else-if="isAlbumGridView"
        kind="album"
        :items="minimalAlbumItems"
        :scroll-key="`albums:${playerStore.searchQuery}`"
        empty-text="没有找到专辑"
        @select="onMinimalFavoriteAlbumSelect"
      />

      <!-- ============ 轨道表格视图 ============ -->
      <template v-else-if="isTracksView">
        <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8" @scroll="onListScroll">

          <!-- 统一表头（列配置与行共用；右端含显示列菜单） -->
          <TrackListHeader :columns="resolvedColumns" :menu-columns="menuColumns" />

          <!-- 加载态（首次）骨架屏（播放队列是内存数据，不需要骨架屏） -->
          <div v-if="!isQueueView && playerStore.isLoadingTracks && displayTracks.length === 0" class="py-2">
            <div
              v-for="i in SKELETON_ROWS"
              :key="'skel-' + i"
              class="flex items-center animate-pulse"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <template v-for="col in resolvedColumns" :key="'skel-' + i + '-' + col.id">
                <div :style="columnCellStyle(col)" class="shrink-0 min-w-0 flex px-0" :class="col.align === 'right' ? 'justify-end' : col.align === 'center' ? 'justify-center' : 'justify-start'">
                  <div class="h-3 rounded-[3px] skeleton-bg" :class="col.id === 'title' ? 'w-[60%] max-w-[200px]' : col.kind === 'flex' ? 'w-[70%] max-w-[140px]' : 'w-8'"></div>
                </div>
              </template>
            </div>
          </div>

          <!-- 空态 -->
          <div v-else-if="displayTracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <Music class="w-8 h-8 text-text-disabled" />
            <span class="text-[12px]">{{ isQueueView ? '播放队列为空' : '没有找到歌曲' }}</span>
          </div>

          <!-- 错误态（播放队列无后端加载，不展示错误态） -->
          <div v-else-if="!isQueueView && playerStore.isErrorTracks" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <span class="text-[12px]">加载失败，请稍后重试</span>
          </div>

          <!-- 虚拟列表 -->
          <div v-else :style="{ height: totalHeight + 'px', position: 'relative' }">
            <div :style="{ transform: `translateY(${offsetY}px)` }">
              <TrackRow
                v-for="{ index, data: song } in visibleItems"
                :key="song.id"
                :track="song"
                :columns="resolvedColumns"
                :index="index"
                :display-index="isQueueView ? queueOriginalIndex(index) + 1 : index + 1"
                :playing="isPlayingTrack(song.id)"
                :is-playing-now="playerStore.isPlaying"
                :greyed="isTrackGreyed(song.id)"
                :grey-title="isOfflineRemote(song.id) ? '离线且未缓存' : '文件不可用'"
                :batch-mode="batch.isActive"
                :selected="batch.isSelected(song.id)"
                :trailing-width="trailingExtraWidth"
                @play="playSong(index)"
                @toggle-select="batch.toggle(song)"
              />
            </div>
          </div>

          <!-- 增量加载指示 -->
          <div v-if="!isQueueView && playerStore.isLoadingTracks && displayTracks.length > 0" class="flex items-center justify-center py-4 text-text-muted">
            <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
            <span class="text-[11px]">加载更多…</span>
          </div>

        </div>

        <!-- 批量操作条（多选态） -->
        <BatchActionBar
          v-if="batch.isActive"
          :selected-ids="[...batch.selectedIds]"
          :all-selected="isAllSelected"
          :busy="isSelectAllBusy"
          @exit="batch.exit()"
          @toggle-select-all="onToggleSelectAll"
        />

      </template>

      <!-- ============ 艺术家网格视图（极简换文字列表） ============ -->
      <ArtistGrid v-if="isArtistGridView && visualAllowed" />
      <MinimalEntityList
        v-else-if="isArtistGridView"
        kind="artist"
        :items="minimalArtistItems"
        :scroll-key="`artists:${playerStore.searchQuery}`"
        empty-text="没有找到艺术家"
        @select="onMinimalArtistSelect"
      />

      <!-- ============ 艺术家详情视图 ============ -->
      <ArtistDetail v-if="isArtistDetailView" :artist-id="playerStore.activeArtistId" :filter-query="searchInput" />

      <!-- ============ 文件夹视图 ============ -->
      <FolderView v-if="isFolderView" />

      <!-- ============ 最近播放视图 ============ -->
      <RecentlyPlayed v-if="isRecentlyPlayedView" :filter-query="searchInput" />

      <!-- ============ 喜欢的音乐视图 ============ -->
      <FavoritesView v-if="isFavoriteTracksView" :filter-query="searchInput" />

      <!-- ============ 收藏的专辑视图（极简换文字列表） ============ -->
      <FavoriteAlbums v-if="isFavoriteAlbumsView && visualAllowed" />
      <MinimalEntityList
        v-else-if="isFavoriteAlbumsView"
        kind="album"
        :items="minimalFavoriteAlbumItems"
        scroll-key="favorite-albums"
        empty-text="还没有收藏的专辑"
        @select="onMinimalFavoriteAlbumSelect"
      />

      <!-- ============ 收藏的歌手视图（极简换文字列表） ============ -->
      <FavoriteArtists v-if="isFavoriteArtistsView && visualAllowed" />
      <MinimalEntityList
        v-else-if="isFavoriteArtistsView"
        kind="artist"
        :items="minimalFavoriteArtistItems"
        scroll-key="favorite-artists"
        empty-text="还没有收藏的歌手"
        @select="onMinimalFavoriteArtistSelect"
      />

    </template>
  </div>
</template>
