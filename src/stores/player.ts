import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { invoke } from '@tauri-apps/api/core';

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
}

export interface Album {
  id: number;
  title: string;
  artist: string;
  year: number;
  coverColor: string;
  cover_artwork_id?: number | null;
  artist_name?: string;
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

  // 基础状态
  const isDarkMode = ref(false);
  const isPlaying = ref(false);
  const volume = ref(75);
  
  const queue = ref<any[]>([]);
  const currentIndex = ref(-1);
  const playMode = ref<'normal'|'repeat'|'shuffle'>('normal');
  const progressMs = ref(0);
  const durationMs = ref(0);
  let progressTimer: ReturnType<typeof setInterval> | null = null;
  
  const activeLibraryTab = ref("全部歌曲");
  const activeSourceTab = ref("本地音乐库");
  const activeRightTab = ref<"歌词" | "播放队列" | "文件信息">("歌词");
  const isRightPanelOpen = ref(true);

  const activeAlbumId = ref<number | null>(null);
  const activeArtistId = ref<number | null>(null);
  const activePlaylistId = ref<number | null>(null);

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
  const albums = ref<Album[]>([]);

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
      const result: any[] = await invoke('library_get_tracks', { 
        limit: tracksLimit, 
        offset: tracksOffset,
        searchKeyword: searchQuery.value || null
      });
      
      if (result.length < tracksLimit) {
        hasMoreTracks.value = false;
      }
      
      const newTracks = result.map((t: any) => ({
        ...t,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || '未知专辑',
        duration: formatTime(t.duration_ms / 1000),
        durationSec: Math.floor(t.duration_ms / 1000),
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
        isFavorite: t.is_favorite || false,
        primary_file_id: t.media_file_id
      }));

