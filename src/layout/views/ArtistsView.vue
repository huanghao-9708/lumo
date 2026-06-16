<script setup lang="ts">
import { usePlayerStore } from '../../stores/player';
import { onMounted, ref, computed } from 'vue';
import type { Artist } from '../../stores/player';

const playerStore = usePlayerStore();

const goToArtist = (artistId: number) => {
  playerStore.activeArtistId = artistId;
  playerStore.activeLibraryTab = '艺人详情';
};

// ============ 虚拟滚动 ============
// 单行高度：py-5 (20px*2) + 头像 w-16 h-16 (64px) ≈ 104px
const ROW_HEIGHT = 104;
const BUFFER_ROWS = 4;
const scrollContainer = ref<HTMLElement | null>(null);
const tick = ref(0);

const totalHeight = computed(() => playerStore.artists.length * ROW_HEIGHT);

interface VisibleItem { index: number; data: Artist }

const visibleItems = computed<VisibleItem[]>(() => {
  void tick.value;
  const el = scrollContainer.value;
  const all = playerStore.artists;
  if (!el || all.length === 0) return [];

  const scrollTop = el.scrollTop;
  const viewH = el.clientHeight;
  const startRow = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS);
  const visibleRows = Math.ceil(viewH / ROW_HEIGHT) + BUFFER_ROWS * 2;
  const endRow = startRow + visibleRows;
  const endIdx = Math.min(all.length, endRow);

  const out: VisibleItem[] = [];
  for (let i = startRow; i < endIdx; i++) {
    out.push({ index: i, data: all[i] });
  }
  return out;
});

const offsetY = computed(() => {
  void tick.value;
  const el = scrollContainer.value;
  if (!el) return 0;
  return Math.max(0, Math.floor(el.scrollTop / ROW_HEIGHT) - BUFFER_ROWS) * ROW_HEIGHT;
});

let ticking = false;
const handleScroll = (e: Event) => {
  tick.value++;
  const target = e.target as HTMLElement;
  if (target.scrollHeight - target.scrollTop <= target.clientHeight + 300) {
    playerStore.fetchArtists();
  }
  if (!ticking) {
    ticking = true;
    requestAnimationFrame(() => { ticking = false; });
  }
};

onMounted(() => {
  if (playerStore.artists.length === 0) {
    playerStore.fetchArtists(true);
  }
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- 加载中 -->
    <div v-if="playerStore.isLoadingArtists && playerStore.artists.length === 0" class="flex-1 flex flex-col items-center justify-center py-20 text-text-muted tracking-[0.25em] text-xs">
      <span class="animate-pulse">LOADING METADATA...</span>
    </div>

    <!-- 加载出错 -->
    <div v-else-if="playerStore.isErrorArtists" class="flex-1 flex flex-col items-center justify-center py-20 text-[#d25050] tracking-[0.25em] text-xs font-bold uppercase">
      <span>加载艺人失败，请稍后重试</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="playerStore.artists.length === 0" class="flex-1 flex flex-col items-center justify-center py-20">
      <p class="font-serif italic text-2xl text-accent/60 mb-4">暂无艺人</p>
      <p class="text-xs text-text-muted tracking-widest max-w-sm text-center leading-relaxed">
        未检测到艺人列表。请确保本地音乐目录中含有音频文件并已完成扫描。
      </p>
    </div>

    <!-- 正常渲染：虚拟滚动 -->
    <div
      v-else
      ref="scrollContainer"
      class="flex-1 overflow-y-auto custom-scrollbar relative z-10 pr-2"
      @scroll="handleScroll"
    >
      <div :style="{ height: totalHeight + 'px', position: 'relative' }">
        <div class="absolute top-0 left-0 right-0 will-change-transform" :style="{ transform: `translateY(${offsetY}px)` }">
          <div class="flex flex-col pb-10">
            <div
              v-for="item in visibleItems"
              :key="item.data.id"
              @click="goToArtist(item.data.id)"
              class="group cursor-pointer flex items-center justify-between py-5 border-b border-[#f0eee6]/50 hover:border-black transition-colors"
              :style="{ height: ROW_HEIGHT + 'px' }"
            >
              <div class="flex items-center gap-8">
                <span class="text-[10px] font-bold tracking-widest text-text-muted w-6 text-right">
                  {{ String(item.index + 1).padStart(2, '0') }}
                </span>
                <div class="w-16 h-16 rounded-full overflow-hidden bg-[#e8e6df] shrink-0">
                  <div
                    class="w-full h-full bg-gradient-to-tr opacity-70 group-hover:opacity-100 transition-opacity"
                    :class="item.data.avatarColor"
                  ></div>
                </div>
                <h3 class="font-serif italic text-3xl text-accent group-hover:translate-x-2 transition-transform duration-300">{{ item.data.name }}</h3>
              </div>
              <div class="text-[10px] font-bold tracking-[0.2em] text-text-muted  uppercase">
                {{ item.data.track_count }} 首歌曲
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
