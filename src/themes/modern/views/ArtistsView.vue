<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
import { onMounted, ref, computed, onUnmounted } from 'vue';
import type { Artist } from '../../../stores/player';

const playerStore = usePlayerStore();

const goToArtist = (artistId: number) => {
  playerStore.activeArtistId = artistId;
  playerStore.activeLibraryTab = '艺人详情';
};

// ============ 响应式列数 ============
const scrollContainer = ref<HTMLElement | null>(null);
const columnCount = ref(5);
const rowHeight = ref(220);
const GRID_GAP = 20;

function recalcLayout() {
  const el = scrollContainer.value;
  if (!el) return;
  const w = el.clientWidth;
  let cols = 5;
  if (w < 460) cols = 2;
  else if (w < 680) cols = 3;
  else if (w < 920) cols = 4;
  else cols = 5;
  columnCount.value = cols;

  // 行高：头像直径 + 名字 20 + 计数 16 + 间距 ≈ 96
  const avatarSize = Math.max(80, (w - (cols - 1) * GRID_GAP) / cols);
  rowHeight.value = Math.round(avatarSize + 96);
}

let resizeObserver: ResizeObserver | null = null;

// ============ 虚拟滚动 ============
const tick = ref(0);
const BUFFER_ROWS = 4;
const rowCount = computed(() => Math.ceil(playerStore.artists.length / columnCount.value));
const totalHeight = computed(() => rowCount.value * rowHeight.value);

interface VisibleItem { index: number; data: Artist }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value;
  const el = scrollContainer.value;
  const cols = columnCount.value;
  const rh = rowHeight.value;
  const all = playerStore.artists;
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

let ticking = false;
const handleScroll = (e: Event) => {
  tick.value++;
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 400) {
    playerStore.fetchArtists();
  }
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
};

// 艺人名首字符（用作圆形头像里的字母占位）
const initialOf = (name: string) => (name?.trim()?.[0] || '?').toUpperCase();

onMounted(() => {
  recalcLayout();
  if (typeof ResizeObserver !== 'undefined' && scrollContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      recalcLayout();
      tick.value++;
    });
    resizeObserver.observe(scrollContainer.value);
  }
  if (playerStore.artists.length === 0) {
    playerStore.fetchArtists(true);
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
    <div v-if="playerStore.isLoadingArtists && playerStore.artists.length === 0" class="flex-1 flex flex-col items-center justify-center text-text-muted">
      <div class="w-8 h-8 border-2 border-accent/20 border-t-accent rounded-full animate-spin mb-4"></div>
      <span class="text-xs tracking-[0.2em] uppercase">Loading Artists…</span>
    </div>

    <!-- 加载出错 -->
    <div v-else-if="playerStore.isErrorArtists" class="flex-1 flex flex-col items-center justify-center text-[#d25050]">
      <span class="text-sm font-medium">加载艺人失败，请稍后重试</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="playerStore.artists.length === 0" class="flex-1 flex flex-col items-center justify-center px-8">
      <div class="w-20 h-20 rounded-2xl bg-bg-active flex items-center justify-center mb-6">
        <span class="text-3xl text-text-muted">♫</span>
      </div>
      <p class="text-lg font-semibold text-text-main mb-2">暂无艺人</p>
      <p class="text-xs text-text-muted tracking-wide max-w-sm text-center leading-relaxed">
        未检测到艺人列表。请确保本地音乐目录中含有音频文件并已完成扫描。
      </p>
    </div>

    <!-- 正常渲染：圆形头像卡片网格 -->
    <div
      v-else
      ref="scrollContainer"
      class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-1"
      @scroll="handleScroll"
    >
      <div :style="{ height: totalHeight + 'px', position: 'relative' }">
        <div class="absolute top-0 left-0 right-0 will-change-transform" :style="{ transform: `translateY(${offsetY}px)` }">
          <div
            class="grid pb-10 pt-1"
            :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`, gap: GRID_GAP + 'px' }"
          >
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @click="goToArtist(item.data.id)"
              class="group cursor-pointer flex flex-col items-center text-center px-2"
            >
              <!-- 圆形头像：渐变背景 + 首字母占位 + 悬浮播放 -->
              <div class="relative mb-4">
                <div
                  class="w-full aspect-square rounded-full bg-gradient-to-br flex items-center justify-center shadow-sm group-hover:shadow-xl transition-all duration-300 overflow-hidden"
                  :class="item.data.avatarColor"
                  :style="{ maxWidth: '160px' }"
                >
                  <span class="text-3xl font-bold text-white/90 select-none">{{ initialOf(item.data.name) }}</span>
                </div>
                <!-- 悬浮播放遮罩 -->
                <div class="absolute inset-0 rounded-full bg-black/0 group-hover:bg-black/30 transition-colors duration-300 flex items-center justify-center max-w-[160px] mx-auto">
                  <div class="w-12 h-12 rounded-full bg-accent shadow-lg flex items-center justify-center opacity-0 scale-75 group-hover:opacity-100 group-hover:scale-100 transition-all duration-300">
                    <svg class="w-5 h-5 text-white ml-0.5" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
                  </div>
                </div>
              </div>

              <h3 class="font-semibold text-[14px] text-text-main truncate w-full group-hover:text-accent transition-colors leading-tight">{{ item.data.name }}</h3>
              <p class="text-[12px] text-text-muted mt-0.5">{{ item.data.track_count || 0 }} 首歌曲</p>
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
