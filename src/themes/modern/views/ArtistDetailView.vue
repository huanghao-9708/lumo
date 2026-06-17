<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { Heart, AudioLines, Play, Plus, ArrowLeft } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import ArtworkImage from '../components/ArtworkImage.vue';

const playerStore = usePlayerStore();

const activeMenuTrackId = ref<number | null>(null);

const openPlaylistMenu = (trackId: number) => {
  activeMenuTrackId.value = activeMenuTrackId.value === trackId ? null : trackId;
};

const addToPlaylist = (playlistId: number, trackId: number) => {
  playerStore.addToPlaylist(playlistId, trackId);
  activeMenuTrackId.value = null;
};

const closeMenu = () => { activeMenuTrackId.value = null; };

onMounted(() => window.addEventListener('click', closeMenu));
onUnmounted(() => window.removeEventListener('click', closeMenu));

const goBack = () => playerStore.goBack();
const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

const activeTab = ref<'tracks' | 'albums'>('tracks');

const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 200) {
    if (activeTab.value === 'tracks') {
      playerStore.fetchArtistTracks(playerStore.currentArtistDetails.id, true);
    } else {
      playerStore.fetchArtistAlbums(playerStore.currentArtistDetails.id, true);
    }
  }
};

// 艺人名首字符（大头像占位字母）
const initialOf = (name: string) => (name?.trim()?.[0] || '?').toUpperCase();

