import { computed, ref, onMounted, onUnmounted } from 'vue';
import { usePlayerStore } from '../stores/player';

export function usePlayerControls() {
  const playerStore = usePlayerStore();

  // 1. 播放进度百分比
  const progressPercent = computed(() => {
    const total = playerStore.durationMs;
    if (!total) return 0;
    return Math.min(100, Math.max(0, (playerStore.progressMs / total) * 100));
  });

  // 2. 进度条点击拖拽计算
  const handleProgressClick = (e: MouseEvent) => {
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const percent = (e.clientX - rect.left) / rect.width;
    if (playerStore.durationMs > 0) {
      playerStore.seek(Math.floor(percent * playerStore.durationMs));
    }
  };

  // 3. 音量条点击拖拽计算
  const handleVolumeClick = (e: MouseEvent) => {
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    playerStore.setVolume(Math.floor(percent * 100));
  };

  // 4. 时间格式化
  const formatTimeMs = (ms: number) => {
    const seconds = Math.floor(ms / 1000);
    const min = Math.floor(seconds / 60);
    const sec = seconds % 60;
    return `${min.toString().padStart(2, '0')}:${sec.toString().padStart(2, '0')}`;
  };

  // 5. 播放模式切换循环
  const cyclePlayMode = () => {
    const modes: ('normal' | 'repeat' | 'repeat-one' | 'shuffle')[] = ['normal', 'repeat', 'repeat-one', 'shuffle'];
    const currentIdx = modes.indexOf(playerStore.playMode as any);
    const nextIdx = (currentIdx + 1) % modes.length;
    playerStore.playMode = modes[nextIdx];
  };

  // 6. 歌单菜单管理
  const isPlaylistMenuOpen = ref(false);

  const togglePlaylistMenu = (e: Event) => {
    e.stopPropagation();
    isPlaylistMenuOpen.value = !isPlaylistMenuOpen.value;
  };

  const closePlaylistMenu = () => {
    isPlaylistMenuOpen.value = false;
  };

  const addToPlaylist = (playlistId: number) => {
    const trackId = playerStore.currentTrack?.id;
    if (trackId) {
      playerStore.addToPlaylist(playlistId, trackId);
    }
    isPlaylistMenuOpen.value = false;
  };

  onMounted(() => {
    window.addEventListener('click', closePlaylistMenu);
  });

  onUnmounted(() => {
    window.removeEventListener('click', closePlaylistMenu);
  });

  return {
    playerStore,
    progressPercent,
    handleProgressClick,
    handleVolumeClick,
    formatTimeMs,
    cyclePlayMode,
    isPlaylistMenuOpen,
    togglePlaylistMenu,
    closePlaylistMenu,
    addToPlaylist
  };
}
