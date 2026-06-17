<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
import { onMounted, ref, computed, onUnmounted } from 'vue';
import type { Album } from '../../../stores/player';
import ArtworkImage from '../components/ArtworkImage.vue';
import { getArtworkUrl } from '../../../utils';
import { prefetchArtworks } from '../../../utils/artworkCache';

const playerStore = usePlayerStore();

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

// ============ 滚动容器与响应式布局 ============
// 列数根据主内容区宽度自适应；现代风格卡片间距更大（GRID_GAP=20），
// 以呈现更通透的留白感。
const scrollContainer = ref<HTMLElement | null>(null);
const columnCount = ref(4);
const rowHeight = ref(260);
const GRID_GAP = 20;

function recalcLayout() {
  const el = scrollContainer.value;
  if (!el) return;
  const w = el.clientWidth;
  let cols = 4;
  if (w < 460) cols = 2;
  else if (w < 760) cols = 3;
  else if (w < 1000) cols = 4;
  else cols = 5;
  columnCount.value = cols;

  // 行高估算：封面边长 + 信息块 (标题 22 + 艺人 16 + 间距 12 + mb-4 16) ≈ 66
  const coverSize = Math.max(100, (w - (cols - 1) * GRID_GAP) / cols);
  rowHeight.value = Math.round(coverSize + 66);
}

let resizeObserver: ResizeObserver | null = null;

// ============ 手写虚拟滚动 ============
const tick = ref(0);
const BUFFER_ROWS = 4;
const rowCount = computed(() => Math.ceil(playerStore.albums.length / columnCount.value));
const totalHeight = computed(() => rowCount.value * rowHeight.value);

interface VisibleItem { index: number; data: Album }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value;
  const el = scrollContainer.value;
  const cols = columnCount.value;
  const rh = rowHeight.value;
  const all = playerStore.albums;
  if (!el || all.length === 0) return [];

  const scrollTop = el.scrollTop;
  const viewH = el.clientHeight;
  const startRow = Math.max(0, Math.floor(scrollTop / rh) - BUFFER_ROWS);
  const visibleRows = Math.ceil(viewH / rh) + BUFFER_ROWS * 2;
  const endRow = startRow + visibleRows;

  const startIdx = startRow * cols;
  const endIdx = Math.min(all.length, endRow * cols);

  const out: VisibleItem[] = [];
  for (let i = startIdx; i < endIdx; i++) {
    out.push({ index: i, data: all[i] });
  }
  return out;
});

const offsetY = computed(() => {
  void tick.value;
  const el = scrollContainer.value;
  if (!el) return 0;
  return Math.max(0, Math.floor(el.scrollTop / rowHeight.value) - BUFFER_ROWS) * rowHeight.value;
});

// ============ 封面预加载 ============
const PREFETCH_ROWS = BUFFER_ROWS + 4;
const PREFETCH_ENABLED = true;
let lastPrefetchTs = 0;
const PREFETCH_THROTTLE_MS = 250;

function maybePrefetch() {
  if (!PREFETCH_ENABLED) return;
  const now = Date.now();
  if (now - lastPrefetchTs < PREFETCH_THROTTLE_MS) return;
  lastPrefetchTs = now;

  const el = scrollContainer.value;
  const all = playerStore.albums;
  if (!el || all.length === 0) return;
  const cols = columnCount.value;
  const rh = rowHeight.value;

  const curRow = Math.floor(el.scrollTop / rh);
  const startRow = Math.max(0, curRow - PREFETCH_ROWS);
  const visibleRows = Math.ceil(el.clientHeight / rh);
  const endRow = Math.min(rowCount.value, curRow + visibleRows + PREFETCH_ROWS);

  const startIdx = startRow * cols;
  const endIdx = Math.min(all.length, endRow * cols);

  const ids: Array<number | null | undefined> = [];
  for (let i = startIdx; i < endIdx; i++) {
    ids.push(all[i]?.cover_artwork_id);
  }
  prefetchArtworks(ids, (id) => getArtworkUrl(id));
}

