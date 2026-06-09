import { reactive } from "vue";

export interface Track {
  id: number;
  title: string;
  artist: string;
  album: string;
  duration: string;
  durationSec: number;
  format: string;
  coverColor: string; // 用渐变色模拟漂亮的封面
  isFavorite: boolean;
}

export interface Playlist {
  name: string;
  count: number;
}

export const store = reactive({
  // 双主题模式切换 (默认浅色模式 Light)
  isDarkMode: false,

  // 播放器播放状态
  isPlaying: false,
  currentTime: 102, // 对应 01:42
  volume: 75,
  
  // 选项卡状态
  activeLibraryTab: "全部歌曲",
  activeSourceTab: "本地音乐库",
  activeRightTab: "歌词" as "歌词" | "播放队列" | "文件信息",
  isRightPanelOpen: true,
  
  // 当前播放歌曲索引 (Experience 索引为 2)
  currentTrackIndex: 2,

  // 歌词数据
  lyrics: [
    { text: "We are going on a journey", time: 0 },
    { text: "A journey to experience", time: 15 },
    { text: "We are going on a journey", time: 30 },
    { text: "A journey to experience", time: 45, isActive: true }, // 当前句高亮
    { text: "We are going on a journey", time: 60 },
    { text: "A journey to experience", time: 75 },
    { text: "We are going on a journey", time: 90 },
    { text: "A journey to experience", time: 105 },
    { text: "Close your eyes", time: 120 },
    { text: "Let the music take you higher", time: 135 },
    { text: "Close your eyes", time: 150 },
    { text: "Let the music take you higher", time: 165 },
  ],

  // 歌单数据
  playlists: [
    { name: "日常音乐", count: 58 },
    { name: "工作专注", count: 24 },
    { name: "放松时刻", count: 36 },
    { name: "90s 精选", count: 57 },
  ] as Playlist[],

  // 歌曲数据列表 (高精度匹配模板图)
  tracks: [
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
      coverColor: "from-blue-950 via-slate-800 to-zinc-950",
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
  ] as Track[],

  // 获取当前正在播放歌曲的辅助属性
  get currentTrack(): Track {
    return this.tracks[this.currentTrackIndex];
  },

  // 格式化时间函数 (秒转 mm:ss)
  formatTime(seconds: number): string {
    const min = Math.floor(seconds / 60);
    const sec = Math.floor(seconds % 60);
    return `${min.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
  },

  // 播放控制
  togglePlay() {
    this.isPlaying = !this.isPlaying;
  },

  playTrack(id: number) {
    const idx = this.tracks.findIndex((t) => t.id === id);
    if (idx !== -1) {
      this.currentTrackIndex = idx;
      this.isPlaying = true;
      this.currentTime = 0;
    }
  },

  nextTrack() {
    this.currentTrackIndex = (this.currentTrackIndex + 1) % this.tracks.length;
    this.currentTime = 0;
  },

  prevTrack() {
    this.currentTrackIndex =
      (this.currentTrackIndex - 1 + this.tracks.length) % this.tracks.length;
    this.currentTime = 0;
  },
});
