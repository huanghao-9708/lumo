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
  libraryGetArtistById
} from '../api/library';
import {
  playbackPlay, playbackPause, playbackResume, playbackSetVolume, playbackGetPos, playbackSeek, playbackIsFinished, playbackEnqueueNext, playbackGetQueueLen
} from '../api/playback';
import {
  sourceAddLocal, sourceAddWebdav, sourceList, sourceRemove, sourceScan
} from '../api/scanner';


// ================= 后端 DTO 接口（与 Rust 端 models.rs 保持一致） =================
import type { TrackDTO, ArtistDTO, AlbumDTO, PlaylistDTOBackend, FolderChildrenResultDTO, FolderTracksResultDTO } from '../api/types';

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
}

export interface Playlist {
  id: number;
  name: string;
  count: number;
  description?: string | null;
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
  username?: string;
}

export interface FolderEntry {
  name: string;
  is_dir: boolean;
  path: string;
  track?: Track;
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

// ================= Store 实现 =================

export const usePlayerStore = defineStore("player", () => {
  function getDeterministicColor(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const colors = [
      'from-gray-500 to-gray-800',
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
    };
  }

  /** 把 `TrackDTO[]` 批量映射为前端 Track[] */
  function mapTrackList(list: TrackDTO[]): Track[] {
    return list.map(mapTrackDTO);
  }

  // 基础状态
  const isPlaying = ref(false);
  const volume = ref(75);

  const queue = ref<Track[]>([]);
  const currentIndex = ref(-1);
  const playMode = ref<'normal'|'repeat'|'repeat-one'|'shuffle'>('normal');
  const progressMs = ref(0);
  const durationMs = ref(0);
  let progressTimer: ReturnType<typeof setInterval> | null = null;

  const currentTrackFileInfo = ref<TrackFileInfoDTO | null>(null);
  const isErrorTracks = ref(false);
  const isErrorArtists = ref(false);
  const hasLoadedCurrentFile = ref(false);

  const activeLibraryTab = ref("全部歌曲");
  const activeSourceTab = ref("本地音乐库");
  const activeRightTab = ref<"歌词" | "播放队列" | "文件信息">("歌词");
  const isRightPanelOpen = ref(true);

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

  // 页面导航历史栈
  interface HistoryState {
    tab: string;
    albumId: number | null;
    artistId: number | null;
    playlistId: number | null;
  }
  const historyStack = ref<HistoryState[]>([]);
  const isGoingBack = ref(false);
  const forwardStack = ref<HistoryState[]>([]);
  const isGoingForward = ref(false);

  // 监听导航状态变化以记录历史
  watch([activeLibraryTab, activeAlbumId, activeArtistId, activePlaylistId], (_newVals, oldVals) => {
    if (isGoingBack.value) {
      isGoingBack.value = false;
      return;
    }
    if (isGoingForward.value) {
      isGoingForward.value = false;
      return;
    }
    const [oldTab, oldAlbumId, oldArtistId, oldPlaylistId] = oldVals;
    if (oldTab) {
      historyStack.value.push({
        tab: oldTab as string,
        albumId: oldAlbumId as number | null,
        artistId: oldArtistId as number | null,
        playlistId: oldPlaylistId as number | null
      });
    }
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
        playlistId: activePlaylistId.value
      });
      isGoingBack.value = true;
      const state = historyStack.value.pop()!;
      activeLibraryTab.value = state.tab;
      activeAlbumId.value = state.albumId;
      activeArtistId.value = state.artistId;
      activePlaylistId.value = state.playlistId;
    }
  }

