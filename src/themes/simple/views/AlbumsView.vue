<script setup lang="ts">
import { usePlayerStore } from '../../../stores/player';
import { onMounted, computed, ref, watch } from 'vue';

const playerStore = usePlayerStore();

const goToAlbum = (albumId: number) => {
  playerStore.activeAlbumId = albumId;
  playerStore.activeLibraryTab = '专辑详情';
};

// 5 列 × 3 行 = 15 张/页（与 store 里 albumsPageSize 对齐）
const columnCount = 5;

const currentPage = computed(() => playerStore.albumsCurrentPage);
const totalPages = computed(() => playerStore.albumsTotalPages);
const totalCount = computed(() => playerStore.albumsTotalCount);
const canPrev = computed(() => currentPage.value > 1);
const canNext = computed(() => currentPage.value < totalPages.value);

// 页码下拉框
const pageSelectOpen = ref(false);
const pageOptions = computed(() => {
  const total = totalPages.value;
  const cur = currentPage.value;
  const set = new Set<number>([1, total]);
  for (let p = cur - 5; p <= cur + 5; p++) {
    if (p >= 1 && p <= total) set.add(p);
  }
  return Array.from(set).sort((a, b) => a - b);
});

// 跳转输入框
const jumpInput = ref('');
function handleJumpInput() {
  const p = parseInt(jumpInput.value, 10);
  if (!isNaN(p) && p >= 1 && p <= totalPages.value) {
    playerStore.goToAlbumsPage(p);
  }
  jumpInput.value = '';
}

function handlePrev() { if (canPrev.value) playerStore.prevAlbumsPage(); }
function handleNext() { if (canNext.value) playerStore.nextAlbumsPage(); }
function handleJump(page: number) {
  playerStore.goToAlbumsPage(page);
  pageSelectOpen.value = false;
}
function togglePageSelect() { pageSelectOpen.value = !pageSelectOpen.value; }

function onDocumentClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (!target.closest('.page-select-wrap')) pageSelectOpen.value = false;
}
watch(pageSelectOpen, (open) => {
  if (open) document.addEventListener('click', onDocumentClick);
  else document.removeEventListener('click', onDocumentClick);
});

onMounted(() => {
  if (playerStore.albums.length === 0 || playerStore.albumsCurrentPage === 1) {
    playerStore.fetchAlbums(true);
  }
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- 加载中（首屏） -->
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

    <!-- 正常渲染：分页网格 -->
    <div v-else class="flex-1 flex flex-col min-h-0">
      <div v-if="playerStore.isLoadingAlbums" class="absolute top-2 left-1/2 -translate-x-1/2 z-20 px-3 py-1 bg-[#fdfcf9]/90 border border-[#dcdad1] text-[10px] tracking-widest text-text-muted">
        LOADING...
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-2">
        <div
          class="grid pb-6 pt-2"
          :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`, gap: '16px' }"
        >
          <div
            v-for="item in playerStore.albums"
            :key="item.id"
            @click="goToAlbum(item.id)"
            class="group cursor-pointer flex flex-col"
          >
            <div class="relative aspect-square w-full mb-2 overflow-hidden bg-[#e8e6df] shadow-sm">
              <img
                v-if="item.cover_thumb"
                :src="item.cover_thumb"
                loading="lazy"
                decoding="async"
                class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300 ease-out"
              />
              <div
                v-else
                class="w-full h-full bg-gradient-to-br opacity-80"
                :class="item.coverColor || 'from-gray-400 to-gray-600'"
              ></div>
              <div class="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors duration-200 pointer-events-none"></div>
            </div>
            <div class="flex flex-col gap-0.5">
              <h3 class="font-serif italic font-semibold text-sm text-accent truncate leading-tight">{{ item.title }}</h3>
              <div class="flex items-center justify-between gap-1">
                <p class="text-[10px] font-medium text-text-muted truncate">{{ item.artist_name || 'Unknown Artist' }}</p>
                <span class="text-[9px] tracking-widest text-text-muted shrink-0">{{ item.track_count }}T</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 分页导航栏 -->
      <div class="flex items-center justify-center gap-4 py-3 border-t border-[#e8e6df] relative">
        <!-- 上一页 -->
        <button
          @click="handlePrev"
          :disabled="!canPrev"
          class="text-[11px] tracking-widest text-text-muted hover:text-accent transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
        >
          ←
        </button>

        <!-- 页码下拉框 + 当前/总页数 -->
        <div class="page-select-wrap relative">
          <button
            @click="togglePageSelect"
            class="font-serif italic text-sm text-accent hover:underline flex items-center gap-1"
          >
            {{ currentPage }}
            <span class="text-text-muted mx-0.5">/</span>
            <span class="text-text-muted">{{ totalPages }}</span>
            <span class="text-[9px] text-text-muted ml-1">▾</span>
          </button>
          <div
            v-if="pageSelectOpen"
            class="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 bg-[#fdfcf9] border border-[#dcdad1] shadow-lg py-1 min-w-[80px] max-h-[240px] overflow-y-auto z-30"
          >
            <button
              v-for="p in pageOptions"
              :key="p"
              @click="handleJump(p)"
              class="block w-full text-center py-1.5 text-xs hover:bg-[#eae8e1] transition-colors"
              :class="p === currentPage ? 'font-serif italic text-accent font-bold' : 'text-text-muted'"
            >
              {{ p }}
            </button>
          </div>
        </div>

        <!-- 下一页 -->
        <button
          @click="handleNext"
          :disabled="!canNext"
          class="text-[11px] tracking-widest text-text-muted hover:text-accent transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
        >
          →
        </button>

        <!-- 分隔符 -->
        <span class="text-[#dcdad1] mx-1">|</span>

        <!-- 总数量 -->
        <span class="text-[10px] tracking-widest text-text-muted">共 {{ totalCount }} 张</span>

        <!-- 分隔符 -->
        <span class="text-[#dcdad1] mx-1">|</span>

        <!-- 跳转输入框 -->
        <div class="flex items-center gap-1">
          <span class="text-[10px] tracking-widest text-text-muted">跳至</span>
          <input
            v-model="jumpInput"
            @keyup.enter="handleJumpInput"
            type="number"
            min="1"
            :max="totalPages"
            class="w-12 px-1 py-0.5 text-xs text-center bg-transparent border border-[#dcdad1] focus:border-accent focus:outline-none text-text-muted"
            placeholder="页"
          />
          <span class="text-[10px] tracking-widest text-text-muted">页</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background-color: #dcdad1; border-radius: 10px; }
.custom-scrollbar:hover::-webkit-scrollbar-thumb { background-color: #b4b2a9; }
</style>
