import { defineStore } from "pinia";
import { ref, computed } from "vue";
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

  // 歌词数据
  const lyrics = ref([
    { text: "We are going on a journey", time: 0 },
    { text: "A journey to experience", time: 15 },
    { text: "We are going on a journey", time: 30 },
    { text: "A journey to experience", time: 45, isActive: true }, // 当前播放的高亮行
    { text: "We are going on a journey", time: 60 },
    { text: "A journey to experience", time: 75 },
    { text: "We are going on a journey", time: 90 },
    { text: "A journey to experience", time: 105 },
    { text: "Close your eyes", time: 120 },
    { text: "Let the music take you higher", time: 135 },
    { text: "Close your eyes", time: 150 },
    { text: "Let the music take you higher", time: 165 },
  ]);

  // 歌单数据
  const playlists = ref<Playlist[]>([]);

  // 来源数据
  const sources = ref<MusicSource[]>([]);

  // 专辑数据
  const albums = ref<Album[]>([
    { id: 1, title: "Sleep", artist: "Max Richter", year: 2015, coverColor: "from-blue-600 to-indigo-900" },
    { id: 2, title: "The Blue Notebooks", artist: "Max Richter", year: 2004, coverColor: "from-purple-600 to-slate-900" },
    { id: 3, title: "Divenire", artist: "Ludovic Einaudi", year: 2006, coverColor: "from-amber-500 via-orange-600 to-stone-900" },
    { id: 4, title: "Una Mattina", artist: "Ludovic Einaudi", year: 2004, coverColor: "from-cyan-600 to-emerald-950" },
    { id: 5, title: "Ma Fleur", artist: "The Cinematic Orchestra", year: 2007, coverColor: "from-teal-500 to-neutral-900" },
    { id: 6, title: "The Earth Is Not a Cold Dead Place", artist: "Explosions in the Sky", year: 2003, coverColor: "from-red-500 to-zinc-900" },
    { id: 7, title: "Bon Iver", artist: "Bon Iver", year: 2011, coverColor: "from-lime-600 to-stone-950" },
    { id: 8, title: "Takk...", artist: "Sigur Rós", year: 2005, coverColor: "from-rose-600 to-zinc-900" },
  ]);

  // 艺人数据
  const artists = ref<Artist[]>([
    { id: 1, name: "Max Richter", trackCount: 24, avatarColor: "from-blue-600 to-indigo-900" },
    { id: 2, name: "Ludovic Einaudi", trackCount: 18, avatarColor: "from-amber-500 to-stone-900" },
    { id: 3, name: "The Cinematic Orchestra", trackCount: 12, avatarColor: "from-teal-500 to-neutral-900" },
    { id: 4, name: "Explosions in the Sky", trackCount: 9, avatarColor: "from-red-500 to-zinc-900" },
    { id: 5, name: "Alexandre Desplat", trackCount: 31, avatarColor: "from-sky-700 to-zinc-950" },
    { id: 6, name: "Bon Iver", trackCount: 14, avatarColor: "from-lime-600 to-stone-950" },
    { id: 7, name: "Sigur Rós", trackCount: 16, avatarColor: "from-rose-600 to-zinc-900" },
    { id: 8, name: "Hans Zimmer", trackCount: 42, avatarColor: "from-blue-950 to-zinc-955" },
    { id: 9, name: "Yann Tiersen", trackCount: 20, avatarColor: "from-yellow-600 to-stone-900" },
  ]);

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
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
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
        count: p.track_count
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function toggleFavorite(trackId: number) {
    const track = tracks.value.find(t => t.id === trackId);
    if (!track) return;
    const newStatus = !track.isFavorite;
    track.isFavorite = newStatus;
    try {
      await invoke('library_toggle_favorite', { trackId, isFavorite: newStatus });
    } catch (e) {
      console.error(e);
      track.isFavorite = !newStatus; // fallback
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
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
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
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
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
        coverColor: getDeterministicColor(t.album_title || t.title || 'Unknown'),
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

  async function fetchAlbums(reset: boolean = false) { void reset; }
  async function fetchArtists(reset: boolean = false) { void reset; }

  const currentTrack = computed(() => {
    return queue.value[currentIndex.value] || null;
  });

  const currentAlbumDetails = computed(() => {
    if (!activeAlbumId.value) return null;
    const album = albums.value.find(a => a.id === activeAlbumId.value);
    if (!album) return null;
    const albumTracks = tracks.value.filter(t => t.album === album.title);
    return { ...album, tracks: albumTracks };
  });

  const currentArtistDetails = computed(() => {
    if (!activeArtistId.value) return null;
    const artist = artists.value.find(a => a.id === activeArtistId.value);
    if (!artist) return null;
    const artistTracks = tracks.value.filter(t => t.artist === artist.name);
    const artistAlbums = albums.value.filter(a => a.artist === artist.name);
    return { ...artist, tracks: artistTracks, albums: artistAlbums };
  });

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
    lyrics,
    sources,
    localSources,
    webdavSources,
    albums,
    artists,
    tracks,
    currentTime,
    playTrack,
    fetchTracks,
    fetchAlbums,
    fetchArtists,
    fetchPlaylists,
    fetchPlaylistTracks,
    fetchRecentlyPlayed,
    fetchFavoriteTracks,
    addToPlaylist,
    fetchSources,
    toggleFavorite,
    searchQuery,
    currentTrack,
    currentAlbumDetails,
    currentArtistDetails,
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
  };
});
