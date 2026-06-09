import { defineStore } from "pinia";
import { ref, computed } from "vue";

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
}

export interface Playlist {
  name: string;
  count: number;
}

export interface Album {
  id: number;
  title: string;
  artist: string;
  year: number;
  coverColor: string;
}

export interface Artist {
  id: number;
  name: string;
  trackCount: number;
  avatarColor: string;
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
  // 状态变量 (State)
  const isDarkMode = ref(false);
  const isPlaying = ref(false);
  const currentTime = ref(102); // 默认 01:42
  const volume = ref(75);
  
  const activeLibraryTab = ref("全部歌曲");
  const activeSourceTab = ref("本地音乐库");
  const activeRightTab = ref<"歌词" | "播放队列" | "文件信息">("歌词");
  const isRightPanelOpen = ref(true);
  const currentTrackIndex = ref(2);

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
  const playlists = ref<Playlist[]>([
    { name: "日常音乐", count: 58 },
    { name: "工作专注", count: 24 },
    { name: "放松时刻", count: 36 },
    { name: "90s 精选", count: 57 },
  ]);

  // 来源数据
  const sources = ref<MusicSource[]>([
    { id: 1, kind: 'local', name: "本地无损库", path: "C:\\Users\\hao\\Music\\Lossless", isEnabled: true, lastScanned: "2 hours ago" },
    { id: 2, kind: 'webdav', name: "群晖 NAS", path: "https://nas.local:5006/music", isEnabled: true, lastScanned: "1 day ago", username: "admin" },
  ]);

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
  const tracks = ref<Track[]>([
    {
      id: 1,
      title: "Intro",
      artist: "Max Richter",
      album: "Sleep",
      duration: "02:08",
      durationSec: 128,
      format: "FLAC",
      coverColor: "from-blue-600 to-indigo-900",
      isFavorite: false,
    },
    {
      id: 2,
      title: "On the Nature of Daylight",
      artist: "Max Richter",
      album: "The Blue Notebooks",
      duration: "06:12",
      durationSec: 372,
      format: "FLAC",
      coverColor: "from-purple-600 to-slate-900",
      isFavorite: false,
    },
    {
      id: 3,
      title: "Experience",
      artist: "Ludovic Einaudi",
      album: "Divenire",
      duration: "05:15",
      durationSec: 315,
      format: "FLAC",
      coverColor: "from-amber-500 via-orange-600 to-stone-900",
      isFavorite: true,
    },
    {
      id: 4,
      title: "Nuvole Bianche",
      artist: "Ludovic Einaudi",
      album: "Una Mattina",
      duration: "07:48",
      durationSec: 468,
      format: "FLAC",
      coverColor: "from-cyan-600 to-emerald-950",
      isFavorite: false,
    },
    {
      id: 5,
      title: "Arrival of the Birds",
      artist: "The Cinematic Orchestra",
      album: "Ma Fleur",
      duration: "06:10",
      durationSec: 370,
      format: "FLAC",
      coverColor: "from-teal-500 to-neutral-900",
      isFavorite: false,
    },
    {
      id: 6,
      title: "First Breath After Coma",
      artist: "Explosions in the Sky",
      album: "The Earth Is Not a Cold Dead Place",
      duration: "09:34",
      durationSec: 574,
      format: "FLAC",
      coverColor: "from-red-500 to-zinc-900",
      isFavorite: false,
    },
    {
      id: 7,
      title: "Elegy for Dunkirk",
      artist: "Alexandre Desplat",
      album: "Dunkirk (Original Motion Picture Soundtrack)",
      duration: "06:25",
      durationSec: 385,
      format: "FLAC",
      coverColor: "from-sky-700 to-zinc-950",
      isFavorite: false,
    },
    {
      id: 8,
      title: "Holocene",
      artist: "Bon Iver",
      album: "Bon Iver",
      duration: "05:36",
      durationSec: 336,
      format: "FLAC",
      coverColor: "from-lime-600 to-stone-950",
      isFavorite: false,
    },
    {
      id: 9,
      title: "Hoppípolla",
      artist: "Sigur Rós",
      album: "Takk...",
      duration: "04:28",
      durationSec: 268,
      format: "FLAC",
      coverColor: "from-rose-600 to-zinc-900",
      isFavorite: false,
    },
    {
      id: 10,
      title: "Time",
      artist: "Hans Zimmer",
      album: "Inception (Music From The Motion Picture)",
      duration: "04:35",
      durationSec: 275,
      format: "FLAC",
      coverColor: "from-blue-950 via-slate-800 to-zinc-955",
      isFavorite: false,
    },
    {
      id: 11,
      title: "Comptine d'un autre été: L'après-midi",
      artist: "Yann Tiersen",
      album: "Amélie (Original Soundtrack)",
      duration: "02:20",
      durationSec: 140,
      format: "FLAC",
      coverColor: "from-yellow-600 to-stone-900",
      isFavorite: false,
    },
    {
      id: 12,
      title: "To Build a Home",
      artist: "The Cinematic Orchestra",
      album: "Ma Fleur",
      duration: "06:07",
      durationSec: 367,
      format: "FLAC",
      coverColor: "from-emerald-700 to-zinc-950",
      isFavorite: false,
    },
  ]);

  // 计算属性 (Getters)
  const currentTrack = computed(() => {
    return tracks.value[currentTrackIndex.value];
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

  function togglePlay() {
    isPlaying.value = !isPlaying.value;
  }

  function playTrack(id: number) {
    const idx = tracks.value.findIndex((t) => t.id === id);
    if (idx !== -1) {
      currentTrackIndex.value = idx;
      isPlaying.value = true;
      currentTime.value = 0;
    }
  }

  function nextTrack() {
    currentTrackIndex.value = (currentTrackIndex.value + 1) % tracks.value.length;
    currentTime.value = 0;
  }

  function prevTrack() {
    currentTrackIndex.value =
      (currentTrackIndex.value - 1 + tracks.value.length) % tracks.value.length;
    currentTime.value = 0;
  }

  // 来源管理 actions
  function addSource(kind: 'local' | 'webdav', name: string, path: string, username?: string) {
    const newId = sources.value.length > 0 ? Math.max(...sources.value.map(s => s.id)) + 1 : 1;
    sources.value.push({
      id: newId,
      kind,
      name,
      path,
      isEnabled: true,
      lastScanned: "Just now",
      username,
    });
  }

  function removeSource(id: number) {
    sources.value = sources.value.filter(s => s.id !== id);
  }

  function toggleSource(id: number) {
    const source = sources.value.find(s => s.id === id);
    if (source) {
      source.isEnabled = !source.isEnabled;
    }
  }

  function scanSource(id: number) {
    const source = sources.value.find(s => s.id === id);
    if (source) {
      source.lastScanned = "Just now";
    }
  }

  return {
    isDarkMode,
    isPlaying,
    currentTime,
    volume,
    activeLibraryTab,
    activeSourceTab,
    activeRightTab,
    isRightPanelOpen,
    currentTrackIndex,
    activeAlbumId,
    activeArtistId,
    lyrics,
    playlists,
    sources,
    localSources,
    webdavSources,
    albums,
    artists,
    tracks,
    currentTrack,
    currentAlbumDetails,
    currentArtistDetails,
    formatTime,
    togglePlay,
    playTrack,
    nextTrack,
    prevTrack,
    addSource,
    removeSource,
    toggleSource,
    scanSource,
  };
});
