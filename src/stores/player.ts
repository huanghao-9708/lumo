import { defineStore } from "pinia";
import { ref, computed, watch, shallowRef, reactive } from "vue";
import { listen } from '@tauri-apps/api/event';

import {
  libraryGetTracks, libraryGetAlbums, libraryGetAlbumCount, libraryGetArtists, libraryGetAlbumTracks, libraryGetArtistAlbums, libraryGetArtistAlbumCount, libraryGetArtistTracks, libraryGetArtistStats,
  libraryCreatePlaylist, libraryGetPlaylists, libraryAddToPlaylist, libraryGetPlaylistTracks, libraryDeletePlaylist, libraryRemovePlaylistItem, libraryAddFolderToPlaylist,
  libraryToggleFavorite, libraryRecordPlay, libraryGetRecentlyPlayed, libraryGetFavoriteTracks, libraryGetFavoriteAlbums, libraryGetFavoriteArtists, libraryToggleFavoriteAlbum, libraryToggleFavoriteArtist, librarySavePlayQueue, libraryGetPlayQueue,
  libraryGetFolderContents, libraryGetFolderChildren, libraryGetFolderTracks,
  libraryGetLyrics, libraryGetTrackFileInfo, libraryGetCounts,
  libraryFetchMissingAlbumCover, libraryFetchMissingArtistCover,
  libraryGetAlbumById,
  libraryGetArtistById,
  libraryGetSmartPlaylist,
  libraryGetStats, libraryGetInsights, libraryAddTracksToPlaylist, librarySetFavoriteBatch,
  libraryGetStartupBundle,
} from '../api/library';
import {
  playbackPlay, playbackPause, playbackResume, playbackSetVolume, playbackSeek,
  playbackSetSpeed,
} from '../api/playback';
import {
  sourceAddLocal, sourceAddWebdav, sourceList, sourceRemove, sourceScan,
  type ScanCompleteEvent
} from '../api/scanner';
import {
  playbackSetQueue, playbackPlayIndex, playbackQueueState, playbackAdvance, playbackSetMode,
  type QueueItemDTO, type BackendPlayMode
} from '../api/queue';
import { libraryGetPlayability, type PlayabilityState } from '../api/library';
import { useUiStore } from './ui';
import { getArtworkUrl } from '../utils';


// ================= 后端 DTO 接口（与 Rust 端 models.rs 保持一致） =================
import type { TrackDTO, ArtistDTO, AlbumDTO, PlaylistDTOBackend, FolderChildrenResultDTO, FolderTracksResultDTO, LibraryStatsDTO, RankedTrackDTO, RankedArtistDTO, RankedAlbumDTO, StartupBundleDTO } from '../api/types';

// ================= 前端展示模型 =================

export interface Track {
  id: number;
  title: string;
  artistId: number | null;
  artist: string;
  albumId: number | null;
  album: string;
  duration: string;
  durationSec: number;
  format: string;
  coverColor: string;
  cover_artwork_id?: number | null;
  isFavorite: boolean;
  primary_file_id?: number | null;
  playedAt?: string;
  fileSize: number | null;
  /** 来源类型：'local' | 'webdav'，用于离线降级判断 */
  sourceKind: 'local' | 'webdav';
}

export interface Playlist {
  id: number;
  name: string;
  count: number;
  description?: string | null;
  /** 歌单封面 = 歌单内第一首歌曲所属专辑的封面（artwork ID） */
  cover_artwork_id?: number | null;
  /** 封面的 200x200 缩略图 data URL；有值时直接 <img src> 渲染 */
  cover_thumb?: string | null;
}

export interface Album {
  id: number;
  title: string;
  artist: string;
  year: number;
  coverColor: string;
  cover_artwork_id?: number | null;
  /** 200x200 缩略图的 base64 data URL。
   *  有值时前端直接用 <img src> 渲染，不再走 lumo://artwork 协议。
   *  无值时 fallback 到 ArtworkImage 组件（走原协议）。 */
  cover_thumb?: string | null;
  artist_name?: string | null;
  track_count?: number;
}

export interface Artist {
  id: number;
  name: string;
  trackCount: number;
  avatarColor: string;
  track_count?: number;
  avatar_artwork_id?: number | null;
}

export interface MusicSource {
  id: number;
  kind: 'local' | 'webdav';
  name: string;
  path: string;
  isEnabled: boolean;
  lastScanned: string;
  /** 上次扫描的错误说明；为空表示上次扫描成功 */
  lastError?: string;
  username?: string;
}

export interface FolderEntry {
  name: string;
  is_dir: boolean;
  path: string;
  track?: Track;
}

// ================= 首页洞察模型（排行榜 / 统计） =================

/** 排行榜歌曲：Track + 播放次数 */
export interface RankedTrack extends Track {
  playCount: number;
}

export interface RankedArtist {
  id: number;
  name: string;
  /** 名下所有歌曲累计播放次数 */
  playCount: number;
  /** 名下歌曲总数 */
  trackCount: number;
  avatarArtworkId?: number | null;
}

export interface RankedAlbum {
  id: number;
  title: string;
  artist: string;
  coverArtworkId?: number | null;
  /** 专辑内所有歌曲累计播放次数 */
  playCount: number;
}

/** library_get_insights 的前端形态（首页一次 IPC 拿全） */
export interface HomeInsights {
  topPlayedTracks: RankedTrack[];
  recentPlayedTracks: RankedTrack[];
  recentAddedTracks: RankedTrack[];
  favoriteTracks: RankedTrack[];
  topPlayedArtists: RankedArtist[];
  topPlayedAlbums: RankedAlbum[];
  todayPlayCount: number;
  lastPlayed: RankedTrack | null;
}

// ================= 详情页数据接口 =================

import type { ArtistStatsDTO, TrackFileInfoDTO, SourceDTO } from '../api/types';

interface AlbumDetails extends Album {
  tracks: Track[];
}

interface ArtistDetails extends Artist {
  stats: ArtistStatsDTO;
  tracks: Track[];
  albums: Album[];
  /** 详情页子标签：'tracks' 全部歌曲 / 'albums' 全部专辑。
   *  提升到 store 而非组件局部状态，返回详情页时才能回到原来的分栏。 */
  subTab: 'tracks' | 'albums';
  tracksOffset: number;
  albumsOffset: number;
  hasMoreTracks: boolean;
  isLoadingTracks: boolean;
  albumsCurrentPage: number;
  albumsTotalCount: number;
  albumsTotalPages: number;
  hasMoreAlbums: boolean;
  isLoadingAlbums: boolean;
}

interface PlaylistDetails extends Playlist {
  tracks: Track[];
  isLoadingTracks: boolean;
}

// 列表过滤阈值：开关打开时，曲目数低于该值的专辑/艺人不会被加载
export const ALBUM_MIN_TRACK_COUNT = 3;
export const ARTIST_MIN_TRACK_COUNT = 10;

function toBackendPlayMode(mode: 'normal' | 'repeat' | 'repeat-one' | 'shuffle'): BackendPlayMode {
  switch (mode) {
    case 'repeat': return 'repeatAll';
    case 'repeat-one': return 'repeatOne';
    case 'shuffle': return 'shuffle';
    case 'normal':
    default:
      return 'normal';
  }
}

function fromBackendPlayMode(mode: BackendPlayMode): 'normal' | 'repeat' | 'repeat-one' | 'shuffle' {
  switch (mode) {
    case 'repeatAll': return 'repeat';
    case 'repeatOne': return 'repeat-one';
    case 'shuffle': return 'shuffle';
    case 'normal':
    default:
      return 'normal';
  }
}

// ================= Store 实现 =================