  function goForward() {
    if (forwardStack.value.length > 0) {
      historyStack.value.push({
        tab: activeLibraryTab.value,
        albumId: activeAlbumId.value,
        artistId: activeArtistId.value,
        playlistId: activePlaylistId.value
      });
      isGoingForward.value = true;
      const state = forwardStack.value.pop()!;
      activeLibraryTab.value = state.tab;
      activeAlbumId.value = state.albumId;
      activeArtistId.value = state.artistId;
      activePlaylistId.value = state.playlistId;
    }
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

  // 来源数据
  const sources = ref<MusicSource[]>([]);

  // 专辑数据
  // [验证] 改为 shallowRef：只追踪 .value 的整体替换，不深度代理数组内对象。
// 目的：验证 invoke 慢是否由 Vue 对大数组的深度 reactive proxy 开销导致。
// 若验证成立，后续会把 tracks/queue 等也迁移（它们需要额外处理属性直改）。
const albums = shallowRef<Album[]>([]);

  // 艺人数据
  const artists = ref<Artist[]>([]);

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

  async function fetchPlaylists() {
    try {
      const result: PlaylistDTOBackend[] = await libraryGetPlaylists();
      playlists.value = result.map(p => ({
        id: p.id,
        name: p.name,
        description: p.description,
        count: p.track_count
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

  async function recordPlay(trackId: number, durationPlayed: number) {
    if (durationPlayed < 1000) return; // 忽略极短的切歌
    try {
      await libraryRecordPlay(trackId, durationPlayed);
    } catch(e) {
      console.error("Failed to record play:", e);
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

      const fetchList = libraryGetAlbums(albumsPageSize, albumsOffset, searchQuery.value || undefined);
      const fetchCount = needCount ? libraryGetAlbumCount(searchQuery.value || undefined) : Promise.resolve(albumsTotalCount.value);

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
      const result: ArtistDTO[] = await libraryGetArtists(
          artistsLimit,
          artistsOffset,
          searchQuery.value || undefined
      );
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

  // 监听当前播放曲目，自动加载对应歌词与文件元数据
  watch(currentTrack, async (newTrack) => {
    if (newTrack) {
      try {
        const lrcText = await libraryGetLyrics(newTrack.id);
        if (lrcText) {
          lyrics.value = parseLrc(lrcText);
        } else {
          lyrics.value = [
            { text: newTrack.title, time: 0 },
            { text: newTrack.artist, time: 3 },
            { text: "— 暂无歌词 —", time: 6 }
          ];
        }
      } catch (e) {
        console.error("Failed to load lyrics:", e);
        lyrics.value = [
          { text: newTrack.title, time: 0 },
          { text: "— 暂无歌词 —", time: 3 }
        ];
      }

      try {
        const fileInfo = await libraryGetTrackFileInfo(newTrack.id);
        currentTrackFileInfo.value = fileInfo;
      } catch (e) {
        console.error("Failed to load track file info:", e);
        currentTrackFileInfo.value = null;
      }
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
    const playlist = playlists.value.find(p => p.id === playlistId) || { id: playlistId, name: '未知歌单', count: 0, description: '' };
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
          // 不要 await 阻塞 tracks 的加载，改为异步后台执行
          if (album.cover_artwork_id == null) {
            libraryFetchMissingAlbumCover(newId).then((newCoverId) => {
              if (newCoverId) {
                // 如果当前选中的专辑还是这一个，更新它的封面
                if (currentAlbumDetailsData.value?.id === newId) {
                  currentAlbumDetailsData.value.cover_artwork_id = newCoverId;
                }
                // 更新列表中的封面
                const foundAlbum = albums.value.find(a => a.id === newId);
                if (foundAlbum) foundAlbum.cover_artwork_id = newCoverId;
              }
            }).catch(console.error);
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

  watch(activeArtistId, async (newId) => {
    if (newId) {
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

      try {
        if (artist.avatar_artwork_id == null) {
          // 不阻塞，异步获取封面
          libraryFetchMissingArtistCover(newId).then((newCoverId) => {
            if (newCoverId) {
              if (currentArtistDetailsData.value?.id === newId) {
                currentArtistDetailsData.value.avatar_artwork_id = newCoverId;
              }
              const foundArtist = artists.value.find(a => a.id === newId);
              if (foundArtist) foundArtist.avatar_artwork_id = newCoverId;
            }
          }).catch(console.error);
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
  function persistPlayQueueIfNeeded() {
    const sig = queue.value.map(t => t.id).join(',');
    if (sig === lastSavedQueueSignature) return;
    lastSavedQueueSignature = sig;
    librarySavePlayQueue(queue.value.map(t => t.id))
      .catch(e => console.error("Failed to auto-save play queue:", e));
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
  });
  watch(volume, (newVol) => {
    localStorage.setItem('lumo_volume', String(newVol));
  });

  // 恢复状态与队列
  async function restoreSession() {
    try {
      // 1. 恢复播放队列
      const savedQueue: TrackDTO[] = await libraryGetPlayQueue();
      if (savedQueue && savedQueue.length > 0) {
        queue.value = mapTrackList(savedQueue);
        lastSavedQueueSignature = queue.value.map(t => t.id).join(',');
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
    fetchCounts();
  }

  // ================= 歌单操作 Actions =================

  // 删除歌单
  async function deletePlaylist(playlistId: number) {
    try {
      await libraryDeletePlaylist(playlistId);
      await fetchPlaylists();
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
      await refreshCurrentPlaylistTracks(playlistId);
      await fetchPlaylists();
    } catch (e) {
      console.error("Failed to remove track from playlist:", e);
      throw e;
    }
  }

  async function togglePlay() {
    if (queue.value.length === 0) return;
    if (currentIndex.value === -1) {
      currentIndex.value = 0;
    }

    const track = queue.value[currentIndex.value];
    if (!track) return;

    try {
      if (isPlaying.value) {
        await playbackPause();
        isPlaying.value = false;
        stopProgressAutoSave();
      } else {
        if (!hasLoadedCurrentFile.value) {
          if (track.primary_file_id) {
            await playbackPlay(track.primary_file_id);
            hasLoadedCurrentFile.value = true;

            if (progressMs.value > 0) {
              await playbackSeek(progressMs.value);
            }
          }
        } else {
          await playbackResume();
        }
        isPlaying.value = true;
        startProgressPolling();
        startProgressAutoSave();
      }
    } catch (e) {
      console.error("Toggle play failed:", e);
    }
  }

  let actualListenMs = 0;
  let hasEnqueuedNext = false;
  let enqueuedTrackIndex: number | null = null;

  // Shuffle 模式播放历史栈：记录已播放的 index，用于 prevTrack 精确回退
  const playHistory: number[] = [];
  const MAX_PLAY_HISTORY = 100;

  async function startProgressPolling() {
    if (progressTimer) clearInterval(progressTimer);
    progressTimer = setInterval(async () => {
      if (!isPlaying.value) return;
      actualListenMs += 500;
      try {
        const pos = await playbackGetPos();
        progressMs.value = pos;

        /**
         * 无缝播放 (Gapless Playback) 预加载逻辑：
         * 倒数 5 秒时，将下一首歌送入底层解码器队列。
         */
        if (!hasEnqueuedNext && durationMs.value > 0 && pos >= durationMs.value - 5000) {
           let nextIdx = currentIndex.value;
           if (playMode.value === 'shuffle') {
             nextIdx = Math.floor(Math.random() * queue.value.length);
           } else if (playMode.value === 'repeat-one') {
             nextIdx = currentIndex.value;
           } else {
             nextIdx = (currentIndex.value + 1) % queue.value.length;
           }
           
           const nextTrackObj = queue.value[nextIdx];
           if (nextTrackObj && nextTrackObj.primary_file_id) {
             console.log("[Gapless] Pre-enqueuing next track:", nextTrackObj.title);
             try {
                await playbackEnqueueNext(nextTrackObj.primary_file_id);
                hasEnqueuedNext = true;
                                enqueuedTrackIndex = nextIdx;
             } catch(e) {
                console.error("[Gapless] Failed to enqueue next track", e);
             }
           }
        }

        /**
         * 状态轮询：静默状态切换 (Silent Switch)
         * 当队列长度回归到 1，说明第一首刚好播完，物理上已经进入了无缝的新一首。
         * 我们在这里偷偷切换前端的 UI 状态，不发 play 指令。
         */
        if (hasEnqueuedNext) {
           try {
             const queueLen = await playbackGetQueueLen();
             if (queueLen === 1) {
                console.log("[Gapless] Silent transition to next track!");
                if (queue.value && queue.value[currentIndex.value] && actualListenMs > 0) {
                   recordPlay(queue.value[currentIndex.value].id, actualListenMs);
                }

                currentIndex.value = enqueuedTrackIndex as number;
                const newTrack = queue.value[currentIndex.value];
                actualListenMs = 0;
                durationMs.value = newTrack.durationSec ? newTrack.durationSec * 1000 : 0;
                progressMs.value = 0;
                hasEnqueuedNext = false;
                                enqueuedTrackIndex = null;

                return;
             }
           } catch(e) {}
           return; // 已预加载 gapless，跳过下方传统兜底，避免竞态
        }

        // 传统切歌兜底判定
        const reachedEnd = durationMs.value > 0 && pos >= durationMs.value - 500;
        let backendFinished = false;
        if (!reachedEnd) {
          try {
            backendFinished = await playbackIsFinished();
          } catch {
            // 忽略
          }
        }
        if (reachedEnd || backendFinished) {
          nextTrack(true);
        }
      } catch (e) {
        console.error(e);
      }
    }, 500);
  }

  async function playQueue(newQueue: Track[], index: number, skipHistoryPush = false) {
    // 切歌前记录上一首的播放时长
    if (queue.value && queue.value[currentIndex.value] && actualListenMs > 0) {
      recordPlay(queue.value[currentIndex.value].id, actualListenMs);
    }

    // 记录播放历史（用于 shuffle 模式 prevTrack 精确回退）
    if (!skipHistoryPush && currentIndex.value >= 0 && currentIndex.value < queue.value.length) {
      playHistory.push(currentIndex.value);
      if (playHistory.length > MAX_PLAY_HISTORY) {
        playHistory.shift();
      }
    }

    queue.value = [...newQueue];
    currentIndex.value = index;
    const track = queue.value[index];
    actualListenMs = 0;
    if (track && track.primary_file_id) {
      durationMs.value = track.durationSec ? track.durationSec * 1000 : 0;
      progressMs.value = 0;
      try {
        const playbackDuration = await playbackPlay(track.primary_file_id);
        if (playbackDuration && playbackDuration > 0) {
          durationMs.value = playbackDuration;
        }
        isPlaying.value = true;
        hasLoadedCurrentFile.value = true;
        startProgressPolling();
        startProgressAutoSave();
        persistPlayQueueIfNeeded();
      } catch (e) {
        console.error("Play failed:", e);
      }
    }
  }

  async function nextTrack(isAuto = false) {
    // 重置 gapless 状态，防止竞态残留
    hasEnqueuedNext = false;
    enqueuedTrackIndex = null;
    if (queue.value.length === 0) return;
    if (isAuto && playMode.value === 'repeat-one') {
      await playQueue(queue.value, currentIndex.value, true);
      return;
    }
    if (playMode.value === 'shuffle') {
      currentIndex.value = Math.floor(Math.random() * queue.value.length);
    } else {
      currentIndex.value = (currentIndex.value + 1) % queue.value.length;
    }
    await playQueue(queue.value, currentIndex.value);
  }

  async function prevTrack() {
    if (queue.value.length === 0) return;
    if (playMode.value === 'shuffle' && playHistory.length > 0) {
      currentIndex.value = playHistory.pop()!;
    } else if (playMode.value === 'shuffle') {
      currentIndex.value = Math.floor(Math.random() * queue.value.length);
    } else {
      currentIndex.value = (currentIndex.value - 1 + queue.value.length) % queue.value.length;
    }
    await playQueue(queue.value, currentIndex.value, true);
  }

  async function setVolume(v: number) {
    volume.value = v;
    try {
      await playbackSetVolume(v / 100);
    } catch (e) {
      console.error(e);
    }
  }

  async function seek(positionMs: number) {
    try {
      await playbackSeek(positionMs);
      progressMs.value = positionMs;
    } catch (e) {
      console.error(e);
    }
  }

  // 来源管理 actions
  async function addSource(kind: 'local' | 'webdav', name: string, path: string, username?: string, password?: string) {
    if (kind === 'local') {
        try {
            const id: number = await sourceAddLocal(path, name);
            sources.value.push({ id, kind, name, path, isEnabled: true, lastScanned: "Never", username });
        } catch (e) {
            console.error("Failed to add local source:", e);
        }
    } else if (kind === 'webdav') {
        try {
            const id: number = await sourceAddWebdav(path, name, username, password);
            sources.value.push({ id, kind, name, path, isEnabled: true, lastScanned: "Never", username });
        } catch (e) {
            console.error("Failed to add webdav source:", e);
            throw e; // throw error so UI can show it
        }
    }
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
        username: s.credential_ref || undefined
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

  async function initEventListeners() {
    // 先清理可能残留的旧监听（Vite HMR 场景）
    unlistenScanProgress?.();
    unlistenScanComplete?.();
    unlistenArtworkBackfill?.();

    unlistenScanProgress = await listen('scan-progress', (event: any) => {
      const payload = event.payload as { source_id: number; scanned_count: number; skipped_count?: number; current_path: string };
      const source = sources.value.find(s => s.id === payload.source_id);
      if (source) {
        const skipped = payload.skipped_count ? `，跳过 ${payload.skipped_count}` : '';
        source.lastScanned = `扫描中: ${payload.scanned_count} 首${skipped}...`;
      }
    });

    unlistenScanComplete = await listen('scan-complete', async (event: any) => {
      const sourceId = event.payload as number;
      const source = sources.value.find(s => s.id === sourceId);
      if (source) {
        source.lastScanned = "刚刚扫描";
      }
      await fetchTracks(true);
      await fetchAlbums(true);
      await fetchArtists(true);
    });

    unlistenArtworkBackfill = await listen('artwork-backfill-complete', async () => {
      console.log('[artwork-backfill-complete] 缩略图回填完成，重新拉取专辑列表');
      await fetchAlbums(true);
    });
  }

  initEventListeners();

  // 页面关闭前保存播放进度（P0-3 兜底）
  if (typeof window !== 'undefined') {
    window.addEventListener('beforeunload', saveProgressToStorage);
  }

  return {
    isPlaying,
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
    currentTrack,
    currentAlbumDetails,
    currentArtistDetails,
    currentPlaylistDetails,
    formatTime,
    togglePlay,
    playQueue,
    nextTrack,
    prevTrack,
    setVolume,
    seek,
    addSource,
    removeSource,
    toggleSource,
    scanSource,
    refreshCurrentPlaylistTracks,
    activeLyricIndex,
    currentTrackFileInfo,
    isErrorTracks,
    isErrorAlbums,
    isErrorArtists,
    isLoadingTracks,
    isLoadingAlbums,
    isLoadingArtists,
    hasMoreTracks,
    restoreSession,
    deletePlaylist,
    removeTrackFromPlaylist,
    // 专辑无限滚动
    albumsTotalCount,
    hasMoreAlbums,
    // 艺人详情页专辑分页
    nextArtistAlbumsPage,
    prevArtistAlbumsPage,
    goToArtistAlbumsPage,
  };
});
