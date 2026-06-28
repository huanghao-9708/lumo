<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { Heart, AudioLines, Play, Plus } from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';

const playerStore = usePlayerStore();

const activeMenuTrackId = ref<number | null>(null);

const openPlaylistMenu = (trackId: number) => {
  if (activeMenuTrackId.value === trackId) {
    activeMenuTrackId.value = null;
  } else {
    activeMenuTrackId.value = trackId;
  }
};

const addToPlaylist = (playlistId: number, trackId: number) => {
  playerStore.addToPlaylist(playlistId, trackId);
  activeMenuTrackId.value = null;
};

const closeMenu = () => {
  activeMenuTrackId.value = null;
};

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

const activeTab = ref<'tracks' | 'albums'>('tracks');

// 艺人详情页专辑分页（5列×3行=15张/页，与专辑页保持一致）
const albumColumnCount = 5;
const artistAlbumsCurrentPage = computed(() => playerStore.currentArtistDetails?.albumsCurrentPage || 1);
const artistAlbumsTotalPages = computed(() => playerStore.currentArtistDetails?.albumsTotalPages || 1);
const artistAlbumsTotalCount = computed(() => playerStore.currentArtistDetails?.albumsTotalCount || 0);
const canPrevArtistAlbum = computed(() => artistAlbumsCurrentPage.value > 1);
const canNextArtistAlbum = computed(() => artistAlbumsCurrentPage.value < artistAlbumsTotalPages.value);

// 页码下拉框
const artistPageSelectOpen = ref(false);
const artistPageOptions = computed(() => {
  const total = artistAlbumsTotalPages.value;
  const cur = artistAlbumsCurrentPage.value;
  const set = new Set<number>([1, total]);
  for (let p = cur - 5; p <= cur + 5; p++) {
    if (p >= 1 && p <= total) set.add(p);
  }
  return Array.from(set).sort((a, b) => a - b);
});

// 跳转输入框
const artistJumpInput = ref('');
function handleArtistJumpInput() {
  const p = parseInt(artistJumpInput.value, 10);
  if (!isNaN(p) && p >= 1 && p <= artistAlbumsTotalPages.value) {
    playerStore.goToArtistAlbumsPage(p);
  }
  artistJumpInput.value = '';
}

function handleArtistAlbumPrev() { if (canPrevArtistAlbum.value) playerStore.prevArtistAlbumsPage(); }
function handleArtistAlbumNext() { if (canNextArtistAlbum.value) playerStore.nextArtistAlbumsPage(); }
function handleArtistAlbumJump(page: number) {
  playerStore.goToArtistAlbumsPage(page);
  artistPageSelectOpen.value = false;
}
function toggleArtistPageSelect() { artistPageSelectOpen.value = !artistPageSelectOpen.value; }

function onDocumentClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (!target.closest('.artist-page-select-wrap')) artistPageSelectOpen.value = false;
}
watch(artistPageSelectOpen, (open) => {
  if (open) document.addEventListener('click', onDocumentClick);
  else document.removeEventListener('click', onDocumentClick);
});