let ticking = false;
function onScroll(e: Event) {
  tick.value++;
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 600) {
    playerStore.fetchAlbums();
  }
  maybePrefetch();
}

onMounted(() => {
  recalcLayout();
  if (typeof ResizeObserver !== 'undefined' && scrollContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      recalcLayout();
      tick.value++;
    });
    resizeObserver.observe(scrollContainer.value);
  }
  if (playerStore.albums.length === 0) {
    playerStore.fetchAlbums(true).then(() => {
      maybePrefetch();
    });
  } else {
    maybePrefetch();
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- 加载中 -->
    <div v-if="playerStore.isLoadingAlbums && playerStore.albums.length === 0" class="flex-1 flex flex-col items-center justify-center text-text-muted">
      <div class="w-8 h-8 border-2 border-accent/20 border-t-accent rounded-full animate-spin mb-4"></div>
      <span class="text-xs tracking-[0.2em] uppercase">Loading Albums…</span>
    </div>

    <!-- 加载出错 -->
    <div v-else-if="playerStore.isErrorAlbums" class="flex-1 flex flex-col items-center justify-center text-[#d25050]">
      <span class="text-sm font-medium">加载专辑失败，请稍后重试</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="playerStore.albums.length === 0" class="flex-1 flex flex-col items-center justify-center px-8">
      <div class="w-20 h-20 rounded-2xl bg-bg-active flex items-center justify-center mb-6">
        <span class="text-3xl text-text-muted">♪</span>
      </div>
      <p class="text-lg font-semibold text-text-main mb-2">暂无专辑</p>
      <p class="text-xs text-text-muted tracking-wide max-w-sm text-center leading-relaxed">
        未检测到专辑信息。请确保本地音乐目录中含有音频文件并已完成扫描。
      </p>
    </div>

    <!-- 正常渲染：虚拟滚动卡片网格 -->
    <div
      v-else
      ref="scrollContainer"
      class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-1"
      @scroll="onScroll"
    >
      <div :style="{ height: totalHeight + 'px', position: 'relative' }">
        <div
          class="absolute top-0 left-0 right-0 will-change-transform"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <div
            class="grid pb-10 pt-1"
            :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`, gap: GRID_GAP + 'px' }"
          >
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @click="goToAlbum(item.data.id)"
              class="group cursor-pointer flex flex-col"
            >
              <!-- 圆角封面卡片 + 悬浮播放按钮 -->
              <div class="relative aspect-square w-full mb-3 overflow-hidden rounded-2xl bg-bg-panel shadow-sm group-hover:shadow-xl transition-all duration-300">
                <ArtworkImage
                  :artwork-id="item.data.cover_artwork_id"
                  :fallback-color="item.data.coverColor"
                  img-class="group-hover:scale-110 transition-transform duration-500 ease-out"
                />
                <!-- 渐变蒙层增强悬浮感 -->
                <div class="absolute inset-0 bg-gradient-to-t from-black/30 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none rounded-2xl"></div>

                <!-- 悬浮播放按钮（现代风格：橙色实心圆） -->
                <div class="absolute right-3 bottom-3 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-300">
                  <div class="w-11 h-11 rounded-full bg-accent shadow-lg flex items-center justify-center">
                    <svg class="w-4 h-4 text-white ml-0.5" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
                  </div>
                </div>
              </div>

              <!-- 专辑信息（无衬线、克制排版） -->
              <div class="flex flex-col gap-1 px-1">
                <h3 class="font-semibold text-[14px] text-text-main truncate leading-tight group-hover:text-accent transition-colors">{{ item.data.title }}</h3>
                <div class="flex items-center justify-between gap-2">
                  <p class="text-[12px] text-text-muted truncate">{{ item.data.artist_name || 'Unknown Artist' }}</p>
                  <span class="text-[10px] font-mono text-text-muted shrink-0">{{ item.data.track_count }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: var(--border-color); }
</style>
