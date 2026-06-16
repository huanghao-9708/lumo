<script setup lang="ts">
import { usePlayerStore } from '../../stores/player';
import { onMounted, ref, computed, onUnmounted } from 'vue';
import type { Album } from '../../stores/player';
import ArtworkImage from '../components/ArtworkImage.vue';
import { getArtworkUrl } from '../../utils';
import { prefetchArtworks } from '../../utils/artworkCache';

const playerStore = usePlayerStore();

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

// ============ 滚动容器与响应式布局 ============
// 不用 Tailwind lg:/xl: 断点的原因：虚拟列表需要确切的列数才能正确切片。
// 列数映射（已针对"默认布局 = 左侧栏 240 + 右侧栏 320 都打开"的实际主内容区宽度调整）：
//   主内容区 < 420px  → 2 列（极窄，几乎不会发生）
//   420 ~ 720px       → 3 列
//   >= 720px          → 4 列 ← 默认布局目标：保证一行有 4 个
// 在常见 1280~1440px 桌面窗口、双侧栏打开时，主内容区约 600~780px，正好命中 4 列。
const scrollContainer = ref<HTMLElement | null>(null);
const columnCount = ref(4);
const rowHeight = ref(240);
// 网格水平/垂直间距（px），与模板里的 gap-X 保持同步
const GRID_GAP = 16;

function recalcLayout() {
  const el = scrollContainer.value;
  if (!el) return;
  const w = el.clientWidth;
  let cols = 4;
  if (w < 420) cols = 2;
  else if (w < 720) cols = 3;
  columnCount.value = cols;

  // 行高估算：封面边长 + mb-4(16px) + 标题(24px) + 艺人/计数行(18px)
  // 注意：封面尺寸由 grid 自动均分，这里用估算值反推行高供虚拟列表使用。
  const coverSize = Math.max(100, (w - (cols - 1) * GRID_GAP) / cols);
  rowHeight.value = Math.round(coverSize + 58);
}

let resizeObserver: ResizeObserver | null = null;

// ============ 手写虚拟滚动（适配可变行高） ============
// 用一个 tick 触发可见项重算（因为 scrollTop 不是响应式）
const tick = ref(0);
// 渲染缓冲：上下各多渲染 4 行。
// 之前是 2 行（约 8 张图），滚动稍快时新行进入视口前图片来不及加载，会闪。
// 调到 4 行（约 16 张）能给图片更多预热时间，肉眼几乎察觉不到"边滚边加载"。
const BUFFER_ROWS = 4;

const rowCount = computed(() => Math.ceil(playerStore.albums.length / columnCount.value));
const totalHeight = computed(() => rowCount.value * rowHeight.value);

interface VisibleItem { index: number; data: Album }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value; // 建立对滚动的响应依赖
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
  const startRow = Math.max(0, Math.floor(el.scrollTop / rowHeight.value) - BUFFER_ROWS);
  return startRow * rowHeight.value;
});

// ============ 封面预加载 ============
// 思路：用户滚动时，在虚拟窗口"之外"再额外预热 PREFETCH_ROWS 行的封面，
// 让它们提前进入内存 dataURL 缓存。当用户继续滚动让这些行进入 BUFFER_ROWS 时，
// <ArtworkImage> 直接命中缓存，零网络请求 → 视觉上无闪烁。
//
// 预加载是 fire-and-forget 的，不阻塞滚动；prefetchArtworks 内部自带
// 并发限制和去重，多次快速触发不会累积成请求风暴。
const PREFETCH_ROWS = BUFFER_ROWS + 4; // 在渲染 buffer 之外再预热 4 行
// [诊断] 临时开关：设为 false 可禁用封面预加载，用于验证 IPC 通道是否被 fetch 拥堵。
// 验证完毕确认瓶颈后，会改用更合理的方案（错峰/降并发）并把这个开关移除。
const PREFETCH_ENABLED = true;
// 节流：滚动事件极频繁，但预加载不必每帧都跑——相邻两次调用间隔小于阈值就跳过
let lastPrefetchTs = 0;
const PREFETCH_THROTTLE_MS = 250;