const albumColumnCount = computed(() => {
  // 简化：依据已加载专辑数量与容器宽度选列数，由 grid 的 minmax 兜底
  return 'grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5';
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentArtistDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 返回按钮 -->
      <button
        @click="goBack"
        class="flex items-center gap-2 text-text-muted hover:text-text-main transition-colors mb-6 w-fit text-[12px] tracking-wide"
      >
        <ArrowLeft class="w-4 h-4 stroke-[1.5]" />
        <span>返回</span>
      </button>

      <!-- 艺人头：大圆形头像 + 现代排版 -->
      <div class="flex flex-col md:flex-row md:items-center gap-8 mb-8 shrink-0">
        <div class="relative shrink-0">
          <div
            class="w-40 h-40 rounded-full bg-gradient-to-br flex items-center justify-center shadow-2xl overflow-hidden"
            :class="playerStore.currentArtistDetails.avatarColor"
          >
            <span class="text-6xl font-bold text-white/90 select-none">{{ initialOf(playerStore.currentArtistDetails.name) }}</span>
          </div>
        </div>
        <div class="flex flex-col min-w-0">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-[11px] font-bold tracking-[0.25em] text-text-muted uppercase">Artist</span>
            <span class="w-1 h-1 rounded-full bg-accent"></span>
            <span class="text-[11px] font-bold tracking-[0.25em] text-accent uppercase">Verified</span>
          </div>
          <h1 class="font-bold text-5xl md:text-6xl text-text-main mb-3 leading-tight truncate">{{ playerStore.currentArtistDetails.name }}</h1>
          <p class="text-[13px] text-text-muted">
            {{ playerStore.currentArtistDetails.stats?.track_count || 0 }} 首歌曲
            <span class="mx-2">·</span>
            {{ playerStore.currentArtistDetails.stats?.album_count || 0 }} 张专辑
          </p>
        </div>
      </div>

      <!-- Tab 切换 -->
      <div class="flex items-center gap-2 mb-4 shrink-0 border-b border-border-color">
        <button
          @click="activeTab = 'tracks'"
          class="px-4 py-2.5 text-[13px] font-semibold transition-all relative"
          :class="activeTab === 'tracks' ? 'text-text-main' : 'text-text-muted hover:text-text-main'"
        >
          歌曲
          <span class="ml-1 text-[10px] text-text-muted font-normal">{{ playerStore.currentArtistDetails.stats?.track_count || 0 }}</span>
          <div v-if="activeTab === 'tracks'" class="absolute bottom-0 left-0 right-0 h-0.5 bg-accent rounded-full"></div>
        </button>
        <button
          @click="activeTab = 'albums'"
          class="px-4 py-2.5 text-[13px] font-semibold transition-all relative"
          :class="activeTab === 'albums' ? 'text-text-main' : 'text-text-muted hover:text-text-main'"
        >
          专辑
          <span class="ml-1 text-[10px] text-text-muted font-normal">{{ playerStore.currentArtistDetails.stats?.album_count || 0 }}</span>
          <div v-if="activeTab === 'albums'" class="absolute bottom-0 left-0 right-0 h-0.5 bg-accent rounded-full"></div>
        </button>
      </div>

      <!-- 下部分：滚动区域 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-10" @scroll="handleScroll">

        <!-- 歌曲列表 -->
        <section v-if="activeTab === 'tracks'">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-[12px] font-bold tracking-wide text-text-muted uppercase">热门曲目</h3>
            <button
              v-if="playerStore.currentArtistDetails.tracks.length > 0"
              @click="playerStore.playQueue(playerStore.currentArtistDetails.tracks, 0)"
              class="flex items-center gap-2 bg-accent text-white px-5 py-2 rounded-full hover:scale-105 hover:shadow-lg active:scale-95 transition-all font-medium text-[12px] shadow-md"
            >
              <Play class="w-3.5 h-3.5 fill-current" />
              <span>播放全部</span>
            </button>
          </div>

          <!-- 表头 -->
          <div class="flex items-center px-4 py-2 text-[10px] font-bold tracking-[0.2em] text-text-muted uppercase border-b border-border-color">
            <span class="w-8 text-center">#</span>
            <span class="flex-[3] pl-4">标题</span>
            <span class="flex-[2] pl-4">专辑</span>
            <span class="w-16 text-right pr-2">时长</span>
            <span class="w-8"></span>
          </div>

          <div class="flex flex-col">
            <div
              v-for="(song, index) in playerStore.currentArtistDetails.tracks"
              :key="song.id"
              @dblclick="playerStore.playQueue(playerStore.currentArtistDetails.tracks, index)"
              class="flex items-center px-4 text-[13px] group transition-colors duration-150 cursor-pointer rounded-lg hover:bg-bg-active/50"
              :class="playerStore.currentTrack?.id === song.id ? 'bg-accent/10' : ''"
            >
              <div class="w-8 text-center text-text-muted font-medium relative shrink-0 py-3">
                <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 mx-auto stroke-[1.5] text-accent animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity">{{ index + 1 }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity text-accent fill-current" />
                </template>
              </div>
              <div class="flex-[3] pl-4 flex items-center min-w-0 py-3">
                <span
                  class="truncate"
                  :class="playerStore.currentTrack?.id === song.id ? 'text-accent font-semibold' : 'text-text-main font-medium'"
                >{{ song.title }}</span>
              </div>
              <div class="flex-[2] pl-4 truncate text-text-muted py-3">{{ song.album }}</div>
              <div class="w-16 text-right pr-2 text-text-muted font-mono text-[12px] py-3">{{ song.duration }}</div>
              <div class="w-8 flex items-center justify-end gap-2 py-3 shrink-0">
                <button
                  @click.stop="openPlaylistMenu(song.id)"
                  class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent transition-opacity relative"
                  title="添加到歌单"
                >
                  <Plus class="w-4 h-4 stroke-[1.5]" />
                </button>
                <Heart
                  class="w-4 h-4 transition-colors stroke-[1.5] cursor-pointer"
                  :class="song.isFavorite ? 'text-accent fill-current' : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent'"
                  @click.stop="playerStore.toggleFavorite(song.id)"
                />

                <!-- 添加到歌单菜单 -->
                <div v-if="activeMenuTrackId === song.id" class="absolute right-8 top-full mt-1 bg-bg-base border border-border-color shadow-xl z-50 py-1 min-w-[140px] rounded-lg overflow-hidden">
                  <div v-if="playerStore.playlists.length === 0" class="px-3 py-2 text-[11px] text-text-muted whitespace-nowrap">暂无自建歌单</div>
                  <button
                    v-for="pl in playerStore.playlists"
                    :key="pl.id"
                    @click.stop="addToPlaylist(pl.id, song.id)"
                    class="block w-full text-left px-3 py-1.5 text-[12px] text-text-main hover:bg-bg-active hover:text-accent transition-colors whitespace-nowrap truncate"
                  >
                    {{ pl.name }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div v-if="playerStore.currentArtistDetails.isLoadingTracks" class="text-center text-text-muted py-8">
            <div class="w-6 h-6 border-2 border-accent/20 border-t-accent rounded-full animate-spin mx-auto"></div>
          </div>
        </section>

        <!-- 专辑墙 -->
        <section v-if="activeTab === 'albums'">
          <h3 class="text-[12px] font-bold tracking-wide text-text-muted uppercase mb-4">所有专辑</h3>
          <div :class="['grid', 'gap-6', albumColumnCount]">
            <div
              v-for="album in playerStore.currentArtistDetails.albums"
              :key="album.id"
              @click="goToAlbum(album.id)"
              class="group cursor-pointer flex flex-col"
            >
              <div class="relative aspect-square w-full mb-3 overflow-hidden rounded-2xl bg-bg-panel shadow-sm group-hover:shadow-xl transition-all duration-300">
                <ArtworkImage
                  :artwork-id="album.cover_artwork_id"
                  :fallback-color="album.coverColor"
                  img-class="group-hover:scale-110 transition-transform duration-500 ease-out"
                />
                <div class="absolute inset-0 bg-gradient-to-t from-black/30 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none rounded-2xl"></div>
                <div class="absolute right-3 bottom-3 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-300">
                  <div class="w-10 h-10 rounded-full bg-accent shadow-lg flex items-center justify-center">
                    <svg class="w-4 h-4 text-white ml-0.5" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
                  </div>
                </div>
              </div>
              <div class="flex flex-col gap-1 px-1">
                <h4 class="font-semibold text-[13px] text-text-main truncate group-hover:text-accent transition-colors leading-tight">{{ album.title }}</h4>
                <span class="text-[11px] text-text-muted">{{ album.year }}</span>
              </div>
            </div>
          </div>

          <div v-if="playerStore.currentArtistDetails.isLoadingAlbums" class="text-center text-text-muted py-8">
            <div class="w-6 h-6 border-2 border-accent/20 border-t-accent rounded-full animate-spin mx-auto"></div>
          </div>
        </section>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-text-muted text-sm">
      未找到艺人信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: var(--border-color); }
</style>
