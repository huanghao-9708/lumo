import { defineStore } from "pinia";
import { ref, computed, watch, shallowRef } from "vue";
import { listen } from '@tauri-apps/api/event';

import {
  libraryGetTracks, libraryGetAlbums, libraryGetAlbumCount, libraryGetArtists, libraryGetAlbumTracks, libraryGetArtistAlbums, libraryGetArtistAlbumCount, libraryGetArtistTracks, libraryGetArtistStats,
  libraryCreatePlaylist, libraryGetPlaylists, libraryAddToPlaylist, libraryGetPlaylistTracks, libraryDeletePlaylist, libraryRemovePlaylistItem, libraryAddFolderToPlaylist,
  libraryToggleFavorite, libraryRecordPlay, libraryGetRecentlyPlayed, libraryGetFavoriteTracks, librarySavePlayQueue, libraryGetPlayQueue,
  libraryGetFolderContents
, libraryGetLyrics, libraryGetTrackFileInfo } from '../api/library';
import {
  playbackPlay, playbackPause, playbackResume, playbackSetVolume, playbackGetPos, playbackSeek, playbackIsFinished, playbackEnqueueNext, playbackGetQueueLen
} from '../api/playback';
import {
  sourceAddLocal, sourceAddWebdav, sourceList, sourceRemove, sourceScan
} from '../api/scanner';


// ================= 后端 DTO 接口（与 Rust 端 models.rs 保持一致） =================
import type { TrackDTO, ArtistDTO, PlaylistDTOBackend } from '../api/types';

// ================= 前端展示模型 =================

export interface Track {
  id: number;
  title: string;
  artist: string;
  album: string;
  duration: string;
  durationSec: number;
  format: string;
  coverColor: string;
  cover_artwork_id?: number | null;
  isFavorite: boolean;
  primary_file_id?: number | null;
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
      artist: t.artist_name || '未知艺人',
      album: t.album_title || '未知专辑',
      duration: formatTime(durationMs / 1000),
      durationSec: Math.floor(durationMs / 1000),
      format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
      coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
      cover_artwork_id: t.cover_artwork_id,
      isFavorite: t.is_favorite || false,
      primary_file_id: t.media_file_id,
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

  const currentTrackFileInfo = ref<any>(null);
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

  // 页面导航历史栈
  interface HistoryState {
    tab: string;
    albumId: number | null;
    artistId: number | null;
    playlistId: number | null;
  }
  const historyStack = ref<HistoryState[]>([]);
  const isGoingBack = ref(false);

