<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import {
  Search, Play, List, LayoutGrid, MoreHorizontal, Heart, Loader2, Music,
} from 'lucide-vue-next';
import { usePlayerStore, type Album } from '../../stores/player';
import { useVirtualList } from '../../composables/useVirtualList';
import AlbumGrid from '../content/AlbumGrid.vue';
import AlbumDetail from '../content/AlbumDetail.vue';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();

/* ============ 视图状态 ============ */
const viewMode = ref<'list' | 'grid'>('list');

/* ============ 搜索 ============ */
const searchInput = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;
function onSearchInput() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    playerStore.searchQuery = searchInput.value;
    // 根据当前 tab 拉取对应数据
    loadForCurrentTab();
  }, 250);
}
onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); });

/* ============ 标题 & 元信息 ============ */
const pageTitle = computed(() => {
  switch (playerStore.activeLibraryTab) {
    case '最近播放': return '最近播放';
    case '收藏': return '我喜欢的音乐';
    case '专辑': return '专辑';
    case '艺术家': return '艺术家';
    default: return '全部歌曲';
  }
});

const trackCount = computed(() => {
  if (playerStore.activeLibraryTab === '专辑') return playerStore.albumsTotalCount;
  return playerStore.tracks.length;
});
const metaText = computed(() => {
  const n = trackCount.value;
  if (playerStore.activeLibraryTab === '专辑') return `${n.toLocaleString()} 张专辑`;
  return `${n.toLocaleString()} 首歌曲`;
});

/* ============ 视图分支判定 ============ */

// 专辑网格视图（无 activeAlbumId 时）
const isAlbumGridView = computed(() => {
  return playerStore.activeLibraryTab === '专辑' && !playerStore.activeAlbumId;
});

// 专辑详情视图（有 activeAlbumId 时）
const isAlbumDetailView = computed(() => {
  return playerStore.activeLibraryTab === '专辑' && !!playerStore.activeAlbumId;
});

// 轨道表格视图
const isTracksView = computed(() => {
  return ['全部歌曲', '最近播放', '收藏', '播放列表'].includes(playerStore.activeLibraryTab)
    || playerStore.activePlaylistId !== null;
});

/* ============ 轨道列表相关 ============ */
function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playSong(index: number) {
  playerStore.playTrack(index);
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}

/* ============ 虚拟列表 ============ */
const ROW_HEIGHT = 40;
const scrollContainer = ref<HTMLElement | null>(null);
const { totalHeight, offsetY, visibleItems } = useVirtualList({
  containerRef: scrollContainer,
  items: computed(() => playerStore.tracks) as any,
  itemHeight: ROW_HEIGHT,
  buffer: 8,
});

/* ============ 无限加载更多 ============ */
function onListScroll() {
  const el = scrollContainer.value;
  if (!el) return;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 200) {
    if (playerStore.hasMoreTracks && !playerStore.isLoadingTracks) {
      playerStore.fetchTracks();
    }
  }
}

/* ============ 专辑选择 / 返回 ============ */
function onAlbumSelect(album: Album) {
  playerStore.activeAlbumId = album.id;
}

/* ============ Tab 切换时重新拉取数据 ============ */
function loadForCurrentTab() {
  const tab = playerStore.activeLibraryTab;
  if (tab === '最近播放') playerStore.fetchRecentlyPlayed();
  else if (tab === '收藏') playerStore.fetchFavoriteTracks();
  else if (tab === '专辑') playerStore.fetchAlbums(true);
  else playerStore.fetchTracks(true);
}
watch(() => playerStore.activeLibraryTab, loadForCurrentTab);