// Tracks tab 保留原滚动加载逻辑
const handleScroll = (e: Event) => {
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 200) {
    if (activeTab.value === 'tracks') {
      playerStore.fetchArtistTracks(playerStore.currentArtistDetails.id, true);
    }
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 relative z-10">
    <div v-if="playerStore.currentArtistDetails" class="flex-1 flex flex-col h-full overflow-hidden">
      <!-- 艺人头 (巨型 Typography) -->
      <div class="flex flex-col mb-20 shrink-0 relative">
        <h2 class="text-[10px] font-bold tracking-[0.3em] text-text-muted mb-8 uppercase z-10">Artist</h2>
        
        <div class="relative z-10">
          <h1 class="font-serif italic text-[100px] leading-[0.85] tracking-tight text-accent mb-10 break-words mix-blend-multiply">
            {{ playerStore.currentArtistDetails.name }}
          </h1>
        </div>

        <div class="flex items-center gap-6 mt-4 z-10 border-t border-black pt-6 max-w-md">
          <div class="flex items-center gap-8">
            <button 
              @click="activeTab = 'tracks'"
              class="text-[12px] tracking-[0.1em] uppercase transition-all duration-300"
              :class="activeTab === 'tracks' ? 'text-accent font-bold border-b-2 border-black pb-1' : 'text-text-muted  font-medium hover:text-accent '"
            >
              TRACKS 
              <span class="text-[10px] ml-1">{{ playerStore.currentArtistDetails.stats?.track_count || 0 }}</span>
            </button>
            
            <button 
              @click="activeTab = 'albums'"
              class="text-[12px] tracking-[0.1em] uppercase transition-all duration-300"
              :class="activeTab === 'albums' ? 'text-accent font-bold border-b-2 border-black pb-1' : 'text-text-muted  font-medium hover:text-accent '"
            >
              ALBUMS 
              <span class="text-[10px] ml-1">{{ playerStore.currentArtistDetails.stats?.album_count || 0 }}</span>
            </button>
          </div>
        </div>

        <!-- 装饰性背景文字，错位放大 -->
        <div class="absolute -top-10 -right-10 pointer-events-none select-none overflow-hidden w-full h-full flex justify-end opacity-[0.03]">
          <h1 class="font-serif italic text-[200px] leading-none whitespace-nowrap">
            {{ playerStore.currentArtistDetails.name }}
          </h1>
        </div>
      </div>

      <!-- 下部分：滚动区域 -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-4 pb-10" @scroll="handleScroll">
        
        <!-- 歌曲列表 -->
        <section v-if="activeTab === 'tracks'">
          <div class="flex items-center justify-between mb-4 border-b border-[#e8e6df] pb-2">
            <h3 class="text-[10px] font-bold tracking-[0.2em] text-text-muted uppercase">热门曲目</h3>
            <button 
              v-if="playerStore.currentArtistDetails.tracks.length > 0"
              @click="playerStore.playQueue(playerStore.currentArtistDetails.tracks, 0)" 
              class="flex items-center gap-2 bg-black text-white px-4 py-1.5 hover:bg-black/80 transition-all group rounded-sm shadow-sm"
            >
              <Play class="w-3 h-3 fill-current text-white" />
              <span class="text-[10px] font-bold tracking-[0.2em] uppercase">播放全部</span>
            </button>
          </div>
          <div class="flex flex-col">
            <div 
              v-for="(song, index) in playerStore.currentArtistDetails.tracks" 
              :key="song.id"
              @dblclick="playerStore.playQueue(playerStore.currentArtistDetails.tracks, index)"
              class="flex items-center text-[13px] py-4 border-b border-[#f0eee6]/60 group transition-colors duration-200 cursor-pointer hover:bg-[#faf9f5]"
            >
              <div class="w-12 text-left text-text-muted font-medium relative">
                <template v-if="playerStore.currentTrack?.id === song.id && playerStore.isPlaying">
                  <AudioLines class="w-4 h-4 stroke-[1.5] text-accent animate-pulse" />
                </template>
                <template v-else>
                  <span class="group-hover:opacity-0 transition-opacity duration-200">{{ String(index + 1).padStart(2, '0') }}</span>
                  <Play class="w-3.5 h-3.5 absolute top-1/2 left-0 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-accent fill-current" />
                </template>
              </div>
              <div class="flex-[3] flex items-center gap-4">
                <Heart 
                  class="w-3.5 h-3.5 transition-colors stroke-[1.5]" 
                  :class="[
                    song.isFavorite ? 'text-accent fill-current' : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent '
                  ]"
                  @click.stop="playerStore.toggleFavorite(song.id)"
                />
                <div class="relative flex items-center">
                  <button @click.stop="openPlaylistMenu(song.id)" class="text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent  transition-opacity" title="添加到歌单">
                    <Plus class="w-3.5 h-3.5 stroke-[1.5]" />
                  </button>
                  <div v-if="activeMenuTrackId === song.id" class="absolute left-6 top-0 bg-bg-base border border-[#e8e6df] shadow-sm z-50 py-1 min-w-[120px] rounded-sm">
                    <div v-if="playerStore.playlists.length === 0" class="px-3 py-1.5 text-xs text-text-muted whitespace-nowrap">暂无自建歌单</div>
                    <button 
                      v-for="pl in playerStore.playlists" 
                      :key="pl.id"
                      @click.stop="addToPlaylist(pl.id, song.id)"
                      class="block w-full text-left px-3 py-1.5 text-[11px] font-medium text-[#555] hover:text-accent  hover:bg-black/5 transition-colors whitespace-nowrap truncate tracking-wider"
                    >
                      {{ pl.name }}
                    </button>
                  </div>
                </div>
                <span 
                  class="truncate" 
                  :class="playerStore.currentTrack?.id === song.id ? 'font-serif italic font-semibold text-[16px] text-accent' : 'text-text-main  font-medium'"
                >{{ song.title }}</span>
              </div>
              <div class="flex-[2] truncate pr-4 text-text-muted italic">{{ song.album }}</div>
              <div class="w-16 text-right pr-4 text-text-muted ">{{ song.duration }}</div>
            </div>
          </div>
        </section>

        <!-- 专辑墙（分页：5列×3行=15张/页） -->
        <section v-if="activeTab === 'albums'">
          <h3 class="text-[10px] font-bold tracking-[0.2em] text-text-muted mb-6 uppercase border-b border-[#e8e6df] pb-2">所有专辑</h3>
          <div
            class="grid gap-4"
            :style="{ gridTemplateColumns: `repeat(${albumColumnCount}, minmax(0, 1fr))` }"
          >
            <div
              v-for="album in playerStore.currentArtistDetails.albums"
              :key="album.id"
              @click="goToAlbum(album.id)"
              class="group cursor-pointer flex flex-col"
            >
              <div class="relative aspect-square w-full mb-2 overflow-hidden bg-[#e8e6df] shadow-sm">
                <img
                  v-if="album.cover_thumb"
                  :src="album.cover_thumb"
                  loading="lazy"
                  decoding="async"
                  class="absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition-transform duration-700 ease-out"
                />
                <div
                  v-else
                  class="absolute inset-0 bg-gradient-to-br opacity-80 from-gray-400 to-gray-600 group-hover:scale-105 transition-transform duration-700 ease-out"
                ></div>
                <div class="absolute inset-0 bg-black/0 group-hover:bg-black/5 transition-colors duration-300"></div>
              </div>
              <div class="flex flex-col gap-0.5">
                <h4 class="font-serif italic font-semibold text-sm text-accent truncate">{{ album.title }}</h4>
                <span class="text-[10px] tracking-widest text-text-muted">{{ album.year }}</span>
              </div>
            </div>
          </div>

          <!-- 分页导航栏 -->
          <div class="flex items-center justify-center gap-4 py-3 mt-4 border-t border-[#e8e6df] relative">
            <button
              @click="handleArtistAlbumPrev"
              :disabled="!canPrevArtistAlbum"
              class="text-[11px] tracking-widest text-text-muted hover:text-accent transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
            >←</button>

            <div class="artist-page-select-wrap relative">
              <button
                @click="toggleArtistPageSelect"
                class="font-serif italic text-sm text-accent hover:underline flex items-center gap-1"
              >
                {{ artistAlbumsCurrentPage }}
                <span class="text-text-muted mx-0.5">/</span>
                <span class="text-text-muted">{{ artistAlbumsTotalPages }}</span>
                <span class="text-[9px] text-text-muted ml-1">▾</span>
              </button>
              <div
                v-if="artistPageSelectOpen"
                class="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 bg-[#fdfcf9] border border-[#dcdad1] shadow-lg py-1 min-w-[80px] max-h-[240px] overflow-y-auto z-30"
              >
                <button
                  v-for="p in artistPageOptions"
                  :key="p"
                  @click="handleArtistAlbumJump(p)"
                  class="block w-full text-center py-1.5 text-xs hover:bg-[#eae8e1] transition-colors"
                  :class="p === artistAlbumsCurrentPage ? 'font-serif italic text-accent font-bold' : 'text-text-muted'"
                >{{ p }}</button>
              </div>
            </div>

            <button
              @click="handleArtistAlbumNext"
              :disabled="!canNextArtistAlbum"
              class="text-[11px] tracking-widest text-text-muted hover:text-accent transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
            >→</button>

            <span class="text-[#dcdad1] mx-1">|</span>
            <span class="text-[10px] tracking-widest text-text-muted">共 {{ artistAlbumsTotalCount }} 张</span>
            <span class="text-[#dcdad1] mx-1">|</span>

            <div class="flex items-center gap-1">
              <span class="text-[10px] tracking-widest text-text-muted">跳至</span>
              <input
                v-model="artistJumpInput"
                @keyup.enter="handleArtistJumpInput"
                type="number"
                min="1"
                :max="artistAlbumsTotalPages"
                class="w-12 px-1 py-0.5 text-xs text-center bg-transparent border border-[#dcdad1] focus:border-accent focus:outline-none text-text-muted"
                placeholder="页"
              />
              <span class="text-[10px] tracking-widest text-text-muted">页</span>
            </div>
          </div>

          <div v-if="playerStore.currentArtistDetails.isLoadingAlbums" class="text-center text-text-muted py-4 text-xs tracking-widest uppercase">
            Loading...
          </div>
        </section>

        <!-- 加载动画给Tracks -->
        <div v-if="activeTab === 'tracks' && playerStore.currentArtistDetails.isLoadingTracks" class="text-center text-text-muted  py-8 text-xs tracking-widest uppercase">
          Loading...
        </div>
      </div>
    </div>
    <div v-else class="flex-1 flex items-center justify-center text-text-muted ">
      未找到艺人信息
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