export const usePlayerStore = defineStore("player", () => {
  function getDeterministicColor(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const colors = [
      'from-warm-500 to-warm-800',
      'from-blue-500 to-blue-800',
      'from-green-500 to-green-800',
      'from-red-500 to-red-800',
      'from-purple-500 to-purple-800',
      'from-indigo-500 to-indigo-800',
      'from-pink-500 to-pink-800',
      'from-teal-500 to-teal-800',
      'from-orange-500 to-orange-800'
    ];
    return colors[Math.abs(hash) % colors.length];
  }

  /** 把秒数格式化成 `mm:ss`，用于 UI 显示 */
  function formatTime(seconds: number): string {
    const min = Math.floor(seconds / 60);
    const sec = Math.floor(seconds % 60);
    return `${min.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
  }

  /**
   * 统一的 DTO → 前端 Track 映射器。
   * 之前这段逻辑在 7+ 处 fetch 函数里几乎一字不差地重复，难以维护，
   * 现在抽到这里，所有 `TrackDTO[]` 来源都走同一通道。
   */
  function mapTrackDTO(t: TrackDTO): Track {
    const durationMs = t.duration_ms ?? 0;
    return {
      id: t.id,
      title: t.title,
      artistId: t.artist_id || null,
      artist: t.artist_name || '未知艺人',
      albumId: t.album_id || null,
      album: t.album_title || '未知专辑',
      duration: formatTime(durationMs / 1000),
      durationSec: Math.floor(durationMs / 1000),
      format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
      coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
      cover_artwork_id: t.cover_artwork_id,
      isFavorite: t.is_favorite || false,
      primary_file_id: t.media_file_id,
      playedAt: t.last_played_at ?? '',
      fileSize: t.file_size ?? null,
      sourceKind: (t.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
    };
  }

  /** 把 `TrackDTO[]` 批量映射为前端 Track[] */
  function mapTrackList(list: TrackDTO[]): Track[] {
    return list.map(mapTrackDTO);
  }

  // 基础状态
  const isPlaying = ref(false);
  const isBuffering = ref(false);
  const volume = ref(75);

  /** 可选的播放速率档位（UI 与校验共用） */
  const PLAYBACK_RATES = [0.5, 0.8, 1, 1.2, 1.5] as const;
  /** 播放速率（1 = 原速）。持久化在 localStorage，切歌与重启都保持。 */
  const playbackRate = ref<number>(1);

  const queue = ref<Track[]>([]);
  const currentIndex = ref(-1);
  const playMode = ref<'normal'|'repeat'|'repeat-one'|'shuffle'>('normal');
  const progressMs = ref(0);
  const durationMs = ref(0);

  const currentTrackFileInfo = ref<TrackFileInfoDTO | null>(null);
  const isErrorTracks = ref(false);
  const isErrorArtists = ref(false);
  const hasLoadedCurrentFile = ref(false);

  // 启动落地页即首页（迭代记录 2.2）；移动端由 MobileLayout 的 immediate watch 纠正回曲库
  const activeLibraryTab = ref("首页");
  const activeSourceTab = ref("本地音乐库");
  const activeRightTab = ref<"歌词" | "播放队列" | "文件信息">("歌词");
  const isRightPanelOpen = ref(true);
  const uiStore = useUiStore();

  // ===== 可播性状态（迭代一：离线降级） =====
  // 每首 track 主文件的播放形态：local 本地 / cached 已缓存 / remote 纯云端未缓存 / unavailable 本地文件丢失。
  // 由 library_get_playability 批量计算；列表视图按需拉取，离线时据此置灰与拦截点击。
  const playability = ref(new Map<number, PlayabilityState>());
  const playabilityEpoch = ref(0);
  let playabilityFetching = false;

  /**
   * 确保给定 track 的可播性已加载（缺失的按 500/批拉取，已在 Map 中的跳过）。
   * 列表视图在列表变化或 epoch 变化（扫描/恢复后失效）时调用。
   */
  async function ensurePlayability(trackIds: number[]) {
    const missing = [...new Set(trackIds)].filter(id => Number.isFinite(id) && !playability.value.has(id));
    if (missing.length === 0 || playabilityFetching) return;
    playabilityFetching = true;
    try {
      const BATCH = 500;
      for (let i = 0; i < missing.length; i += BATCH) {
        const batch = missing.slice(i, i + BATCH);
        const res = await libraryGetPlayability(batch);
        const next = new Map(playability.value);
        for (const [k, v] of Object.entries(res)) {
          next.set(Number(k), v);
        }
        playability.value = next;
      }
    } catch (e) {
      console.error('[Playability] Failed to fetch:', e);
    } finally {
      playabilityFetching = false;
    }
  }

  /** 单曲可播性查询（未加载返回 undefined，调用方按可播处理） */
  function getPlayability(trackId: number | null | undefined): PlayabilityState | undefined {
    if (trackId == null) return undefined;
    return playability.value.get(trackId);
  }

  /** 行级置灰判定：本地文件丢失，或离线时纯云端未缓存 */
  function isTrackUnplayable(trackId: number | null | undefined): boolean {
    const state = getPlayability(trackId);
    if (!state) return false;
    if (state === 'unavailable') return true;
    return !uiStore.isOnline && state === 'remote';
  }

  /** 扫描完成 / 同步恢复后可播性可能变化：清空并推进 epoch，让视图重新拉取 */
  function invalidatePlayability() {
    playability.value = new Map();
    playabilityEpoch.value++;
  }

  /** 播放前的离线守卫：拦截必然失败的请求，给出即时原因而不是无限转圈。返回 true 表示已拦截 */
  function guardPlayback(track: Track | undefined): boolean {
    if (!track) return false;
    const state = getPlayability(track.id);
    if (state === 'unavailable') {
      uiStore.showToast('该歌曲的本地文件不可用，请重新扫描来源');
      return true;
    }
    if (!uiStore.isOnline && state === 'remote') {
      uiStore.showToast('离线状态，该歌曲尚未缓存到本地');
      return true;
    }
    return false;
  }

  const activeAlbumId = ref<number | null>(null);
  const activeArtistId = ref<number | null>(null);
  const activePlaylistId = ref<number | null>(null);
  const globalSearchQuery = ref('');

  // 文件夹浏览状态
  const currentFolderContents = ref<FolderEntry[]>([]);
  const activeFolderSourceId = ref<number | null>(null);
  const activeFolderPath = ref<string | null>(null);
  const folderBreadcrumbs = ref<string[]>([]);
  const isFetchingFolder = ref(false);

  // 文件夹分页状态：服务端真分页，前端按需增量加载。
  // hasMoreFolderEntries 用 total - 已加载条数 判定，避免大目录一次性塞进 DOM。
  const folderPageSize = 100;
  let folderOffset = 0;
  const folderTotalCount = ref(0);
  const hasMoreFolderEntries = computed(() => currentFolderContents.value.length < folderTotalCount.value);

  // ===== 新文件浏览器状态 =====
  const folderTreeChildren = ref<import('../api/types').DirectoryNodeDTO[]>([]);
  const folderTreeSourceRoot = ref('');
  const folderTracks = ref<Track[]>([]);
  const folderTracksTotal = ref(0);
  let folderTracksOffset = 0;
  const folderTracksLimit = 100;
  const isLoadingFolderTracks = ref(false);
  const hasMoreFolderTracks = computed(() => folderTracks.value.length < folderTracksTotal.value);
  const selectedTreePath = ref<string | null>(null);

  /**
   * 拉取文件夹内容。
   * - `append=false`（默认）：重置并加载第一页，用于切换来源/进入子目录
   * - `append=true`：在当前列表后追加下一页，用于滚动到底部时增量加载
   */
  async function fetchFolderContents(sourceId: number, folderPath?: string, append = false) {
    // 增量加载时如果正在请求或已无更多，直接返回，避免重复请求
    if (append && (isFetchingFolder.value || !hasMoreFolderEntries.value)) return;

    if (!append) {
      // 切换目录：先清空旧内容，立刻给用户"已切换"的视觉反馈
      currentFolderContents.value = [];
      folderOffset = 0;
      folderTotalCount.value = 0;
    }

    isFetchingFolder.value = true;
    try {
      const res = await libraryGetFolderContents(
          sourceId,
          folderPath || undefined,
          folderPageSize,
          folderOffset
      );

      const page = res.entries.map(item => ({
        name: item.name,
        is_dir: item.is_dir,
        path: item.path,
        track: item.track ? mapTrackDTO(item.track) : undefined,
      }));

      folderTotalCount.value = res.total;
      if (append) {
        currentFolderContents.value.push(...page);
      } else {
        currentFolderContents.value = page;
      }
      folderOffset += page.length;

      activeFolderSourceId.value = sourceId;
      activeFolderPath.value = folderPath || null;

      // 更新面包屑（只在非追加模式下更新，避免追加时破坏导航状态）
      if (!append) {
        if (!folderPath) {
          folderBreadcrumbs.value = [];
        } else {
          // 如果我们进入一个子目录，且当前面包屑最后一个不是它，则加入面包屑
          if (folderBreadcrumbs.value[folderBreadcrumbs.value.length - 1] !== folderPath) {
            const isGoingBack = folderBreadcrumbs.value.includes(folderPath);
            if (isGoingBack) {
              const idx = folderBreadcrumbs.value.indexOf(folderPath);
              folderBreadcrumbs.value = folderBreadcrumbs.value.slice(0, idx + 1);
            } else {
              folderBreadcrumbs.value.push(folderPath);
            }
          }
        }
      }
    } catch (e) {
      console.error(e);
      if (!append) currentFolderContents.value = [];
    } finally {
      isFetchingFolder.value = false;
    }
  }

  /** 滚动到底部时调用：加载当前文件夹的下一页 */
  async function fetchMoreFolderEntries() {
    if (activeFolderSourceId.value === null) return;
    await fetchFolderContents(
      activeFolderSourceId.value,
      activeFolderPath.value || undefined,
      true,
    );
  }

  async function addFolderToPlaylist(sourceId: number, folderPath: string, playlistId: number) {
    try {
      await libraryAddFolderToPlaylist(sourceId, folderPath, playlistId);
      // 刷新对应歌单的轨道
      if (activeLibraryTab.value === playlists.value.find(p => p.id === playlistId)?.name) {
        fetchPlaylistTracks(playlistId);
      }
      return true;
    } catch (e) {
      console.error("Failed to add folder to playlist:", e);
      return false;
    }
  }

  // ===== 新文件浏览器功能 =====

  async function fetchFolderTreeChildren(sourceId: number, folderPath?: string) {
    try {
      const res: FolderChildrenResultDTO = await libraryGetFolderChildren(sourceId, folderPath);
      folderTreeChildren.value = res.children;
      folderTreeSourceRoot.value = res.source_root;
    } catch (e) {
      console.error('Failed to fetch folder children:', e);
    }
  }

  async function fetchFolderTracks(sourceId: number, folderPath: string, reset = false) {
    if (!reset && (isLoadingFolderTracks.value || !hasMoreFolderTracks.value)) return;
    if (reset) {
      folderTracks.value = [];
      folderTracksTotal.value = 0;
      folderTracksOffset = 0;
    }
    isLoadingFolderTracks.value = true;
    try {
      const res: FolderTracksResultDTO = await libraryGetFolderTracks(sourceId, folderPath, folderTracksLimit, folderTracksOffset);
      folderTracksTotal.value = res.total;
      const mapped = res.tracks.map(mapTrackDTO);
      if (reset) {
        folderTracks.value = mapped;
      } else {
        folderTracks.value.push(...mapped);
      }
      folderTracksOffset += mapped.length;
      selectedTreePath.value = folderPath;
    } catch (e) {
      console.error('Failed to fetch folder tracks:', e);
      if (reset) folderTracks.value = [];
    } finally {
      isLoadingFolderTracks.value = false;
    }
  }

  async function fetchMoreFolderTracks(sourceId: number, folderPath: string) {
    await fetchFolderTracks(sourceId, folderPath, false);
  }

  /** 下一次加载艺人详情时是否把子标签重置为「全部歌曲」。
   *  显式导航进入某位艺人时重置；历史前进/后退（返回详情页）时保留原分栏。 */
  let resetArtistSubTabOnLoad = false;

  /**
   * 「本次 tab 变化来自历史前进/后退」标志。
   *
   * 视图在响应 activeLibraryTab 变化时会重新拉取该 tab 的数据（loadForCurrentTab），
   * 而返回上一页时数据其实还在内存里：重拉会把分页重置到第 1 页，已加载的滚动高度消失，
   * 滚动位置记忆也就无处可恢复。所以返回时视图应复用内存数据、跳过重拉。
   *
   * 消费方：MainContent / MobileContentView 的 tab watcher（读到后自行清除）。
   * 清除方：任何显式导航（navigateToTab / Artist / Album、侧边栏选择）。
   */
  const isHistoryRestore = ref(false);

  // 页面导航历史栈
  interface HistoryState {
    tab: string;
    albumId: number | null;
    artistId: number | null;
    playlistId: number | null;
    /** 离开该页时生效的全局搜索词。非空表示这一页是全局搜索结果页，
     *  后退回来时要重现（搜索态也是页面状态的一部分）。 */
    searchQuery: string;
  }

  /**
   * 导航前对「即将离开的页面」的全局搜索词做一次快照。
   *
   * 为什么不用 globalSearchQuery 的当前值：点搜索结果进详情时，导航函数会先把
   * globalSearchQuery 清空（否则搜索视图会盖住详情页），等 history watcher 真正 push 时
   * 读到的已经是空串了，搜索页就进不了历史栈。所以必须在清空之前取一次。
   */
  let leavingSearchQuery = '';

  function takeSearchSnapshot() {
    leavingSearchQuery = globalSearchQuery.value.trim();
  }

  const historyStack = ref<HistoryState[]>([]);
  const isGoingBack = ref(false);
  const forwardStack = ref<HistoryState[]>([]);
  const isGoingForward = ref(false);

  // 监听导航状态变化以记录历史
  watch([activeLibraryTab, activeAlbumId, activeArtistId, activePlaylistId], (_newVals, oldVals) => {
    const [oldTab, oldAlbumId, oldArtistId, oldPlaylistId] = oldVals;
    // 历史前进/后退：不改写历史栈
    if (isGoingBack.value || isGoingForward.value) {
      if (isGoingBack.value) isGoingBack.value = false;
      else isGoingForward.value = false;
      // 但若这次还原没有改变 tab（详情 → 详情），视图的 tab watcher 不会触发、
      // 也就没人消费 isHistoryRestore；这里清掉，避免残留到下一次真实的 tab 切换上（那会漏加载）。
      if (oldTab === activeLibraryTab.value) isHistoryRestore.value = false;
      return;
    }
    if (oldTab) {
      // 不记录指向已删除歌单的历史：删歌单时 activePlaylistId 置空会触发一次记录，
      // 若不拦，后退会回到一个不存在的歌单详情页。
      const stalePlaylist =
        oldPlaylistId != null && !playlists.value.some(p => p.id === oldPlaylistId);
      if (!stalePlaylist) {
        historyStack.value.push({
          tab: oldTab as string,
          albumId: oldAlbumId as number | null,
          artistId: oldArtistId as number | null,
          playlistId: oldPlaylistId as number | null,
          searchQuery: leavingSearchQuery
        });
      }
    }
    // 快照是一次性的：这次导航已经用掉（或本来就不需要）
    leavingSearchQuery = '';
    forwardStack.value = [];
  });

  const canGoBack = computed(() => historyStack.value.length > 0);
  const canGoForward = computed(() => forwardStack.value.length > 0);

  function goBack() {
    if (historyStack.value.length > 0) {
      forwardStack.value.push({
        tab: activeLibraryTab.value,
        albumId: activeAlbumId.value,
        artistId: activeArtistId.value,
        playlistId: activePlaylistId.value,
        searchQuery: globalSearchQuery.value.trim()
      });
      isGoingBack.value = true;
      isHistoryRestore.value = true;
      const state = historyStack.value.pop()!;
      activeLibraryTab.value = state.tab;
      activeAlbumId.value = state.albumId;
      activeArtistId.value = state.artistId;
      activePlaylistId.value = state.playlistId;
      // 搜索结果页也是页面：还原搜索词即可让 GlobalSearch 重新挂载并复现结果
      globalSearchQuery.value = state.searchQuery ?? '';
    }
  }

  function goForward() {
    if (forwardStack.value.length > 0) {
      historyStack.value.push({
        tab: activeLibraryTab.value,
        albumId: activeAlbumId.value,
        artistId: activeArtistId.value,
        playlistId: activePlaylistId.value,
        searchQuery: globalSearchQuery.value.trim()
      });
      isGoingForward.value = true;
      isHistoryRestore.value = true;
      const state = forwardStack.value.pop()!;
      activeLibraryTab.value = state.tab;
      activeAlbumId.value = state.albumId;
      activeArtistId.value = state.artistId;
      activePlaylistId.value = state.playlistId;
      globalSearchQuery.value = state.searchQuery ?? '';
    }
  }

  // ===== 导航辅助 =====
  // 注意：同一函数内同步改多个 ref，history watch 只触发一次、只记一条历史。

  /** 切换到某个一级页面（清空详情选中态） */
  function navigateToTab(tab: string) {
    // 搜索中点了侧边栏/首页等显式导航：先快照搜索词（后退能回到搜索结果页），再关掉搜索视图，
    // 否则搜索结果会盖住刚点开的页面、看起来像"点了没反应"。没在搜索时行为不变。
    if (globalSearchQuery.value.trim()) {
      takeSearchSnapshot();
      // 若这次导航最终没有改变任何导航状态（点的就是当前所在的一级入口），
      // nav watcher 不会触发、搜索页就进不了历史栈 —— 这里手动补一条。
      const willChange =
        tab !== activeLibraryTab.value ||
        activeAlbumId.value !== null ||
        activeArtistId.value !== null ||
        activePlaylistId.value !== null;
      if (!willChange) {
        historyStack.value.push({
          tab: activeLibraryTab.value,
          albumId: null,
          artistId: null,
          playlistId: null,
          searchQuery: leavingSearchQuery
        });
        forwardStack.value = [];
        leavingSearchQuery = '';
      }
      globalSearchQuery.value = '';
    }
    isHistoryRestore.value = false;
    activeAlbumId.value = null;
    activeArtistId.value = null;
    activePlaylistId.value = null;
    activeLibraryTab.value = tab;
  }

  /** 回首页 */
  function goHome() {
    navigateToTab('首页');
  }

  /** 进艺人详情页（id 与 tab 同 tick 修改，只记一条历史） */
  function navigateToArtist(artistId: number | null | undefined) {
    if (!artistId) return;
    takeSearchSnapshot();
    isHistoryRestore.value = false;
    // 从搜索结果页点进详情：先记住搜索词（快照已取），再清空——否则搜索视图会盖住详情页
    globalSearchQuery.value = '';
    // 显式换艺人：详情页子标签回到「全部歌曲」；返回同一艺人（goBack）时保留原分栏
    if (activeArtistId.value !== artistId) resetArtistSubTabOnLoad = true;
    activeAlbumId.value = null;
    activePlaylistId.value = null;
    activeArtistId.value = artistId;
    activeLibraryTab.value = '艺术家';
  }

  /** 进专辑详情页（id 与 tab 同 tick 修改，只记一条历史） */
  function navigateToAlbum(albumId: number | null | undefined) {
    if (!albumId) return;
    takeSearchSnapshot();
    isHistoryRestore.value = false;
    globalSearchQuery.value = '';
    activeArtistId.value = null;
    activePlaylistId.value = null;
    activeAlbumId.value = albumId;
    activeLibraryTab.value = '专辑';
  }

  // 歌词数据
  const lyrics = ref<LyricLine[]>([]);

  const activeLyricIndex = computed(() => {
    if (lyrics.value.length === 0) return -1;
    const currentSec = progressMs.value / 1000;
    let index = -1;
    for (let i = 0; i < lyrics.value.length; i++) {
      if (lyrics.value[i].time <= currentSec) {
        index = i;
      } else {
        break;
      }
    }
    return index;
  });

  // 歌单数据
  const playlists = ref<Playlist[]>([]);

  // 智能歌单数据
  const activeSmartPlaylistKind = ref<string | null>(null);
  const smartPlaylistTracks = ref<Track[]>([]);
  const isLoadingSmartPlaylist = ref(false);

  // 来源数据
  const sources = ref<MusicSource[]>([]);

  // 专辑数据
  // [验证] 改为 shallowRef：只追踪 .value 的整体替换，不深度代理数组内对象。
// 目的：验证 invoke 慢是否由 Vue 对大数组的深度 reactive proxy 开销导致。
// 若验证成立，后续会把 tracks/queue 等也迁移（它们需要额外处理属性直改）。
const albums = shallowRef<Album[]>([]);

  // 艺人数据
  const artists = ref<Artist[]>([]);
  const artistsTotalCount = ref(0);

  // 收藏的专辑和艺术家
  const favoriteAlbums = ref<Album[]>([]);
  const favoriteArtists = ref<Artist[]>([]);

  // 歌曲数据列表
  const tracks = ref<Track[]>([]);
  const tracksTotalCount = ref(0);

  // 收藏数据计数（用于侧边栏徽标）
  const libraryCounts = reactive<import('../api/types').LibraryCountsDTO>({
    tracks: 0,
    favorite_tracks: 0,
    favorite_albums: 0,
    favorite_artists: 0,
    recently_played: 0,
  });

  // 分页与加载状态
  const tracksLimit = 50;
  let tracksOffset = 0;
  const hasMoreTracks = ref(true);
  const isLoadingTracks = ref(false);
  const searchQuery = ref("");

  // fetchTracks 的请求代计数器：reset 时递增，用于丢弃过期响应
  let fetchTracksGeneration = 0;

  // 从后端获取歌曲列表
  async function fetchTracks(reset = false) {
    if (reset) {
      fetchTracksGeneration++;
      isLoadingTracks.value = true;
      tracks.value = [];
      tracksOffset = 0;
      hasMoreTracks.value = true;
    }

    if (!hasMoreTracks.value || (isLoadingTracks.value && !reset)) return;
    isLoadingTracks.value = true;
    const gen = fetchTracksGeneration;

    try {
      isErrorTracks.value = false;
      const result: TrackDTO[] = await libraryGetTracks(
          tracksLimit,
          tracksOffset,
          searchQuery.value || undefined
      );

      // 如果 reset 了（generation 变化），丢弃过期响应避免数据错乱
      if (gen !== fetchTracksGeneration) return;

      if (result.length < tracksLimit) {
        hasMoreTracks.value = false;
      }

      const newTracks = mapTrackList(result);
      tracks.value.push(...newTracks);
      tracksOffset += result.length;
    } catch (e) {
      console.error("Failed to fetch tracks:", e);
      isErrorTracks.value = true;
    } finally {
      isLoadingTracks.value = false;
    }
  }

  async function fetchCounts() {
    try {
      const c = await libraryGetCounts();
      Object.assign(libraryCounts, c);
      tracksTotalCount.value = c.tracks;
    } catch (e) {
      console.error("Failed to fetch library counts:", e);
    }
  }

  // ===== 首页数据（stats / insights，迭代记录 2.2 / 2.3） =====

  const stats = ref<LibraryStatsDTO | null>(null);
  const isLoadingStats = ref(false);

  async function fetchStats() {
    isLoadingStats.value = true;
    try {
      stats.value = await libraryGetStats();
    } catch (e) {
      console.error("Failed to fetch library stats:", e);
    } finally {
      isLoadingStats.value = false;
    }
  }

  const insights = ref<HomeInsights | null>(null);
  const isLoadingInsights = ref(false);

  /** RankedTrackDTO → RankedTrack（复用统一 DTO 映射器，只多 playCount） */
  function mapRankedTrack(dto: RankedTrackDTO): RankedTrack {
    return { ...mapTrackDTO(dto), playCount: dto.play_count };
  }

  function mapRankedArtist(a: RankedArtistDTO): RankedArtist {
    return {
      id: a.id,
      name: a.name,
      playCount: a.play_count,
      trackCount: a.track_count,
      avatarArtworkId: a.avatar_artwork_id,
    };
  }

  function mapRankedAlbum(a: RankedAlbumDTO): RankedAlbum {
    return {
      id: a.id,
      title: a.title,
      artist: a.artist_name || '未知艺人',
      coverArtworkId: a.cover_artwork_id,
      playCount: a.play_count,
    };
  }

  async function fetchInsights() {
    isLoadingInsights.value = true;
    try {
      const dto = await libraryGetInsights();
      insights.value = {
        topPlayedTracks: dto.top_played_tracks.map(mapRankedTrack),
        recentPlayedTracks: dto.recent_played_tracks.map(mapRankedTrack),
        recentAddedTracks: dto.recent_added_tracks.map(mapRankedTrack),
        favoriteTracks: dto.favorite_tracks.map(mapRankedTrack),
        topPlayedArtists: dto.top_played_artists.map(mapRankedArtist),
        topPlayedAlbums: dto.top_played_albums.map(mapRankedAlbum),
        todayPlayCount: dto.today_play_count,
        lastPlayed: dto.last_played ? mapRankedTrack(dto.last_played) : null,
      };
    } catch (e) {
      console.error("Failed to fetch library insights:", e);
    } finally {
      isLoadingInsights.value = false;
    }
  }

  async function fetchPlaylists() {
    try {
      const result: PlaylistDTOBackend[] = await libraryGetPlaylists();
      playlists.value = result.map(p => ({
        id: p.id,
        name: p.name,
        description: p.description,
        count: p.track_count,
        cover_artwork_id: p.cover_artwork_id,
        cover_thumb: p.cover_thumbnail_base64
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function toggleFavorite(trackId: number) {
    try {
      // 在所有可能缓存了该 track 的位置中找到引用，统一更新 UI。
      // 注意：不再在找不到时硬编码 isFavorite=true，而是先查询后端真实状态再翻转。
      const targetTracks: Track[] = [];

      if (tracks.value) {
        const trackInTracks = tracks.value.find(t => t && t.id === trackId);
        if (trackInTracks) targetTracks.push(trackInTracks);
      }

      if (currentAlbumDetailsData.value?.tracks) {
        const trackInAlbum = currentAlbumDetailsData.value.tracks.find((t: Track) => t && t.id === trackId);
        if (trackInAlbum) targetTracks.push(trackInAlbum);
      }

      if (currentPlaylistDetailsData.value?.tracks) {
        const trackInPlaylist = currentPlaylistDetailsData.value.tracks.find((t: Track) => t && t.id === trackId);
        if (trackInPlaylist) targetTracks.push(trackInPlaylist);
      }

      if (currentArtistDetailsData.value?.tracks) {
        const trackInArtist = currentArtistDetailsData.value.tracks.find((t: Track) => t && t.id === trackId);
        if (trackInArtist) targetTracks.push(trackInArtist);
      }

      if (queue.value) {
        const trackInQueue = queue.value.find(t => t && t.id === trackId);
        if (trackInQueue) targetTracks.push(trackInQueue);
      }

      // 兜底：本地没有该 track 的缓存，无法判断当前状态。
      // 早期实现硬编码 isFavorite=true，会导致"取消收藏"被误当作"添加收藏"。
      // 这里改为：查询是否已在收藏表里，再决定翻转方向。
      let newStatus: boolean;
      if (targetTracks.length === 0) {
        const favorites: TrackDTO[] = await libraryGetFavoriteTracks();
        const exists = favorites.some(t => t.id === trackId);
        newStatus = !exists;
      } else {
        newStatus = !targetTracks[0].isFavorite;
      }

      // 乐观更新
      targetTracks.forEach(t => {
        if (t) t.isFavorite = newStatus;
      });

      try {
        await libraryToggleFavorite(trackId, newStatus);
      } catch (e) {
        console.error("Backend failed to toggle favorite:", e);
        // 回滚
        targetTracks.forEach(t => {
          if (t) t.isFavorite = !newStatus;
        });
      }
    } catch (e) {
      console.error("Exception in toggleFavorite:", e);
    }
  }

  async function recordPlay(trackId: number, durationPlayed: number, mediaFileId?: number | null) {
    if (durationPlayed < 1000) return; // 忽略极短的切歌
    try {
      await libraryRecordPlay(trackId, durationPlayed, mediaFileId);
    } catch(e) {
      console.error("Failed to record play:", e);
    }
  }

  /**
   * 批量设/取消收藏（迭代记录 2.4）。
   * 乐观更新 + 失败整体回滚，覆盖所有持有该曲的列表：
   * tracks / 专辑详情 / 艺人详情 / 歌单详情 / 智能歌单 / 播放队列 / 文件夹列表。
   */
  async function batchSetFavorite(trackIds: number[], isFavorite: boolean) {
    if (trackIds.length === 0) return;
    const idSet = new Set(trackIds);
    const affected: Track[] = [];
    const collect = (list?: Track[] | null) => {
      if (!list) return;
      for (const t of list) {
        if (t && idSet.has(t.id)) affected.push(t);
      }
    };
    collect(tracks.value);
    collect(queue.value);
    collect(smartPlaylistTracks.value);
    collect(folderTracks.value);
    collect(currentAlbumDetailsData.value?.tracks);
    collect(currentArtistDetailsData.value?.tracks);
    collect(currentPlaylistDetailsData.value?.tracks);

    const prevStates = affected.map(t => t.isFavorite);
    affected.forEach(t => { t.isFavorite = isFavorite; });

    try {
      await librarySetFavoriteBatch(trackIds, isFavorite);
      // 收藏数变化影响首页「我喜欢的音乐」榜，顺带刷新
      fetchInsights().catch(() => {});
    } catch (e) {
      console.error("Failed to batch set favorite:", e);
      affected.forEach((t, i) => { t.isFavorite = prevStates[i]; });
      throw e;
    }
  }

  async function addToPlaylist(playlistId: number, trackId: number) {
    try {
      await libraryAddToPlaylist(playlistId, trackId);
      await fetchPlaylists();
      if (activePlaylistId.value === playlistId) {
        await refreshCurrentPlaylistTracks(playlistId);
      }
    } catch(e) {
      console.error("Failed to add to playlist:", e);
    }
  }

  /** 批量添加到歌单（后端单事务去重）。返回 { added, skipped } 供 toast 提示 */
  async function batchAddToPlaylist(playlistId: number, trackIds: number[]): Promise<{ added: number; skipped: number }> {
    const [added, skipped] = await libraryAddTracksToPlaylist(playlistId, trackIds);
    await fetchPlaylists();
    if (activePlaylistId.value === playlistId) {
      await refreshCurrentPlaylistTracks(playlistId);
    }
    return { added, skipped };
  }

  async function fetchPlaylistTracks(playlistId: number) {
    try {
      const result: TrackDTO[] = await libraryGetPlaylistTracks(playlistId);
      tracks.value = mapTrackList(result);
      hasMoreTracks.value = false;
      tracksOffset = tracks.value.length;
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchRecentlyPlayed() {
    try {
      const result: TrackDTO[] = await libraryGetRecentlyPlayed(50);
      tracks.value = mapTrackList(result);
      hasMoreTracks.value = false;
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchFavoriteTracks() {
    try {
      const result: TrackDTO[] = await libraryGetFavoriteTracks();
      tracks.value = mapTrackList(result);
      hasMoreTracks.value = false;
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchFavoriteAlbums() {
    try {
      const result: AlbumDTO[] = await libraryGetFavoriteAlbums();
      favoriteAlbums.value = result.map(a => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: 0,
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        cover_thumb: a.cover_thumbnail_base64,
        track_count: a.track_count,
      }));
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchFavoriteArtists() {
    try {
      const result: ArtistDTO[] = await libraryGetFavoriteArtists();
      favoriteArtists.value = result.map(a => ({
        id: a.id,
        name: a.name,
        trackCount: a.track_count,
        avatarColor: getDeterministicColor(a.name || 'Unknown'),
        avatar_artwork_id: a.avatar_artwork_id
      }));
    } catch(e) {
      console.error(e);
    }
  }

  async function toggleFavoriteAlbum(albumId: number, isFavorite: boolean) {
    try {
      await libraryToggleFavoriteAlbum(albumId, isFavorite);
      await fetchFavoriteAlbums();
    } catch(e) {
      console.error(e);
    }
  }

  async function toggleFavoriteArtist(artistId: number, isFavorite: boolean) {
    try {
      await libraryToggleFavoriteArtist(artistId, isFavorite);
      await fetchFavoriteArtists();
    } catch(e) {
      console.error(e);
    }
  }

  const currentTime = computed({
    get: () => progressMs.value / 1000,
    set: (val: number) => { seek(val * 1000); }
  });

  async function playTrack(index: number) {
    await playQueue(tracks.value, index);
  }

  async function playAll(tracksToPlay: Track[], startIndex: number = 0) {
    if (tracksToPlay && tracksToPlay.length > 0) {
      await playQueue(tracksToPlay, startIndex);
    }
  }

  // ============ 专辑无限滚动（30张/页，IntersectionObserver 触发加载）============
  const albumsPageSize = 30;
  let albumsOffset = 0;
  const albumsTotalCount = ref(0);
  const isLoadingAlbums = ref(false);
  const isErrorAlbums = ref(false);
  const hasMoreAlbums = ref(true);

  // ============ 列表过滤：隐藏低曲目数的专辑/艺人（选择跨会话持久化）============
  const HIDE_SMALL_ALBUMS_KEY = 'lumo_hide_small_albums';
  const HIDE_MINOR_ARTISTS_KEY = 'lumo_hide_minor_artists';
  const hideSmallAlbums = ref(localStorage.getItem(HIDE_SMALL_ALBUMS_KEY) === '1');
  const hideMinorArtists = ref(localStorage.getItem(HIDE_MINOR_ARTISTS_KEY) === '1');

  function albumMinTrackCount() {
    return hideSmallAlbums.value ? ALBUM_MIN_TRACK_COUNT : undefined;
  }
  function artistMinTrackCount() {
    return hideMinorArtists.value ? ARTIST_MIN_TRACK_COUNT : undefined;
  }

  function toggleHideSmallAlbums() {
    hideSmallAlbums.value = !hideSmallAlbums.value;
    localStorage.setItem(HIDE_SMALL_ALBUMS_KEY, hideSmallAlbums.value ? '1' : '0');
    fetchAlbums(true);
  }
  function toggleHideMinorArtists() {
    hideMinorArtists.value = !hideMinorArtists.value;
    localStorage.setItem(HIDE_MINOR_ARTISTS_KEY, hideMinorArtists.value ? '1' : '0');
    fetchArtists(true);
  }

  async function fetchAlbums(reset: boolean = false) {
    if (reset) {
      albumsOffset = 0;
      hasMoreAlbums.value = true;
    }
    if (!hasMoreAlbums.value || isLoadingAlbums.value) return;
    isLoadingAlbums.value = true;
    try {
      isErrorAlbums.value = false;
      const needCount = reset || albumsTotalCount.value === 0;
      const minTrackCount = albumMinTrackCount();

      const fetchList = libraryGetAlbums(albumsPageSize, albumsOffset, searchQuery.value || undefined, minTrackCount);
      const fetchCount = needCount ? libraryGetAlbumCount(searchQuery.value || undefined, minTrackCount) : Promise.resolve(albumsTotalCount.value);

      const [result, count] = await Promise.all([fetchList, fetchCount]);
      albumsTotalCount.value = count;

      const newAlbums: Album[] = result.map((a) => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: a.release_year || 0,
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        cover_thumb: a.cover_thumbnail_base64,
        artist_name: a.artist_name,
        track_count: a.track_count
      }));

      if (reset) {
        albums.value = newAlbums;
      } else {
        albums.value = [...albums.value, ...newAlbums];
      }
      albumsOffset += result.length;
      hasMoreAlbums.value = result.length >= albumsPageSize;
    } catch (e) {
      console.error(e);
      isErrorAlbums.value = true;
    } finally {
      isLoadingAlbums.value = false;
    }
  }

  const artistsLimit = 50;
  let artistsOffset = 0;
  const hasMoreArtists = ref(true);
  const isLoadingArtists = ref(false);

  async function fetchArtists(reset: boolean = false) {
    if (reset) {
      artists.value = [];
      artistsOffset = 0;
      hasMoreArtists.value = true;
    }
    if (!hasMoreArtists.value || isLoadingArtists.value) return;
    isLoadingArtists.value = true;
    try {
      isErrorArtists.value = false;
      const { artists: result, total } = await libraryGetArtists(
          artistsLimit,
          artistsOffset,
          searchQuery.value || undefined,
          artistMinTrackCount()
      );
      artistsTotalCount.value = total;
      if (result.length < artistsLimit) {
        hasMoreArtists.value = false;
      }
      const newArtists: Artist[] = result.map((a) => ({
        id: a.id,
        name: a.name,
        trackCount: a.track_count,
        avatarColor: getDeterministicColor(a.name || 'Unknown'),
        track_count: a.track_count,
        avatar_artwork_id: a.avatar_artwork_id
      }));
      artists.value.push(...newArtists);
      artistsOffset += result.length;
    } catch (e) {
      console.error(e);
      isErrorArtists.value = true;
    } finally {
      isLoadingArtists.value = false;
    }
  }

  const currentTrack = computed(() => {
    return queue.value[currentIndex.value] || null;
  });

  // 歌词行接口定义
  interface LyricLine {
    text: string;
    time: number;
  }

  function parseLrc(lrcText: string): LyricLine[] {
    const lines = lrcText.split('\n');
    const result: LyricLine[] = [];
    const timeReg = /\[(\d+):(\d+)(?:\.(\d+))?\]/g;

    for (const line of lines) {
      const cleanLine = line.trim();
      if (!cleanLine) continue;

      let match;
      const times: number[] = [];
      let lastIndex = 0;

      timeReg.lastIndex = 0;
      while ((match = timeReg.exec(cleanLine)) !== null) {
        const min = parseInt(match[1], 10);
        const sec = parseInt(match[2], 10);
        const ms = match[3] ? parseInt(match[3], 10) : 0;
        const msLen = match[3] ? match[3].length : 0;
        const msFraction = msLen === 3 ? ms / 1000 : ms / 100;
        const timeInSeconds = min * 60 + sec + msFraction;
        times.push(timeInSeconds);
        lastIndex = timeReg.lastIndex;
      }

      const text = cleanLine.substring(lastIndex).trim();
      for (const time of times) {
        result.push({ text, time });
      }
    }

    result.sort((a, b) => a.time - b.time);
    return result;
  }

  // 监听当前播放曲目，自动加载对应歌词与文件元数据（解耦并行加载，防止慢速网络歌词阻塞元数据展示）
  let currentTrackRequestId = 0;
  let trackInfoDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  watch(currentTrack, (newTrack) => {
    const reqId = ++currentTrackRequestId;
    if (trackInfoDebounceTimer) {
      clearTimeout(trackInfoDebounceTimer);
      trackInfoDebounceTimer = null;
    }
    if (newTrack) {
      const trackId = newTrack.id;

      // 快速连续切歌时防抖 100ms，避免为飞速掠过的过时曲目发出无谓的元数据和歌词 IPC 请求
      trackInfoDebounceTimer = setTimeout(() => {
        if (reqId !== currentTrackRequestId) return;

        // 1. 独立异步加载文件元数据（本地查询，零等待）
        libraryGetTrackFileInfo(trackId)
          .then(fileInfo => {
            if (reqId === currentTrackRequestId) {
              currentTrackFileInfo.value = fileInfo;
            }
          })
          .catch(e => {
            console.error("Failed to load track file info:", e);
            if (reqId === currentTrackRequestId) {
              currentTrackFileInfo.value = null;
            }
          });

        // 2. 独立异步加载歌词（网络请求与本地查询独立，不拖垮主 UI）
        const lyricsPromise = uiStore.fetchLyricsOnline
          ? libraryGetLyrics(trackId, true)
          : Promise.resolve(null);

        lyricsPromise
          .then(lrcText => {
            if (reqId !== currentTrackRequestId) return;
            if (lrcText) {
              lyrics.value = parseLrc(lrcText);
            } else {
              lyrics.value = [
                { text: newTrack.title, time: 0 },
                { text: newTrack.artist, time: 3 },
                { text: "— 暂无歌词 —", time: 6 }
              ];
            }
          })
          .catch(e => {
            console.error("Failed to load lyrics:", e);
            if (reqId === currentTrackRequestId) {
              lyrics.value = [
                { text: newTrack.title, time: 0 },
                { text: "— 暂无歌词 —", time: 3 }
              ];
            }
          });
      }, 100);
    } else {
      lyrics.value = [];
      currentTrackFileInfo.value = null;
    }
  }, { immediate: true });

  const currentAlbumDetailsData = ref<AlbumDetails | null>(null);
  const currentArtistDetailsData = ref<ArtistDetails | null>(null);
  const currentPlaylistDetailsData = ref<PlaylistDetails | null>(null);
  const isCreatePlaylistModalOpen = ref(false);

  const createPlaylist = async (name: string, description: string): Promise<number> => {
    try {
      const id = await libraryCreatePlaylist(name, description);
      await fetchPlaylists();
      return id;
    } catch(e) {
      console.error("Failed to create playlist:", e);
      throw e;
    }
  };

  async function refreshCurrentPlaylistTracks(playlistId: number) {
    const playlist = playlists.value.find(p => p.id === playlistId)
      || { id: playlistId, name: '未知歌单', count: 0, description: '', cover_artwork_id: null, cover_thumb: null };
    if (!currentPlaylistDetailsData.value) {
      currentPlaylistDetailsData.value = { ...playlist, tracks: [], isLoadingTracks: true };
    } else {
      currentPlaylistDetailsData.value.isLoadingTracks = true;
    }

    try {
      const result: TrackDTO[] = await libraryGetPlaylistTracks(playlistId);
      const tracksData = mapTrackList(result);

      playlist.count = tracksData.length;
      currentPlaylistDetailsData.value = {
        ...playlist,
        tracks: tracksData,
        isLoadingTracks: false
      };
    } catch(e) {
      console.error(e);
      currentPlaylistDetailsData.value.isLoadingTracks = false;
    }
  }

  watch(activePlaylistId, async (newId) => {
    if (newId) {
      await refreshCurrentPlaylistTracks(newId);
    } else {
      currentPlaylistDetailsData.value = null;
    }
  });

  /** 打开某个歌单详情（显式导航：清掉其它详情选中态，并取消「历史还原」标志） */
  async function openPlaylist(playlistId: number) {
    isHistoryRestore.value = false;
    activeAlbumId.value = null;
    activeArtistId.value = null;
    activeLibraryTab.value = '播放列表';
    if (activePlaylistId.value === playlistId) {
      await refreshCurrentPlaylistTracks(playlistId);
    } else {
      activePlaylistId.value = playlistId;
    }
  }

  watch(activeAlbumId, async (newId) => {
    if (newId) {
      let album = albums.value.find(a => a.id === newId);
      if (!album && currentArtistDetailsData.value?.albums) {
        album = currentArtistDetailsData.value.albums.find((a: any) => a.id === newId);
      }
      
      // 如果内存里找不到该专辑（例如从全局搜索或最近播放跳转），去后端查询
      if (!album) {
        try {
          const albumDto = await libraryGetAlbumById(newId);
          if (albumDto) {
            album = {
              id: albumDto.id,
              title: albumDto.title,
              artist: albumDto.artist_name || '未知艺人',
              year: albumDto.release_year || 0,
              coverColor: getDeterministicColor(albumDto.title),
              cover_artwork_id: albumDto.cover_artwork_id ?? null,
              cover_thumb: albumDto.cover_thumbnail_base64 || null,
              artist_name: albumDto.artist_name || '未知艺人',
              track_count: albumDto.track_count || 0,
            } as any;
          }
        } catch (e) {
          console.error('Failed to load album info:', e);
        }
      }

      // 如果连后端也查不到（兜底逻辑），尝试从 tracks 推断
      if (!album) {
        const loadedTracks = currentAlbumDetailsData.value?.tracks ?? [];
        if (loadedTracks.length > 0) {
          album = {
            id: newId,
            title: loadedTracks[0].album,
            artist: loadedTracks[0].artist,
            cover_artwork_id: loadedTracks[0].cover_artwork_id,
            coverColor: loadedTracks[0].coverColor,
            track_count: loadedTracks.length,
          } as any;
        }
      }

      if (album) {
        try {
          // 不要 await 阻塞 tracks 的加载，改为后台触发（第七轮：命令立即返回不占 IPC
          // 并发，完成后由 album-cover-fetched 事件回调统一更新 UI）
          if (album.cover_artwork_id == null && uiStore.fetchCoversOnline) {
            libraryFetchMissingAlbumCover(newId, true).catch(console.error);
          }
          const result: TrackDTO[] = await libraryGetAlbumTracks(newId);
          const tracksData = mapTrackList(result);
          currentAlbumDetailsData.value = { ...album, tracks: tracksData };
        } catch (e) {
          console.error(e);
        }
      }
    } else {
       currentAlbumDetailsData.value = null;
    }
  });

  const fetchArtistTracks = async (artistId: number, isLoadMore = false) => {
    if (!currentArtistDetailsData.value) return;
    if (isLoadMore && (!currentArtistDetailsData.value.hasMoreTracks || currentArtistDetailsData.value.isLoadingTracks)) return;

    currentArtistDetailsData.value.isLoadingTracks = true;
    try {
      const limit = 30;
      const offset = currentArtistDetailsData.value.tracksOffset;
      const tracksResult: TrackDTO[] = await libraryGetArtistTracks(artistId, limit, offset);

      const tracksData = mapTrackList(tracksResult);

      if (isLoadMore) {
        currentArtistDetailsData.value.tracks.push(...tracksData);
      } else {
        currentArtistDetailsData.value.tracks = tracksData;
      }
      currentArtistDetailsData.value.tracksOffset += tracksData.length;
      currentArtistDetailsData.value.hasMoreTracks = tracksData.length === limit;
    } catch(e) {
      console.error(e);
    } finally {
      currentArtistDetailsData.value.isLoadingTracks = false;
    }
  };

  // 艺人详情页专辑分页（与专辑页保持一致：15张/页 = 5列×3行）
  const ARTIST_ALBUMS_PAGE_SIZE = 15;

  const fetchArtistAlbums = async (artistId: number, isLoadMore = false) => {
    if (!currentArtistDetailsData.value) return;
    if (currentArtistDetailsData.value.isLoadingAlbums) return;

    currentArtistDetailsData.value.isLoadingAlbums = true;
    try {
      const page = currentArtistDetailsData.value.albumsCurrentPage || 1;
      const offset = (page - 1) * ARTIST_ALBUMS_PAGE_SIZE;
      const needCount = isLoadMore === false || currentArtistDetailsData.value.albumsTotalCount === 0;

      const fetchList = libraryGetArtistAlbums(artistId, ARTIST_ALBUMS_PAGE_SIZE, offset);
      const fetchCount = needCount ? libraryGetArtistAlbumCount(artistId) : Promise.resolve(currentArtistDetailsData.value.albumsTotalCount || 0);

      const [albumsResult, count] = await Promise.all([fetchList, fetchCount]);
      currentArtistDetailsData.value.albumsTotalCount = count;

      const artistAlbums: Album[] = albumsResult.map(a => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: a.release_year || 0,
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        cover_thumb: a.cover_thumbnail_base64,
        artist_name: a.artist_name,
        track_count: a.track_count
      }));

      currentArtistDetailsData.value.albums = artistAlbums;
      currentArtistDetailsData.value.albumsTotalPages = count > 0 ? Math.ceil(count / ARTIST_ALBUMS_PAGE_SIZE) : 1;
      currentArtistDetailsData.value.hasMoreAlbums = albumsResult.length === ARTIST_ALBUMS_PAGE_SIZE;
    } catch(e) {
      console.error(e);
    } finally {
      currentArtistDetailsData.value.isLoadingAlbums = false;
    }
  };

  async function goToArtistAlbumsPage(page: number) {
    if (!currentArtistDetailsData.value || !activeArtistId.value) return;
    if (page < 1 || page > (currentArtistDetailsData.value.albumsTotalPages || 1)) return;
    currentArtistDetailsData.value.albumsCurrentPage = page;
    await fetchArtistAlbums(activeArtistId.value, false);
  }

  function nextArtistAlbumsPage() {
    if (!currentArtistDetailsData.value) return;
    const cur = currentArtistDetailsData.value.albumsCurrentPage || 1;
    if (cur >= (currentArtistDetailsData.value.albumsTotalPages || 1)) return;
    goToArtistAlbumsPage(cur + 1);
  }

  function prevArtistAlbumsPage() {
    if (!currentArtistDetailsData.value) return;
    const cur = currentArtistDetailsData.value.albumsCurrentPage || 1;
    if (cur <= 1) return;
    goToArtistAlbumsPage(cur - 1);
  }

  /**
   * 艺人详情缓存（LRU，上限 8 位艺人）。
   *
   * 动机：activeArtistId 在「详情 → 某首歌的专辑名 → 返回」这类路径中会被清空，
   * 原来的实现每次都会重建详情对象（分页回到第 1 页、子标签回到全部歌曲）。
   * 这里把离开时的详情整体留下，返回同一艺人时直接复用，实现「原页面 + 原分页」。
   */
  const artistDetailsCache = new Map<number, ArtistDetails>();
  const ARTIST_DETAILS_CACHE_MAX = 8;

  function cacheArtistDetails(id: number, data: ArtistDetails) {
    if (!data.tracks || data.tracks.length === 0) return; // 空壳不缓存
    artistDetailsCache.delete(id);
    artistDetailsCache.set(id, data);
    while (artistDetailsCache.size > ARTIST_DETAILS_CACHE_MAX) {
      const oldest = artistDetailsCache.keys().next().value;
      if (oldest === undefined) break;
      artistDetailsCache.delete(oldest);
    }
  }

  watch(activeArtistId, async (newId, oldId) => {
    // 1. 离开上一位艺人：留下详情（保留分页与子标签），供返回时复用
    if (oldId) {
      const prev = currentArtistDetailsData.value;
      if (prev && prev.id === oldId) cacheArtistDetails(oldId, prev);
    }

    if (newId) {
      // 2. 命中缓存：直接复用，不重置分页、不重拉
      const cached = artistDetailsCache.get(newId);
      if (cached) {
        // LRU 触达
        artistDetailsCache.delete(newId);
        artistDetailsCache.set(newId, cached);
        if (resetArtistSubTabOnLoad) cached.subTab = 'tracks';
        resetArtistSubTabOnLoad = false;
        currentArtistDetailsData.value = cached;
        return;
      }
      resetArtistSubTabOnLoad = false;

      let artist = artists.value.find(a => a.id === newId);

      // 如果内存列表里找不到，从后端查询
      if (!artist) {
        try {
          const artistDto = await libraryGetArtistById(newId);
          if (artistDto) {
            artist = {
              id: artistDto.id,
              name: artistDto.name,
              avatarColor: getDeterministicColor(artistDto.name),
              trackCount: artistDto.track_count || 0,
              avatar_artwork_id: artistDto.avatar_artwork_id ?? null,
            };
          }
        } catch (e) {
          console.error('Failed to load artist info:', e);
        }
      }

      // 兜底：如果还是找不到，给个默认值
      if (!artist) {
        artist = { 
          id: newId, 
          name: '未知艺人', 
          avatarColor: getDeterministicColor('未知艺人'), 
          trackCount: 0,
          avatar_artwork_id: null,
        };
      }

      currentArtistDetailsData.value = {
        ...artist,
        albums: [],
        tracks: [],
        stats: { track_count: 0, album_count: 0 },
        subTab: 'tracks',
        tracksOffset: 0,
        albumsOffset: 0,
        albumsCurrentPage: 1,
        albumsTotalCount: 0,
        albumsTotalPages: 1,
        hasMoreTracks: true,
        hasMoreAlbums: true,
        isLoadingTracks: false,
        isLoadingAlbums: false
      } as ArtistDetails;
      resetArtistSubTabOnLoad = false;

      try {
        if (artist.avatar_artwork_id == null && uiStore.fetchCoversOnline) {
          // 不阻塞，后台触发（第七轮：完成后由 artist-cover-fetched 事件回调统一更新 UI）
          libraryFetchMissingArtistCover(newId, true).catch(console.error);
        }
        
        // 并行加载统计信息，不阻塞轨道和专辑
        libraryGetArtistStats(newId).then(stats => {
          if (currentArtistDetailsData.value?.id === newId) {
             currentArtistDetailsData.value.stats = stats;
             currentArtistDetailsData.value.trackCount = stats.track_count;
          }
        }).catch(console.error);
      } catch(e) {
        console.error(e);
      }

      await fetchArtistTracks(newId, false);
      await fetchArtistAlbums(newId, false);
    } else {
      currentArtistDetailsData.value = null;
    }
  });

  const currentAlbumDetails = computed(() => currentAlbumDetailsData.value);
  const currentArtistDetails = computed(() => currentArtistDetailsData.value);
  const currentPlaylistDetails = computed(() => currentPlaylistDetailsData.value);

  /** 切换艺人详情页子标签（全部歌曲 / 全部专辑），状态挂在上，返回时才能回到原分栏 */
  function setArtistDetailSubTab(tab: 'tracks' | 'albums') {
    if (currentArtistDetailsData.value) currentArtistDetailsData.value.subTab = tab;
  }

  const localSources = computed(() => {
    return sources.value.filter(s => s.kind === 'local');
  });

  const webdavSources = computed(() => {
    return sources.value.filter(s => s.kind === 'webdav');
  });

  // ================= 持久化与恢复逻辑 =================

  /**
   * 持久化播放队列：只在队列内容（id 序列）真正变化时调用，避免 deep watch 在
   * toggleFavorite / 任何深层字段修改时都触发整表 DELETE+INSERT。
   * 通过比较 id 序列的快照来判断"内容是否变了"。
   */
  let lastSavedQueueSignature = '';
  let persistQueueTimer: ReturnType<typeof setTimeout> | null = null;
  function persistPlayQueueIfNeeded() {
    const sig = queue.value.map(t => t.id).join(',');
    if (sig === lastSavedQueueSignature) return;
    lastSavedQueueSignature = sig;
    if (persistQueueTimer) {
      clearTimeout(persistQueueTimer);
    }
    // 防抖 3000ms：让播放启动、解码与 UI 切换平稳完成后，再异步落库，彻底避免抢占写锁
    persistQueueTimer = setTimeout(() => {
      persistQueueTimer = null;
      librarySavePlayQueue(queue.value.map(t => t.id))
        .catch(e => console.error("Failed to auto-save play queue:", e));
    }, 3000);
  }

  // ===== 进度持久化（事件驱动，避免高频写磁盘） =====
  let progressSaveTimer: ReturnType<typeof setInterval> | null = null;

  function saveProgressToStorage() {
    localStorage.setItem('lumo_progress_ms', String(progressMs.value));
  }

  function startProgressAutoSave() {
    if (progressSaveTimer) return;
    progressSaveTimer = setInterval(saveProgressToStorage, 30000);
  }

  function stopProgressAutoSave() {
    if (progressSaveTimer) {
      clearInterval(progressSaveTimer);
      progressSaveTimer = null;
    }
    saveProgressToStorage();
  }

  // 监视状态标量并写入 localStorage
  watch(currentIndex, (newIdx) => {
    localStorage.setItem('lumo_current_index', String(newIdx));
  });
  watch(playMode, (newMode) => {
    localStorage.setItem('lumo_play_mode', newMode);
    playbackSetMode(toBackendPlayMode(newMode)).catch(e => console.warn('Failed to sync playMode to backend:', e));
  });
  watch(volume, (newVol) => {
    localStorage.setItem('lumo_volume', String(newVol));
  });

  // 恢复状态与队列
  // 启动数据包（遗留事项 2）：一次 IPC 拿回 counts/playlists/albums/artists/play_queue，
  // 返回的 bundle 交给 restoreSession 做纯本地恢复；失败时返回 null，restoreSession
  // 自动回退到旧的逐个 IPC 路径。
  async function fetchStartupBundle(): Promise<StartupBundleDTO | null> {
    try {
      const b = await libraryGetStartupBundle(albumMinTrackCount(), artistMinTrackCount());

      Object.assign(libraryCounts, b.counts);
      tracksTotalCount.value = b.counts.tracks;

      playlists.value = b.playlists.map(p => ({
        id: p.id,
        name: p.name,
        description: p.description,
        count: p.track_count,
        cover_artwork_id: p.cover_artwork_id,
        cover_thumb: p.cover_thumbnail_base64
      }));

      const newAlbums: Album[] = b.albums.map((a) => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: a.release_year || 0,
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        cover_thumb: a.cover_thumbnail_base64,
        artist_name: a.artist_name,
        track_count: a.track_count
      }));
      albums.value = newAlbums;
      albumsOffset = b.albums.length;
      albumsTotalCount.value = b.album_total;
      hasMoreAlbums.value = b.albums.length >= albumsPageSize && b.albums.length < b.album_total;

      const newArtists: Artist[] = b.artists.map((a) => ({
        id: a.id,
        name: a.name,
        trackCount: a.track_count,
        avatarColor: getDeterministicColor(a.name || 'Unknown'),
        track_count: a.track_count,
        avatar_artwork_id: a.avatar_artwork_id
      }));
      artists.value = newArtists;
      artistsOffset = b.artists.length;
      artistsTotalCount.value = b.artist_total;
      hasMoreArtists.value = b.artists.length < b.artist_total;

      if (b.play_queue.length > 0) {
        queue.value = mapTrackList(b.play_queue);
        lastSavedQueueSignature = queue.value.map(t => t.id).join(',');
      }

      return b;
    } catch (e) {
      console.error("Failed to fetch startup bundle:", e);
      return null;
    }
  }

  async function restoreSession(bundle?: StartupBundleDTO | null) {
    try {
      // 1. 恢复播放队列（启动包已带入 queue；bundle 缺失时兜底单独拉）
      if (!bundle && queue.value.length === 0) {
        const savedQueue: TrackDTO[] = await libraryGetPlayQueue();
        if (savedQueue && savedQueue.length > 0) {
          queue.value = mapTrackList(savedQueue);
          lastSavedQueueSignature = queue.value.map(t => t.id).join(',');
        }
      }

      // 2. 恢复播放模式
      const savedMode = localStorage.getItem('lumo_play_mode');
      if (savedMode && ['normal', 'repeat', 'repeat-one', 'shuffle'].includes(savedMode)) {
        playMode.value = savedMode as 'normal' | 'repeat' | 'repeat-one' | 'shuffle';
      }

      // 3. 恢复音量
      const savedVolume = localStorage.getItem('lumo_volume');
      if (savedVolume !== null) {
        const vol = parseInt(savedVolume, 10);
        if (!isNaN(vol) && vol >= 0 && vol <= 100) {
          volume.value = vol;
          await playbackSetVolume(vol / 100);
        }
      }

      // 3.5 恢复播放速率（后端每次启动都是 1.0，需要显式下发）
      const savedRate = localStorage.getItem('lumo_playback_rate');
      if (savedRate !== null) {
        const rate = parseFloat(savedRate);
        if (!isNaN(rate) && rate >= 0.5 && rate <= 1.5) {
          await setPlaybackRate(rate);
        }
      }

      // 4. 恢复当前曲目索引（处于暂停/载入锁状态）
      const savedIdx = localStorage.getItem('lumo_current_index');
      if (savedIdx !== null) {
        const idx = parseInt(savedIdx, 10);
        if (!isNaN(idx) && idx >= 0 && idx < queue.value.length) {
          currentIndex.value = idx;
          hasLoadedCurrentFile.value = false; // 设定需要初次重新加载文件锁

          const savedProgress = localStorage.getItem('lumo_progress_ms');
          if (savedProgress !== null) {
            const prog = parseInt(savedProgress, 10);
            if (!isNaN(prog) && prog >= 0) {
              progressMs.value = prog;
            }
          }
        }
      }
    } catch (e) {
      console.error("Failed to restore session:", e);
    }
    // bundle 已含 counts；兜底路径才需要单独拉
    if (!bundle) fetchCounts();
  }

  // ================= 歌单操作 Actions =================

  // 删除歌单
  async function deletePlaylist(playlistId: number) {
    try {
      await libraryDeletePlaylist(playlistId);
      await fetchPlaylists();
      // 历史里指向该歌单的条目一并清掉（后退/前进都不该回到已删除的歌单）
      historyStack.value = historyStack.value.filter(s => s.playlistId !== playlistId);
      forwardStack.value = forwardStack.value.filter(s => s.playlistId !== playlistId);
      if (activePlaylistId.value === playlistId) {
        activePlaylistId.value = null;
        activeLibraryTab.value = '全部歌曲';
      }
    } catch (e) {
      console.error("Failed to delete playlist:", e);
      throw e;
    }
  }

  // 从歌单移除单曲
  async function removeTrackFromPlaylist(playlistId: number, trackId: number) {
    try {
      await libraryRemovePlaylistItem(playlistId, trackId);
      // 先刷新歌单列表（封面取首曲，删掉第一首后封面会变），再重建详情
      await fetchPlaylists();
      await refreshCurrentPlaylistTracks(playlistId);
    } catch (e) {
      console.error("Failed to remove track from playlist:", e);
      throw e;
    }
  }

  // ================= 智能歌单 Actions =================

  /**
   * 加载指定类型的智能歌单。
   * @param kind 预设类型，目前支持 "most_played"（播放最多）
   */
  async function loadSmartPlaylist(kind: string) {
    activeSmartPlaylistKind.value = kind;
    activeLibraryTab.value = '智能歌单';
    activeAlbumId.value = null;
    activeArtistId.value = null;
    activePlaylistId.value = null;
    isLoadingSmartPlaylist.value = true;
    try {
      const dtos = await libraryGetSmartPlaylist(kind, 100);
      smartPlaylistTracks.value = mapTrackList(dtos);
    } catch (e) {
      console.error(`[SmartPlaylist] Failed to load kind=${kind}:`, e);
      smartPlaylistTracks.value = [];
    } finally {
      isLoadingSmartPlaylist.value = false;
    }
  }

  let isTogglingPlay = false;
  async function togglePlay() {
    if (isTogglingPlay) return;
    if (queue.value.length === 0) return;
    if (currentIndex.value === -1) {
      currentIndex.value = 0;
    }

    const track = queue.value[currentIndex.value];
    if (!track) return;

    isTogglingPlay = true;
    try {
      if (isPlaying.value) {
        // 乐观更新：立刻在前端呈现暂停状态，停止进度轮询，给用户瞬时响应
        isPlaying.value = false;
        stopProgressAutoSave();
        if ('mediaSession' in navigator) {
          navigator.mediaSession.playbackState = 'paused';
        }
        try {
          await playbackPause();
        } catch (err) {
          // 发生错误时回滚状态
          isPlaying.value = true;
          startProgressAutoSave();
          if ('mediaSession' in navigator) {
            navigator.mediaSession.playbackState = 'playing';
          }
          throw err;
        }
      } else {
        if (guardPlayback(track)) {
          return;
        }
        if (!hasLoadedCurrentFile.value) {
          if (track.primary_file_id) {
            await playbackPlay(track.primary_file_id, !uiStore.isOnline);
            hasLoadedCurrentFile.value = true;

            if (progressMs.value > 0) {
              // 恢复上次听到的地方。seek 失败只记日志，绝不把整个播放动作带崩
              try {
                await playbackSeek(progressMs.value);
              } catch (seekErr) {
                console.warn('Resume seek failed, playing from start:', seekErr);
                progressMs.value = 0;
              }
            }
            // 同步系统媒体通知
            updateMediaSessionMetadata(track);
          }
        } else {
          // 乐观恢复
          isPlaying.value = true;
          startProgressAutoSave();
          if ('mediaSession' in navigator) {
            navigator.mediaSession.playbackState = 'playing';
          }
          try {
            await playbackResume();
          } catch (err) {
            isPlaying.value = false;
            stopProgressAutoSave();
            if ('mediaSession' in navigator) {
              navigator.mediaSession.playbackState = 'paused';
            }
            throw err;
          }
          return;
        }
        isPlaying.value = true;
        startProgressAutoSave();
        if ('mediaSession' in navigator) {
          navigator.mediaSession.playbackState = 'playing';
        }
      }
    } catch (e) {
      console.error("Toggle play failed:", e);
    } finally {
      isTogglingPlay = false;
    }
  }

  // ================= 播放推进事件监听 (MA2: ADR-3) =================
  listen<{ index: number; track: any }>('playback-track-changed', (event) => {
    const { index } = event.payload;
    if (index >= 0 && index < queue.value.length) {
      currentIndex.value = index;
      const track = queue.value[index];
      hasLoadedCurrentFile.value = true;
      isPlaying.value = true;
      progressMs.value = 0;
      durationMs.value = track.durationSec ? track.durationSec * 1000 : 0;
      updateMediaSessionMetadata(track);
    }
  });

  listen<{ position: number }>('playback-progress', (event) => {
    progressMs.value = event.payload.position;
    if (isBuffering.value) {
      isBuffering.value = false;
    }
  });

  listen<{ is_playing: boolean }>('playback-status-changed', (event) => {
    if (event.payload && typeof event.payload.is_playing === 'boolean') {
      isPlaying.value = event.payload.is_playing;
      if ('mediaSession' in navigator) {
        navigator.mediaSession.playbackState = isPlaying.value ? 'playing' : 'paused';
      }
    }
  });

  listen<{ index: number; message: string }>('playback-error', (event) => {
    console.error('[playback-error] 播放失败:', event.payload);
    isPlaying.value = false;
    isBuffering.value = false;
    uiStore.showToast(event.payload?.message || '播放失败');
    if ('mediaSession' in navigator) {
      navigator.mediaSession.playbackState = 'paused';
    }
  });

  // 前后台对账：恢复前台时与 Rust 权威队列同步
  async function syncQueueStateFromBackend() {
    try {
      const state = await playbackQueueState();
      if (state && state.items.length > 0) {
        if (queue.value.length === 0) {
          queue.value = state.items.map(item => ({
            id: item.trackId,
            title: item.title,
            artistId: null,
            artist: item.artist,
            albumId: null,
            album: item.album,
            duration: formatTime((item.durationMs ?? 0) / 1000),
            durationSec: Math.floor((item.durationMs ?? 0) / 1000),
            format: 'UNKNOWN',
            coverColor: getDeterministicColor(item.album || item.title || 'Unknown'),
            cover_artwork_id: item.artworkId,
            isFavorite: false,
            primary_file_id: item.mediaFileId,
            fileSize: null,
            sourceKind: 'local',
          }));
        }
        currentIndex.value = state.index;
        progressMs.value = state.positionMs;
        playMode.value = fromBackendPlayMode(state.mode);
        const currentTrack = queue.value[state.index];
        if (currentTrack) {
          durationMs.value = currentTrack.durationSec ? currentTrack.durationSec * 1000 : 0;
        }
      }
    } catch (e) {
      console.warn('[QueueSync] Failed to sync queue state:', e);
    }
  }

  if (typeof window !== 'undefined') {
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') {
        syncQueueStateFromBackend();
      }
    });
  }

  function isSameTrackList(a: Track[], b: Track[]): boolean {
    if (a === b) return true;
    if (a.length !== b.length) return false;
    if (a.length === 0) return true;
    if (a[0].id !== b[0].id || a[a.length - 1].id !== b[b.length - 1].id) return false;
    for (let i = 1; i < a.length - 1; i++) {
      if (a[i].id !== b[i].id) return false;
    }
    return true;
  }

  async function playQueue(newQueue: Track[], index: number, _skipHistoryPush = false) {
    const clickedTrack = newQueue[index];
    // 离线/不可播守卫：本地文件丢失或离线未缓存的曲目直接提示，不发请求
    if (guardPlayback(clickedTrack)) {
      return;
    }
    const isSame = isSameTrackList(queue.value, newQueue);
    if (!isSame) {
      queue.value = [...newQueue];
    }
    currentIndex.value = index;
    const track = queue.value[index];
    if (track) {
      durationMs.value = track.durationSec ? track.durationSec * 1000 : 0;
      progressMs.value = 0;
      isPlaying.value = true;
      isBuffering.value = true;
      hasLoadedCurrentFile.value = true;

      try {
        if (isSame) {
          // 当前播放队列与传入列表完全一致：无需重新全量序列化和设置队列，直接按索引播放
          await playbackPlayIndex(index);
        } else {
          const items: QueueItemDTO[] = newQueue.map(t => ({
            trackId: t.id,
            mediaFileId: t.primary_file_id || 0,
            title: t.title,
            artist: t.artist,
            album: t.album,
            artworkId: t.cover_artwork_id || null,
            durationMs: t.durationSec ? t.durationSec * 1000 : null,
          }));

          await playbackSetQueue(items, index, toBackendPlayMode(playMode.value));
          persistPlayQueueIfNeeded();
        }
        updateMediaSessionMetadata(track);
        if ('mediaSession' in navigator) {
          navigator.mediaSession.playbackState = 'playing';
        }
      } catch (e) {
        console.error("Play queue failed:", e);
        isPlaying.value = false;
        isBuffering.value = false;
        if ('mediaSession' in navigator) {
          navigator.mediaSession.playbackState = 'paused';
        }
      }
    }
  }

  let isAdvancing = false;
  async function nextTrack(_isAuto = false) {
    if (queue.value.length === 0 || isAdvancing) return;
    isAdvancing = true;
    try {
      await playbackAdvance(1);
    } catch (e) {
      console.error("Advance next failed:", e);
    } finally {
      isAdvancing = false;
    }
  }

  async function prevTrack() {
    if (queue.value.length === 0 || isAdvancing) return;
    isAdvancing = true;
    try {
      await playbackAdvance(-1);
    } catch (e) {
      console.error("Advance prev failed:", e);
    } finally {
      isAdvancing = false;
    }
  }

  async function setVolume(v: number) {
    volume.value = v;
    try {
      await playbackSetVolume(v / 100);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * 设置播放速率（0.5–1.5）。后端立即生效（rodio 音频线程每 5ms 取一次该值），
   * 且进度条位置会按速率换算，所以变速后时间显示不会漂。
   * 变速是纯重采样，会有音高变化（无时间拉伸依赖下的取舍）。
   */
  async function setPlaybackRate(rate: number) {
    const clamped = Math.min(1.5, Math.max(0.5, Number(rate) || 1));
    playbackRate.value = clamped;
    localStorage.setItem('lumo_playback_rate', String(clamped));
    try {
      await playbackSetSpeed(clamped);
    } catch (e) {
      console.error('Failed to set playback speed:', e);
    }
  }

  /**
   * seek 的在飞合并：上一发没回来时，新目标只覆盖「待发」值，上一发回来后再补发最新的。
   * 拖进度条松手会连发多次，而后端 seek 可能因切歌解码而排队 —— 不合并的话
   * 请求会越积越多（实测 7 个 seek 排队 22 秒），而且排队的里面只有最后一个有意义。
   */
  let seekInFlight = false;
  let pendingSeekMs: number | null = null;

  async function seek(positionMs: number) {
    if (seekInFlight) {
      pendingSeekMs = positionMs;
      return;
    }
    seekInFlight = true;
    try {
      await playbackSeek(positionMs);
      progressMs.value = positionMs;
    } catch (e) {
      console.error('Seek failed:', e);
    } finally {
      seekInFlight = false;
      if (pendingSeekMs != null) {
        const next = pendingSeekMs;
        pendingSeekMs = null;
        void seek(next);
      }
    }
  }

  // 来源管理 actions
  async function addSource(kind: 'local' | 'webdav', name: string, path: string, username?: string, password?: string): Promise<number> {
    if (kind === 'local') {
        try {
            const id: number = await sourceAddLocal(path, name);
            sources.value.push({ id, kind, name, path, isEnabled: true, lastScanned: "Never", username });
            return id;
        } catch (e) {
            console.error("Failed to add local source:", e);
            throw e;
        }
    } else if (kind === 'webdav') {
        try {
            const id: number = await sourceAddWebdav(path, name, username, password);
            sources.value.push({ id, kind, name, path, isEnabled: true, lastScanned: "Never", username });
            return id;
        } catch (e) {
            console.error("Failed to add webdav source:", e);
            throw e; // throw error so UI can show it
        }
    }
    throw new Error('Unsupported source kind');
  }

  async function fetchSources() {
    try {
      const result: SourceDTO[] = await sourceList();
      sources.value = result.map(s => ({
        id: s.id,
        kind: s.kind as 'local' | 'webdav',
        name: s.name,
        path: s.root_uri,
        isEnabled: s.enabled,
        lastScanned: s.last_scan_at ? new Date(s.last_scan_at).toLocaleString() : 'Never',
        // 后端只在扫描成功时推进 last_scan_at，失败信息全在这里（I3/G-08）
        lastError: s.last_error ?? undefined,
        username: s.username ?? undefined
      }));
    } catch (e) {
      console.error("Failed to fetch sources:", e);
    }
  }

  async function removeSource(id: number) {
    try {
      await sourceRemove(id);
      sources.value = sources.value.filter(s => s.id !== id);
      fetchTracks(true);
    } catch (e) {
      console.error("Failed to remove source:", e);
    }
  }

  // MA1（A1-3）：添加本地来源 → 自动开始扫描
  async function addLocalSource(path: string, name?: string) {
    const sourceName = name?.trim() || path.split(/[\\/]/).filter(Boolean).pop() || path;
    await sourceAddLocal(path, sourceName);
    await fetchSources();
    const created = sources.value.find(s => s.path === path || s.name === sourceName);
    if (created) await scanSource(created.id);
    return created?.id;
  }

  function toggleSource(id: number) {
    const source = sources.value.find(s => s.id === id);
    if (source) {
      source.isEnabled = !source.isEnabled;
    }
  }

  async function scanSource(id: number) {
    const source = sources.value.find(s => s.id === id);
    if (source) {
      source.lastScanned = "Scanning...";
      try {
        await sourceScan(id);
        // Don't set "Just now" here, the backend will emit `scan-progress` and `scan-complete`.
      } catch (e) {
        console.error("Scan failed:", e);
        source.lastScanned = "Error";
      }
    }
  }

  // ===== 全局 Tauri 事件监听器（P1-8: 保存 unlisten 引用，HMR 时清理） =====
  let unlistenScanProgress: (() => void) | null = null;
  let unlistenScanComplete: (() => void) | null = null;
  let unlistenArtworkBackfill: (() => void) | null = null;
  let unlistenAlbumCoverFetched: (() => void) | null = null;
  let unlistenArtistCoverFetched: (() => void) | null = null;

  async function initEventListeners() {
    // 先清理可能残留的旧监听（Vite HMR 场景）
    unlistenScanProgress?.();
    unlistenScanComplete?.();
    unlistenArtworkBackfill?.();
    unlistenAlbumCoverFetched?.();
    unlistenArtistCoverFetched?.();

    unlistenScanProgress = await listen('scan-progress', (event: any) => {
      const payload = event.payload as { source_id: number; scanned_count: number; skipped_count?: number; current_path: string };
      const source = sources.value.find(s => s.id === payload.source_id);
      if (source) {
        const skipped = payload.skipped_count ? `，跳过 ${payload.skipped_count}` : '';
        source.lastScanned = `扫描中: ${payload.scanned_count} 首${skipped}...`;
      }
    });

    unlistenScanComplete = await listen<ScanCompleteEvent>('scan-complete', async (event) => {
      // 扫描结果一律以重新拉取为准：写死"刚刚扫描"会把失败的扫描报成成功
      // （后端 last_scan_at 只在成功时推进，失败原因在 last_error）。
      await fetchSources();
      await fetchTracks(true);
      await fetchAlbums(true);
      await fetchArtists(true);
      // 扫描可能修复/删除了文件，可播性缓存整体失效，由各列表视图按需重拉
      invalidatePlayability();

      // 早退兜底（CR-006）：后端把终态写进 sources 时前端回读即可拿到原因；
      // 但数据库当时不可用（连接池耗尽等）就写不进去，表里仍是上一次的状态。
      // 事件里的原因比表里的状态新：直接盖到这条来源上，否则用户只看到"没变化"。
      const payload = event.payload;
      if (payload && payload.success === false && payload.message && payload.persisted === false) {
        const source = sources.value.find(s => s.id === payload.source_id);
        if (source) {
          source.lastError = payload.message;
        }
      }
    });

    unlistenArtworkBackfill = await listen('artwork-backfill-complete', async () => {
      console.log('[artwork-backfill-complete] 缩略图回填完成，重新拉取专辑列表');
      await fetchAlbums(true);
    });

    // 第七轮：封面后台拉取完成事件 —— 命令已改为立即返回，UI 更新由这里接管。
    // v1.8.1：事件携带 200x200 缩略图 data URL，网格即时显示（数据已同时入库持久化）
    unlistenAlbumCoverFetched = await listen<{ target_id: number; artwork_id: number; cover_thumbnail_base64?: string }>('album-cover-fetched', (event) => {
      const { target_id, artwork_id, cover_thumbnail_base64 } = event.payload;
      const foundAlbum = albums.value.find(a => a.id === target_id);
      if (foundAlbum) {
        foundAlbum.cover_artwork_id = artwork_id;
        if (cover_thumbnail_base64) foundAlbum.cover_thumb = cover_thumbnail_base64;
        // albums 是 shallowRef，浅拷贝整体替换触发网格重渲染
        albums.value = [...albums.value];
      }
      if (currentAlbumDetailsData.value?.id === target_id) {
        currentAlbumDetailsData.value.cover_artwork_id = artwork_id;
      }
    });

    unlistenArtistCoverFetched = await listen<{ target_id: number; artwork_id: number }>('artist-cover-fetched', (event) => {
      const { target_id, artwork_id } = event.payload;
      const foundArtist = artists.value.find(a => a.id === target_id);
      if (foundArtist) foundArtist.avatar_artwork_id = artwork_id;
      if (currentArtistDetailsData.value?.id === target_id) {
        currentArtistDetailsData.value.avatar_artwork_id = artwork_id;
      }
      // 收藏的歌手列表是独立数组，不跟着 artists 走（收藏页补头像就靠这里回填）
      const favArtist = favoriteArtists.value.find(a => a.id === target_id);
      if (favArtist) {
        favArtist.avatar_artwork_id = artwork_id;
        favoriteArtists.value = [...favoriteArtists.value];
      }
    });
  }

  initEventListeners();

  // ===== MediaSession API：接管系统媒体键与系统媒体通知 =====

  /**
   * 更新系统媒体通知（Windows SMTC / macOS Now Playing）中的曲目元数据。
   * 切歌时调用，将当前曲目的标题、艺人、专辑和封面同步给操作系统。
   */
  function updateMediaSessionMetadata(track: Track) {
    if (!('mediaSession' in navigator)) return;
    const artwork: MediaImage[] = [];
    if (track.cover_artwork_id) {
      artwork.push({
        src: getArtworkUrl(track.cover_artwork_id),
        sizes: '512x512',
        type: 'image/jpeg',
      });
    }
    navigator.mediaSession.metadata = new MediaMetadata({
      title: track.title,
      artist: track.artist,
      album: track.album,
      artwork,
    });
  }

  /**
   * 初始化 MediaSession action handler。
   * 将键盘多媒体键（播放/暂停/上一首/下一首/快进快退）映射到 store 中的对应操作。
   * Tauri 的 WebView 原生支持 MediaSession，注册后操作系统会自动接管。
   */
  function setupMediaSession() {
    if (!('mediaSession' in navigator)) return;

    navigator.mediaSession.setActionHandler('play', () => {
      if (!isPlaying.value) togglePlay();
    });
    navigator.mediaSession.setActionHandler('pause', () => {
      if (isPlaying.value) togglePlay();
    });
    navigator.mediaSession.setActionHandler('previoustrack', () => {
      prevTrack();
    });
    navigator.mediaSession.setActionHandler('nexttrack', () => {
      nextTrack(false);
    });
    navigator.mediaSession.setActionHandler('seekto', (details) => {
      if (details.seekTime != null) {
        seek(details.seekTime * 1000);
      }
    });

    console.log('[MediaSession] 已注册媒体键 handler');
  }

  setupMediaSession();

  // 页面关闭前保存播放进度（P0-3 兜底）
  if (typeof window !== 'undefined') {
    window.addEventListener('beforeunload', saveProgressToStorage);
  }

  return {
    isPlaying,
    isBuffering,
    volume,
    queue,
    currentIndex,
    playMode,
    progressMs,
    durationMs,
    activeLibraryTab,
    activeSourceTab,
    activeRightTab,
    isRightPanelOpen,
    playlists,
    activeAlbumId,
    activeArtistId,
    activePlaylistId,
    isCreatePlaylistModalOpen,
    createPlaylist,
    lyrics,
    playability,
    playabilityEpoch,
    ensurePlayability,
    getPlayability,
    isTrackUnplayable,
    invalidatePlayability,
    sources,
    localSources,
    webdavSources,
    albums,
    artists,
    tracks,
    tracksTotalCount,
    libraryCounts,
    currentTime,
    playTrack,
    playAll,
    fetchTracks,
    fetchAlbums,
    fetchArtists,
    hideSmallAlbums,
    hideMinorArtists,
    toggleHideSmallAlbums,
    toggleHideMinorArtists,
    fetchArtistTracks,
    fetchArtistAlbums,
    fetchPlaylists,
    fetchPlaylistTracks,
    fetchRecentlyPlayed,
    fetchFavoriteTracks,
    fetchFavoriteAlbums,
    fetchFavoriteArtists,
    favoriteAlbums,
    favoriteArtists,
    toggleFavoriteAlbum,
    toggleFavoriteArtist,
    addToPlaylist,
    fetchSources,
    toggleFavorite,
    currentFolderContents,
    activeFolderSourceId,
    activeFolderPath,
    folderBreadcrumbs,
    isFetchingFolder,
    fetchFolderContents,
    fetchMoreFolderEntries,
    hasMoreFolderEntries,
    addFolderToPlaylist,
    folderTreeChildren,
    folderTreeSourceRoot,
    folderTracks,
    folderTracksTotal,
    selectedTreePath,
    isLoadingFolderTracks,
    hasMoreFolderTracks,
    fetchFolderTreeChildren,
    fetchCounts,
    fetchFolderTracks,
    fetchMoreFolderTracks,
    searchQuery,
    globalSearchQuery,
    canGoBack,
    canGoForward,
    goBack,
    goForward,
    navigateToTab,
    goHome,
    navigateToArtist,
    navigateToAlbum,
    // 历史还原标志：视图读到后自行清除（详见 store 内声明处注释）
    isHistoryRestore,
    openPlaylist,
    // 首页数据
    stats,
    isLoadingStats,
    fetchStats,
    insights,
    isLoadingInsights,
    fetchInsights,
    // 批量操作
    batchSetFavorite,
    batchAddToPlaylist,
    currentTrack,
    currentAlbumDetails,
    currentArtistDetails,
    currentPlaylistDetails,
    formatTime,
    togglePlay,
    playQueue,
    recordPlay,
    nextTrack,
    prevTrack,
    setVolume,
    seek,
    // 播放速率（0.5–1.5）
    playbackRate,
    PLAYBACK_RATES,
    setPlaybackRate,
    addSource,
    addLocalSource,
    removeSource,
    toggleSource,
    scanSource,
    refreshCurrentPlaylistTracks,
    activeLyricIndex,
    currentTrackFileInfo,
    mapTrackDTO,
    isErrorTracks,
    isErrorAlbums,
    isErrorArtists,
    isLoadingTracks,
    isLoadingAlbums,
    isLoadingArtists,
    hasMoreTracks,
    restoreSession,
    fetchStartupBundle,
    deletePlaylist,
    removeTrackFromPlaylist,
    // 专辑无限滚动
    albumsTotalCount,
    hasMoreAlbums,
    // 艺人分页
    artistsTotalCount,
    hasMoreArtists,
    // 艺人详情页专辑分页
    nextArtistAlbumsPage,
    prevArtistAlbumsPage,
    goToArtistAlbumsPage,
    // 艺人详情页子标签（全部歌曲 / 全部专辑）
    setArtistDetailSubTab,
    // 智能歌单
    activeSmartPlaylistKind,
    smartPlaylistTracks,
    isLoadingSmartPlaylist,
    loadSmartPlaylist,
  };
});