function maybePrefetch() {
  if (!PREFETCH_ENABLED) return; // [诊断] 临时禁用，验证 IPC 拥堵假设
  const now = Date.now();
  if (now - lastPrefetchTs < PREFETCH_THROTTLE_MS) return;
  lastPrefetchTs = now;

  const el = scrollContainer.value;
  const all = playerStore.albums;
  if (!el || all.length === 0) return;
  const cols = columnCount.value;
  const rh = rowHeight.value;

  const curRow = Math.floor(el.scrollTop / rh);
  // 预热范围：从当前行往前 PREFETCH_ROWS 行 到 往后 (可视行数 + PREFETCH_ROWS) 行
  const startRow = Math.max(0, curRow - PREFETCH_ROWS);
  const visibleRows = Math.ceil(el.clientHeight / rh);
  const endRow = Math.min(rowCount.value, curRow + visibleRows + PREFETCH_ROWS);

  const startIdx = startRow * cols;
  const endIdx = Math.min(all.length, endRow * cols);

  const ids: Array<number | null | undefined> = [];
  for (let i = startIdx; i < endIdx; i++) {
    ids.push(all[i]?.cover_artwork_id);
  }
  // fire-and-forget，错误在 prefetchArtworks 内部已吞掉
  prefetchArtworks(ids, (id) => getArtworkUrl(id));
}

// rAF 节流的滚动处理：① 递增 tick 触发可见项重算；② 到底加载更多；③ 触发预加载
let ticking = false;
function onScroll(e: Event) {
  tick.value++;
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
  const target = e.target as HTMLElement;
  // 接近底部时拉取下一页数据（阈值加大到 600px，配合更大的 buffer 提前加载）
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 600) {
    playerStore.fetchAlbums();
  }
  // 触发封面预热（内部已节流）
  maybePrefetch();
}

onMounted(() => {
  recalcLayout();
  if (typeof ResizeObserver !== 'undefined' && scrollContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      recalcLayout();
      tick.value++; // 行高/列数变化，立即重切片
    });
    resizeObserver.observe(scrollContainer.value);
  }
  if (playerStore.albums.length === 0) {
    playerStore.fetchAlbums(true).then(() => {
      // 首屏数据到位后立即预热第一批封面，避免用户一开始滚动就遇到加载
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
    <div v-if="playerStore.isLoadingAlbums && playerStore.albums.length === 0" class="flex-1 flex flex-col items-center justify-center py-20 text-text-muted tracking-[0.25em] text-xs">
      <span class="animate-pulse">LOADING METADATA...</span>
    </div>

    <!-- 加载出错 -->
    <div v-else-if="playerStore.isErrorAlbums" class="flex-1 flex flex-col items-center justify-center py-20 text-[#d25050] tracking-[0.25em] text-xs font-bold uppercase">
      <span>加载专辑失败，请稍后重试</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="playerStore.albums.length === 0" class="flex-1 flex flex-col items-center justify-center py-20">
      <p class="font-serif italic text-2xl text-accent/60 mb-4">暂无专辑</p>
      <p class="text-xs text-text-muted tracking-widest max-w-sm text-center leading-relaxed">
        未检测到您的专辑信息。请确保本地音乐目录中含有音频文件并已完成扫描。
      </p>
    </div>

    <!-- 正常渲染：虚拟滚动网格 -->
    <div
      v-else
      ref="scrollContainer"
      class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2"
      @scroll="onScroll"
    >
      <!--
        外层撑出完整高度（让滚动条长度正确）；
        内层用 translateY 推到当前可视窗口起点；
        只渲染当前窗口内的专辑（约 8-16 张，含 buffer）
      -->
      <div :style="{ height: totalHeight + 'px', position: 'relative' }">
        <div
          class="absolute top-0 left-0 right-0 will-change-transform"
          :style="{ transform: `translateY(${offsetY}px)` }"
        >
          <div
            class="grid pb-10 pt-2"
            :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`, gap: GRID_GAP + 'px' }"
          >
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @click="goToAlbum(item.data.id)"
              class="group cursor-pointer flex flex-col"
            >
              <!-- 专辑封面（gap 缩小后整体更紧凑，每张变小以容纳 4 列） -->
              <div class="relative aspect-square w-full mb-3 overflow-hidden bg-[#e8e6df] shadow-sm">
                <ArtworkImage
                  :artwork-id="item.data.cover_artwork_id"
                  :fallback-color="item.data.coverColor"
                  img-class="group-hover:scale-105 transition-transform duration-300 ease-out"
                />
                <div class="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors duration-200 pointer-events-none"></div>
              </div>

              <!-- 专辑信息（字号略缩，避免 4 列下文字挤占） -->
              <div class="flex flex-col gap-0.5">
                <h3 class="font-serif italic font-semibold text-base text-accent truncate leading-tight">{{ item.data.title }}</h3>
                <div class="flex items-center justify-between gap-2">
                  <p class="text-[11px] font-medium text-text-muted truncate">{{ item.data.artist_name || 'Unknown Artist' }}</p>
                  <span class="text-[9px] tracking-widest text-text-muted shrink-0">{{ item.data.track_count }} TRACKS</span>
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
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: transparent; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #dcdad1; }
</style>