  // 监听导航状态变化以记录历史
  watch([activeLibraryTab, activeAlbumId, activeArtistId, activePlaylistId], (_newVals, oldVals) => {
    if (isGoingBack.value) {
      isGoingBack.value = false;
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
  });

  const canGoBack = computed(() => historyStack.value.length > 0);

  function goBack() {
    if (historyStack.value.length > 0) {
      isGoingBack.value = true;
      const state = historyStack.value.pop()!;
      activeLibraryTab.value = state.tab;
      activeAlbumId.value = state.albumId;
      activeArtistId.value = state.artistId;
      activePlaylistId.value = state.playlistId;
    }
  }

  // 歌词数据
  const lyrics = ref<any[]>([]);

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

  // 歌曲数据列表
  const tracks = ref<Track[]>([]);

  // 分页与加载状态
  const tracksLimit = 50;
  let tracksOffset = 0;
  const hasMoreTracks = ref(true);
  const isLoadingTracks = ref(false);
  const searchQuery = ref("");

  // 从后端获取歌曲列表
  async function fetchTracks(reset = false) {
    if (reset) {
      tracks.value = [];
      tracksOffset = 0;
      hasMoreTracks.value = true;
    }

    if (!hasMoreTracks.value || isLoadingTracks.value) return;
    isLoadingTracks.value = true;

    try {
      isErrorTracks.value = false;
      const result: TrackDTO[] = await libraryGetTracks(
          tracksLimit,
          tracksOffset,
          searchQuery.value || undefined
      );

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

  // ============ 专辑分页（15张/页 = 5列×3行）============
  // 放弃无限下拉+虚拟滚动方案,改为传统分页。
  // 原因：674 张专辑的下拉加载每次都要等 50 张返回+渲染,
  // 且封面加载会持续挤占 IPC 通道导致卡顿 1-3s。
  // 分页每次只加载 15 张,渲染开销和 IPC 压力都降到最低。
  const albumsPageSize = 15;
  const albumsCurrentPage = ref(1);
  const albumsTotalCount = ref(0);
  const isLoadingAlbums = ref(false);
  const isErrorAlbums = ref(false);

  // 总页数 = ceil(总数 / 每页大小),总数从后端 count 接口获取
  const albumsTotalPages = computed(() => {
    if (albumsTotalCount.value === 0) return 1;
    return Math.ceil(albumsTotalCount.value / albumsPageSize);
  });

  async function fetchAlbums(reset: boolean = false) {
    if (reset) {
      albumsCurrentPage.value = 1;
    }
    if (isLoadingAlbums.value) return;
    isLoadingAlbums.value = true;
    try {
      isErrorAlbums.value = false;
      // 并行拉取当前页数据 + 总数(只在 reset 或首次加载时拉总数,避免每次翻页都查)
      const offset = (albumsCurrentPage.value - 1) * albumsPageSize;
      const needCount = reset || albumsTotalCount.value === 0;

      const fetchList = libraryGetAlbums(albumsPageSize, offset, searchQuery.value || undefined);
      const fetchCount = needCount ? libraryGetAlbumCount(searchQuery.value || undefined) : Promise.resolve(albumsTotalCount.value);

      const [result, count] = await Promise.all([fetchList, fetchCount]);
      albumsTotalCount.value = count;

      const newAlbums: Album[] = result.map((a) => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: (a as any).release_year || new Date().getFullYear(),
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        cover_thumb: a.cover_thumbnail_base64,
        artist_name: a.artist_name,
        track_count: a.track_count
      }));
      albums.value = newAlbums;
    } catch (e) {
      console.error(e);
      isErrorAlbums.value = true;
    } finally {
      isLoadingAlbums.value = false;
    }
  }

  async function goToAlbumsPage(page: number) {
    if (page < 1 || page > albumsTotalPages.value || isLoadingAlbums.value) return;
    albumsCurrentPage.value = page;
    await fetchAlbums(false);
  }

  function nextAlbumsPage() {
    if (albumsCurrentPage.value >= albumsTotalPages.value) return;
    goToAlbumsPage(albumsCurrentPage.value + 1);
  }

  function prevAlbumsPage() {
    if (albumsCurrentPage.value <= 1) return;
    goToAlbumsPage(albumsCurrentPage.value - 1);
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
        track_count: a.track_count
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

  const currentAlbumDetailsData = ref<any>(null);
  const currentArtistDetailsData = ref<any>(null);
  const currentPlaylistDetailsData = ref<any>(null);
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
       const album = albums.value.find(a => a.id === newId);
       if (album) {
         try {
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
        year: (a as any).release_year || new Date().getFullYear(),
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
      const artist = artists.value.find(a => a.id === newId) || { id: newId, name: '未知艺人', avatarColor: getDeterministicColor('未知艺人') };

      currentArtistDetailsData.value = {
        ...artist,
        stats: { track_count: 0, album_count: 0 },
        tracks: [],
        albums: [],
        tracksOffset: 0,
        albumsOffset: 0,
        albumsCurrentPage: 1,
        albumsTotalCount: 0,
        albumsTotalPages: 1,
        hasMoreTracks: true,
        hasMoreAlbums: true,
        isLoadingTracks: false,
        isLoadingAlbums: false
      };

      try {
        const stats: any = await libraryGetArtistStats(newId);
        currentArtistDetailsData.value.stats = stats;
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
  watch(progressMs, (newProgress) => {
    localStorage.setItem('lumo_progress_ms', String(newProgress));
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
        playMode.value = savedMode as any;
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
      }
    } catch (e) {
      console.error("Toggle play failed:", e);
    }
  }

  let actualListenMs = 0;
  let hasEnqueuedNext = false;
    let enqueuedTrackIndex: number | null = null;

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
                
                return; // 直接返回，等待下一个轮询获取新歌进度
             }
           } catch(e) {}
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

  async function playQueue(newQueue: Track[], index: number) {
    // 切歌前记录上一首的播放时长
    if (queue.value && queue.value[currentIndex.value] && actualListenMs > 0) {
      recordPlay(queue.value[currentIndex.value].id, actualListenMs);
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
        persistPlayQueueIfNeeded();
      } catch (e) {
        console.error("Play failed:", e);
      }
    }
  }

  async function nextTrack(isAuto = false) {
    if (queue.value.length === 0) return;
    if (isAuto && playMode.value === 'repeat-one') {
      // Keep currentIndex unchanged，但需要重新载入播放
      await playQueue(queue.value, currentIndex.value);
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
    if (playMode.value === 'shuffle') {
      currentIndex.value = Math.floor(Math.random() * queue.value.length);
    } else {
      currentIndex.value = (currentIndex.value - 1 + queue.value.length) % queue.value.length;
    }
    await playQueue(queue.value, currentIndex.value);
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
      const result: any[] = await sourceList();
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

  // Register global listeners for scan events
  listen('scan-progress', (event: any) => {
    const payload = event.payload as { source_id: number; scanned_count: number; skipped_count?: number; current_path: string };
    const source = sources.value.find(s => s.id === payload.source_id);
    if (source) {
      const skipped = payload.skipped_count ? `，跳过 ${payload.skipped_count}` : '';
      source.lastScanned = `扫描中: ${payload.scanned_count} 首${skipped}...`;
    }
  });

  listen('scan-complete', async (event: any) => {
    const sourceId = event.payload as number;
    const source = sources.value.find(s => s.id === sourceId);
    if (source) {
      source.lastScanned = "刚刚扫描";
    }
    await fetchTracks(true); // Reset and fetch
    await fetchAlbums(true);
    await fetchArtists(true);
  });

  // 监听后端 artwork 缩略图回填完成事件。
  // 回填在应用启动后后台执行（674 张图片约 20-40s），
  // 完成后重新拉取专辑列表,让前端拿到内联 base64 缩略图,不再走 lumo:// 协议。
  listen('artwork-backfill-complete', async () => {
    console.log('[artwork-backfill-complete] 缩略图回填完成，重新拉取专辑列表');
    await fetchAlbums(true);
  });

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
    searchQuery,
    canGoBack,
    goBack,
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
    // 专辑分页
    albumsCurrentPage,
    albumsTotalPages,
    albumsTotalCount,
    albumsPageSize,
    nextAlbumsPage,
    prevAlbumsPage,
    goToAlbumsPage,
    // 艺人详情页专辑分页
    nextArtistAlbumsPage,
    prevArtistAlbumsPage,
    goToArtistAlbumsPage,
  };
});