onMounted(() => {
  // 仅在还没有数据时首次拉取，避免覆盖 restoreSession 的状态
  if (playerStore.tracks.length === 0 && playerStore.albums.length === 0) {
    loadForCurrentTab();
  }
});
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- ============ 专辑详情视图（独占整个 Content Area） ============ -->
    <template v-if="isAlbumDetailView">
      <AlbumDetail
        :album-id="playerStore.activeAlbumId"
      />
    </template>

    <!-- ============ 其他视图（共享 Header + Toolbar） ============ -->
    <template v-else>

      <!-- Header -->
      <div class="px-8 pt-6 pb-0 flex-shrink-0" data-tauri-drag-region>
        <div class="flex items-end justify-between mb-2">
          <div>
            <!-- LDL Page Title = 42px -->
            <h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none mb-2">{{ pageTitle }}</h1>
            <p class="text-[12px] text-text-muted leading-relaxed font-mono">{{ metaText }}</p>
          </div>
          <div class="relative w-[240px]">
            <Search class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              v-model="searchInput"
              @input="onSearchInput"
              type="text"
              placeholder="搜索歌曲、艺术家、专辑…"
              class="w-full h-[32px] pl-8 pr-3 text-[12px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:border-brand-orange/50"
            />
          </div>
        </div>
      </div>

      <!-- Page Toolbar -->
      <div class="px-8 py-3 flex items-center justify-end flex-shrink-0">
        <!-- 视图切换（仅轨道视图有用） -->
        <div v-if="isTracksView" class="flex items-center gap-0 bg-bg-canvas border border-border-color rounded-[8px] p-[2px]">
          <button
            class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
            :class="viewMode === 'list' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
            @click="viewMode = 'list'"
            title="列表视图"
          >
            <List class="w-[14px] h-[14px]" />
          </button>
          <button
            class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
            :class="viewMode === 'grid' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
            @click="viewMode = 'grid'"
            title="网格视图"
          >
            <LayoutGrid class="w-[14px] h-[14px]" />
          </button>
        </div>
      </div>

      <!-- ============ 专辑网格视图 ============ -->
      <AlbumGrid
        v-if="isAlbumGridView"
        @select="onAlbumSelect"
      />

      <!-- ============ 轨道表格视图 ============ -->
      <template v-else-if="isTracksView">
        <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8" @scroll="onListScroll">

          <!-- 表头（sticky） -->
          <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
            <div class="w-10 text-center shrink-0">#</div>
            <div class="w-8 shrink-0"></div>
            <div class="flex-[2] min-w-0 pl-1">标题</div>
            <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
            <div class="flex-[1.5] min-w-0 hidden md:block">专辑</div>
            <div class="w-[56px] text-right shrink-0 hidden lg:block">时长</div>
            <div class="w-[50px] text-center shrink-0 hidden lg:block">格式</div>
            <div class="w-8 shrink-0"></div>
          </div>

          <!-- 加载态（首次） -->
          <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
            <span class="text-[12px]">加载中…</span>
          </div>

          <!-- 空态 -->
          <div v-else-if="playerStore.tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <Music class="w-8 h-8 text-text-disabled" />
            <span class="text-[12px]">没有找到歌曲</span>
          </div>

          <!-- 错误态 -->
          <div v-else-if="playerStore.isErrorTracks" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
            <span class="text-[12px]">加载失败，请稍后重试</span>
          </div>

          <!-- 虚拟列表 -->
          <div v-else :style="{ height: totalHeight + 'px', position: 'relative' }">
            <div :style="{ transform: `translateY(${offsetY}px)` }">
              <div
                v-for="{ index, data: song } in visibleItems"
                :key="song.id"
                class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
                :style="{ height: ROW_HEIGHT + 'px' }"
                :class="{
                  'playing-row bg-list-selected': isPlayingTrack(song.id),
                }"
                @dblclick="playSong(index)"
              >
                <!-- 序号 / 播放图标 -->
                <div class="w-10 text-center shrink-0 text-[12px] font-mono">
                  <span v-if="isPlayingTrack(song.id)" class="text-brand-orange inline-flex items-center justify-center">
                    <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
                    <Play v-else class="w-[12px] h-[12px] fill-current" />
                  </span>
                  <template v-else>
                    <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
                    <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
                  </template>
                </div>

                <!-- 收藏 -->
                <div class="w-8 shrink-0 flex items-center justify-center">
                  <Heart
                    v-if="song.isFavorite"
                    class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                    @click="toggleFav(song.id, $event)"
                  />
                  <Heart
                    v-else
                    class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                    @click="toggleFav(song.id, $event)"
                  />
                </div>

                <!-- 标题 -->
                <div class="flex-[2] min-w-0 pl-1">
                  <span class="text-[13px] truncate block" :class="isPlayingTrack(song.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
                    {{ song.title }}
                  </span>
                </div>

                <!-- 艺术家 -->
                <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ song.artist }}</div>

                <!-- 专辑（非斜体） -->
                <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate">{{ song.album }}</div>

                <!-- 时长 -->
                <div class="w-[56px] text-right shrink-0 hidden lg:block text-[12px] font-mono text-text-muted tabular-nums">{{ song.duration }}</div>

                <!-- 格式 -->
                <div class="w-[50px] text-center shrink-0 hidden lg:block">
                  <span class="text-[10px] font-mono text-text-muted uppercase">{{ song.format }}</span>
                </div>

                <!-- more -->
                <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                  <MoreHorizontal class="w-4 h-4 text-text-muted" />
                </div>
              </div>
            </div>
          </div>

          <!-- 增量加载指示 -->
          <div v-if="playerStore.isLoadingTracks && playerStore.tracks.length > 0" class="flex items-center justify-center py-4 text-text-muted">
            <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
            <span class="text-[11px]">加载更多…</span>
          </div>

          <!-- Footer Status -->
          <FooterStatus v-if="playerStore.tracks.length > 0" :count="`${trackCount.toLocaleString()} 首歌曲`" />

        </div>
      </template>

      <!-- ============ 占位：艺术家等未实现视图 ============ -->
      <div v-else class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted px-8">
        <LayoutGrid class="w-8 h-8 text-text-disabled" />
        <p class="text-[13px]">{{ pageTitle }}视图</p>
        <p class="text-[11px] text-text-muted/70">该页面的视图将在后续迭代中接入。</p>
      </div>

    </template>
  </div>
</template>