      tracks.value.push(...newTracks);
      tracksOffset += result.length;
    } catch (e) {
      console.error("Failed to fetch tracks:", e);
    } finally {
      isLoadingTracks.value = false;
    }
  }

  async function fetchPlaylists() {
    try {
      const result: any[] = await invoke('library_get_playlists');
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
      const targetTracks: Track[] = [];
      
      if (tracks.value) {
        const trackInTracks = tracks.value.find(t => t && t.id === trackId);
        if (trackInTracks) targetTracks.push(trackInTracks);
      }
      
      if (currentAlbumDetailsData.value?.tracks) {
        const trackInAlbum = currentAlbumDetailsData.value.tracks.find((t: any) => t && t.id === trackId);
        if (trackInAlbum) targetTracks.push(trackInAlbum);
      }
      
      if (currentPlaylistDetailsData.value?.tracks) {
        const trackInPlaylist = currentPlaylistDetailsData.value.tracks.find((t: any) => t && t.id === trackId);
        if (trackInPlaylist) targetTracks.push(trackInPlaylist);
      }
      
      if (currentArtistDetailsData.value?.tracks) {
        const trackInArtist = currentArtistDetailsData.value.tracks.find((t: any) => t && t.id === trackId);
        if (trackInArtist) targetTracks.push(trackInArtist);
      }
      
      if (queue.value) {
        const trackInQueue = queue.value.find(t => t && t.id === trackId);
        if (trackInQueue) targetTracks.push(trackInQueue);
      }
      
      if (targetTracks.length === 0) {
        // 备选方案：如果都没有，直接向后端发送 toggle，并且乐观添加/移除
        await invoke('library_toggle_favorite', { trackId, isFavorite: true });
        return;
      }
      
      const newStatus = !targetTracks[0].isFavorite;
      targetTracks.forEach(t => {
        if (t) t.isFavorite = newStatus;
      });
      
      try {
        await invoke('library_toggle_favorite', { trackId, isFavorite: newStatus });
      } catch (e) {
        console.error("Backend failed to toggle favorite:", e);
        targetTracks.forEach(t => {
          if (t) t.isFavorite = !newStatus; // rollback
        });
      }
    } catch (e) {
      console.error("Exception in toggleFavorite:", e);
    }
  }

  async function recordPlay(trackId: number) {
    try {
      await invoke('library_record_play', { trackId });
    } catch(e) {
      console.error("Failed to record play:", e);
    }
  }

  async function addToPlaylist(playlistId: number, trackId: number) {
    try {
      await invoke('library_add_to_playlist', { playlistId, trackId });
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
      const result: any[] = await invoke('library_get_playlist_tracks', { playlistId });
      tracks.value = result.map((t: any) => ({
        ...t,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || '未知专辑',
        duration: formatTime(t.duration_ms / 1000),
        durationSec: Math.floor(t.duration_ms / 1000),
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
        isFavorite: t.is_favorite || false,
        primary_file_id: t.media_file_id
      }));
      hasMoreTracks.value = false;
      tracksOffset = tracks.value.length;
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchRecentlyPlayed() {
    try {
      const result: any[] = await invoke('library_get_recently_played', { limit: 50 });
      tracks.value = result.map((t: any) => ({
        ...t,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || '未知专辑',
        duration: formatTime(t.duration_ms / 1000),
        durationSec: Math.floor(t.duration_ms / 1000),
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
        isFavorite: t.is_favorite || false,
        primary_file_id: t.media_file_id
      }));
      hasMoreTracks.value = false;
    } catch(e) {
      console.error(e);
    }
  }

  async function fetchFavoriteTracks() {
    try {
      const result: any[] = await invoke('library_get_favorite_tracks');
      tracks.value = result.map((t: any) => ({
        ...t,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || '未知专辑',
        duration: formatTime(t.duration_ms / 1000),
        durationSec: Math.floor(t.duration_ms / 1000),
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
        isFavorite: t.is_favorite || false,
        primary_file_id: t.media_file_id
      }));
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

  const albumsLimit = 50;
  let albumsOffset = 0;
  const hasMoreAlbums = ref(true);
  const isLoadingAlbums = ref(false);

  async function fetchAlbums(reset: boolean = false) {
    if (reset) {
      albums.value = [];
      albumsOffset = 0;
      hasMoreAlbums.value = true;
    }
    if (!hasMoreAlbums.value || isLoadingAlbums.value) return;
    isLoadingAlbums.value = true;
    try {
      const result: any[] = await invoke('library_get_albums', {
        limit: albumsLimit,
        offset: albumsOffset,
        searchKeyword: searchQuery.value || null
      });
      if (result.length < albumsLimit) {
        hasMoreAlbums.value = false;
      }
      const newAlbums = result.map((a: any) => ({
        id: a.id,
        title: a.title,
        artist: a.artist_name || '未知艺人',
        year: a.release_year || new Date().getFullYear(),
        coverColor: getDeterministicColor(a.title || 'Unknown'),
        cover_artwork_id: a.cover_artwork_id,
        artist_name: a.artist_name,
        track_count: a.track_count
      }));
      albums.value.push(...newAlbums);
      albumsOffset += result.length;
    } catch (e) {
      console.error(e);
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
      const result: any[] = await invoke('library_get_artists', {
        limit: artistsLimit,
        offset: artistsOffset,
        searchKeyword: searchQuery.value || null
      });
      if (result.length < artistsLimit) {
        hasMoreArtists.value = false;
      }
      const newArtists = result.map((a: any) => ({
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

  // 监听当前播放曲目，自动加载对应歌词
  watch(currentTrack, async (newTrack) => {
    if (newTrack) {
      try {
        const lrcText = await invoke<string | null>('library_get_lyrics', { trackId: newTrack.id });
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
    } else {
      lyrics.value = [];
    }
  }, { immediate: true });

  const currentAlbumDetailsData = ref<any>(null);
  const currentArtistDetailsData = ref<any>(null);
  const currentPlaylistDetailsData = ref<any>(null);
  const isCreatePlaylistModalOpen = ref(false);

  const createPlaylist = async (name: string, description: string) => {
    try {
      const id = await invoke('library_create_playlist', { name, description });
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
      const result: any[] = await invoke('library_get_playlist_tracks', { playlistId });
      const tracksData = result.map(t => ({
         ...t,
         artist: t.artist_name || '未知艺人',
         album: t.album_title || '未知专辑',
         duration: formatTime(t.duration_ms / 1000),
         durationSec: Math.floor(t.duration_ms / 1000),
         format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
         coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
         isFavorite: t.is_favorite || false,
         primary_file_id: t.media_file_id
       }));
       
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
           const result: any[] = await invoke('library_get_album_tracks', { albumId: newId });
           const tracksData = result.map(t => ({
             ...t,
             artist: t.artist_name || '未知艺人',
             album: t.album_title || '未知专辑',
             duration: formatTime(t.duration_ms / 1000),
             durationSec: Math.floor(t.duration_ms / 1000),
             format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
             coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
             isFavorite: t.is_favorite || false,
             primary_file_id: t.media_file_id
           }));
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
      const tracksResult: any[] = await invoke('library_get_artist_tracks', { artistId, limit, offset });
      
      const tracksData = tracksResult.map(t => ({
            ...t,
            artist: t.artist_name || '未知艺人',
            album: t.album_title || '未知专辑',
            duration: formatTime(t.duration_ms / 1000),
            durationSec: Math.floor(t.duration_ms / 1000),
            format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
            coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'), cover_artwork_id: t.cover_artwork_id,
            isFavorite: t.is_favorite || false,
            primary_file_id: t.media_file_id
      }));

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

  const fetchArtistAlbums = async (artistId: number, isLoadMore = false) => {
    if (!currentArtistDetailsData.value) return;
    if (isLoadMore && (!currentArtistDetailsData.value.hasMoreAlbums || currentArtistDetailsData.value.isLoadingAlbums)) return;

    currentArtistDetailsData.value.isLoadingAlbums = true;
    try {
      const limit = 20;
      const offset = currentArtistDetailsData.value.albumsOffset;
      const albumsResult: any[] = await invoke('library_get_artist_albums', { artistId, limit, offset });
      
      const artistAlbums = albumsResult.map(a => ({
              id: a.id,
              title: a.title,
              artist: a.artist_name || '未知艺人',
              year: a.release_year || new Date().getFullYear(),
              coverColor: getDeterministicColor(a.title || 'Unknown'),
              cover_artwork_id: a.cover_artwork_id,
              artist_name: a.artist_name,
              track_count: a.track_count
      }));

      if (isLoadMore) {
        currentArtistDetailsData.value.albums.push(...artistAlbums);
      } else {
        currentArtistDetailsData.value.albums = artistAlbums;
      }
      currentArtistDetailsData.value.albumsOffset += artistAlbums.length;
      currentArtistDetailsData.value.hasMoreAlbums = artistAlbums.length === limit;
    } catch(e) {
      console.error(e);
    } finally {
      currentArtistDetailsData.value.isLoadingAlbums = false;
    }
  };

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
        hasMoreTracks: true,
        hasMoreAlbums: true,
        isLoadingTracks: false,
        isLoadingAlbums: false
      };

      try {
        const stats: any = await invoke('library_get_artist_stats', { artistId: newId });
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

  // 方法 (Actions)
  function formatTime(seconds: number): string {
    const min = Math.floor(seconds / 60);
    const sec = Math.floor(seconds % 60);
    return `${min.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
  }

  async function togglePlay() {
    try {
      if (isPlaying.value) {
        await invoke('playback_pause');
        isPlaying.value = false;
      } else {
        await invoke('playback_resume');
        isPlaying.value = true;
      }
    } catch (e) {
      console.error("Toggle play failed:", e);
    }
  }

  async function startProgressPolling() {
    if (progressTimer) clearInterval(progressTimer);
    progressTimer = setInterval(async () => {
      if (isPlaying.value) {
        try {
          const pos = await invoke<number>('playback_get_pos');
          progressMs.value = pos;
          if (durationMs.value > 0 && pos >= durationMs.value - 500) {
            nextTrack();
          }
        } catch (e) {
          console.error(e);
        }
      }
    }, 500);
  }

  async function playQueue(newQueue: any[], index: number) {
    queue.value = [...newQueue];
    currentIndex.value = index;
    const track = queue.value[index];
    if (track && track.primary_file_id) {
      durationMs.value = track.durationSec ? track.durationSec * 1000 : (track.duration_ms || 0);
      progressMs.value = 0;
      try {
        await invoke('playback_play', { mediaFileId: track.primary_file_id });
        isPlaying.value = true;
        startProgressPolling();
        recordPlay(track.id);
      } catch (e) {
        console.error("Play failed:", e);
      }
    }
  }

  async function nextTrack() {
    if (queue.value.length === 0) return;
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
      await invoke('playback_set_volume', { volume: v / 100 });
    } catch (e) {
      console.error(e);
    }
  }

  async function seek(positionMs: number) {
    try {
      await invoke('playback_seek', { positionMs });
      progressMs.value = positionMs;
    } catch (e) {
      console.error(e);
    }
  }

  // 来源管理 actions
  async function addSource(kind: 'local' | 'webdav', name: string, path: string, username?: string) {
    if (kind === 'local') {
      try {
        const id: number = await invoke('source_add_local', { path, name });
        sources.value.push({
          id,
          kind,
          name,
          path,
          isEnabled: true,
          lastScanned: "Never",
          username,
        });
      } catch (e) {
        console.error("Failed to add source:", e);
      }
    }
  }

  async function fetchSources() {
    try {
      const result: any[] = await invoke('source_list');
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
      await invoke('source_remove', { sourceId: id });
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
        await invoke('source_scan', { sourceId: id });
        source.lastScanned = "Just now";
        await fetchTracks(); // 扫描完刷新歌曲列表
      } catch (e) {
        console.error("Scan failed:", e);
        source.lastScanned = "Error";
      }
    }
  }

  return {
    isDarkMode,
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
  };
});
