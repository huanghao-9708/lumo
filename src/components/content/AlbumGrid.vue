<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { Play, Loader2, Disc3, Sparkles, CheckCircle2, X } from 'lucide-vue-next';
import { usePlayerStore, type Album } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import { libraryGetAlbumTracks, libraryGetAlbumMatchTargets, libraryMatchSingleAlbumCover } from '../../api/library';
import type { AlbumMatchTargetDTO } from '../../api/types';
import { useScrollRestore } from '../../composables/useScrollRestore';

const playerStore = usePlayerStore();

const emit = defineEmits<{
  (e: 'select', album: Album): void;
}>();

/** 封面 src：优先使用 cover_thumb（后端内联 base64），否则走 artwork URL 协议 */
function getCoverSrc(album: Album): string {
  if (album.cover_thumb) return album.cover_thumb;
  if (album.cover_artwork_id) return getArtworkUrl(album.cover_artwork_id);
  return '';
}

function selectAlbum(album: Album) {
  emit('select', album);
}

/** 双击专辑封面 → 直接播放第一首 */
async function playAlbum(album: Album) {
  try {
    const result = await libraryGetAlbumTracks(album.id);
    const tracks = result.map(t => {
      const durationMs = t.duration_ms ?? 0;
      const sec = Math.floor(durationMs / 1000);
      return {
        id: t.id,
        title: t.title,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || album.title,
        duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
        durationSec: sec,
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        artistId: t.artist_id ?? null,
        albumId: t.album_id ?? null,
        coverColor: '',
        isFavorite: false,
        primary_file_id: t.media_file_id,
        cover_artwork_id: t.cover_artwork_id,
        fileSize: t.file_size ?? null,
        sourceKind: (t.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
      };
    });
    if (tracks.length > 0) {
      await playerStore.playAll(tracks, 0);
    }
  } catch (e) {
    console.error('Failed to play album:', e);
  }
}

const totalCount = computed(() => playerStore.albumsTotalCount);
const isLoading = computed(() => playerStore.isLoadingAlbums);
const isError = computed(() => playerStore.isErrorAlbums);
const hasMoreAlbums = computed(() => playerStore.hasMoreAlbums);

/* ============ 网络匹配专辑封面功能（全库覆盖） ============ */
const isMatching = ref(false);
const matchCompleted = ref(false);
const matchTotal = ref(0);
const matchProcessed = ref(0);
const matchSuccessCount = ref(0);
const currentMatchName = ref('');
const missingTotalCount = ref<number | null>(null);
let matchAborted = false;

const missingCount = computed(() => missingTotalCount.value !== null ? missingTotalCount.value : playerStore.albums.filter(a => !a.cover_artwork_id && !a.cover_thumb).length);
const matchPercent = computed(() => (matchTotal.value ? Math.min(100, (matchProcessed.value / matchTotal.value) * 100) : 0));

async function refreshMissingCount() {
  try {
    const missingTargets = await libraryGetAlbumMatchTargets(true);
    missingTotalCount.value = missingTargets.length;
  } catch (e) {
    console.error('获取全库待匹配专辑数失败:', e);
  }
}

async function startMatchingCovers() {
  if (isMatching.value) return;

  // 默认从后端数据库拉取所有未拥有封面的专辑；若全库均已拥有封面，则匹配全库全部专辑
  let targets: AlbumMatchTargetDTO[] = [];
  try {
    targets = await libraryGetAlbumMatchTargets(true);
    if (targets.length === 0) {
      targets = await libraryGetAlbumMatchTargets(false);
    }
  } catch (e) {
    console.error('获取全量匹配目标失败:', e);
    return;
  }

  if (targets.length === 0) return;

  isMatching.value = true;
  matchCompleted.value = false;
  matchTotal.value = targets.length;
  matchProcessed.value = 0;
  matchSuccessCount.value = 0;
  matchAborted = false;

  // 并发 2 个，平滑有序推进，避免触发网络请求风控
  const CONCURRENCY = 2;
  let idx = 0;

  async function worker() {
    while (idx < targets.length && !matchAborted) {
      const cur = targets[idx++];
      if (!cur) break;
      currentMatchName.value = cur.artistName ? `${cur.title} (${cur.artistName})` : cur.title;
      try {
        const res = await libraryMatchSingleAlbumCover(cur.id, true);
        if (res) {
          matchSuccessCount.value++;
          if (missingTotalCount.value !== null && missingTotalCount.value > 0) {
            missingTotalCount.value = Math.max(0, missingTotalCount.value - 1);
          }
        }
      } catch (e) {
        console.error('匹配专辑封面失败:', cur.title, e);
      }
      matchProcessed.value++;
      await new Promise(r => setTimeout(r, 160));
    }
  }

  const workers = Array.from({ length: Math.min(CONCURRENCY, targets.length) }, () => worker());
  await Promise.all(workers);

  isMatching.value = false;
  if (!matchAborted) {
    matchCompleted.value = true;
    refreshMissingCount();
    setTimeout(() => {
      matchCompleted.value = false;
    }, 4000);
  } else {
    refreshMissingCount();
  }
}

function abortMatching() {
  matchAborted = true;
  isMatching.value = false;
}

/** IntersectionObserver: 滚动到底部自动加载下一批 */
// 滚动位置记忆：从专辑详情返回时还原到原位置（key 带上过滤条件，避免过滤后错位）
const gridContainer = useScrollRestore(
  () => `album-grid:${playerStore.hideSmallAlbums ? 1 : 0}:${playerStore.searchQuery}`
);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

onMounted(() => {
  refreshMissingCount();
  if (!gridContainer.value) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && !isLoading.value && hasMoreAlbums.value) {
        playerStore.fetchAlbums(false);
      }
    },
    { root: gridContainer.value, rootMargin: '400px' }
  );
  if (sentinelRef.value) {
    observer.observe(sentinelRef.value);
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
});
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden">

    <!-- 网格容器 -->
    <div ref="gridContainer" class="flex-1 overflow-y-auto px-8">

      <!-- 顶部操作条与匹配控制 -->
      <div class="mb-5 flex items-center justify-between gap-4 flex-wrap select-none pt-1">
        <div class="flex items-center gap-2">
          <span class="text-[13px] font-medium text-text-primary">全部专辑</span>
          <span class="text-[11px] font-mono text-text-muted">({{ totalCount.toLocaleString() }} 张)</span>
          <span v-if="missingCount > 0" class="text-[11px] text-brand-orange bg-brand-orange/10 px-2.5 py-0.5 rounded-full font-medium">
            {{ missingCount }} 张待匹配封面
          </span>
        </div>

        <!-- 匹配按钮 -->
        <button
          v-if="!isMatching"
          @click="startMatchingCovers"
          class="h-[32px] px-3.5 rounded-[8px] text-[12px] font-medium bg-bg-content border border-border-color hover:border-brand-orange/40 hover:text-brand-orange transition-all flex items-center gap-2 shadow-sm"
          :title="missingCount === 0 ? '从互联网重新搜索并补全整个曲库所有专辑封面' : '从互联网自动搜索并补全未拥有封面的专辑'"
        >
          <Sparkles class="w-3.5 h-3.5 text-brand-orange" />
          {{ missingCount === 0 ? '重新匹配全部封面' : '匹配网络封面' }}
        </button>
        
        <button
          v-else
          @click="abortMatching"
          class="h-[32px] px-3.5 rounded-[8px] text-[12px] font-medium bg-status-error/10 text-status-error border border-status-error/20 hover:bg-status-error/20 transition-all flex items-center gap-1.5"
          title="停止匹配"
        >
          <X class="w-3.5 h-3.5" />
          停止匹配
        </button>
      </div>

      <!-- 动态进度条卡片 -->
      <Transition name="fade-slide">
        <div
          v-if="isMatching || matchCompleted"
          class="mb-6 p-4 rounded-[10px] bg-bg-content border border-border-color shadow-sm transition-all"
        >
          <div class="flex items-center justify-between text-[12px] mb-2">
            <div class="flex items-center gap-2 min-w-0">
              <Loader2 v-if="isMatching" class="w-4 h-4 animate-spin text-brand-orange shrink-0" />
              <CheckCircle2 v-else class="w-4 h-4 text-status-success shrink-0" />
              <span class="font-medium text-text-primary truncate">
                {{ isMatching ? `正在从网络匹配封面：${currentMatchName}` : '网络封面匹配完成！' }}
              </span>
            </div>
            <span class="font-mono text-text-muted tabular-nums shrink-0 ml-2">
              {{ matchProcessed }} / {{ matchTotal }} ({{ Math.round(matchPercent) }}%)
            </span>
          </div>

          <!-- 进度条轨道 -->
          <div class="w-full h-1.5 bg-border-solid rounded-full overflow-hidden">
            <div
              class="h-full bg-brand-orange rounded-full transition-all duration-300 ease-out"
              :style="{ width: `${matchPercent}%` }"
            ></div>
          </div>

          <div class="flex items-center justify-between mt-2 text-[11px] text-text-muted">
            <span>已成功匹配 {{ matchSuccessCount }} 张专辑封面</span>
            <span v-if="matchCompleted" class="text-text-muted/60">4秒后自动收起</span>
          </div>
        </div>
      </Transition>

      <!-- 加载态 -->
      <div v-if="isLoading && playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
        <span class="text-[12px]">加载专辑…</span>
      </div>

      <!-- 错误态 -->
      <div v-else-if="isError && playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <span class="text-[12px]">加载失败，请稍后重试</span>
      </div>

      <!-- 空态 -->
      <div v-else-if="playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">没有找到专辑</span>
      </div>

      <!-- 5 列网格 -->
      <div
        v-else
        class="grid gap-6 pb-6"
        style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));"
      >
        <div
          v-for="album in playerStore.albums"
          :key="album.id"
          class="group cursor-pointer min-w-0"
          @dblclick="playAlbum(album)"
        >
          <!-- 封面 -->
          <div
            class="relative w-full aspect-square rounded-[12px] overflow-hidden bg-bg-hover mb-3 ring-1 ring-black/5 dark:ring-white/10 shadow-sm group-hover:shadow-[0_12px_24px_-6px_rgba(0,0,0,0.18)] dark:group-hover:shadow-[0_12px_24px_-6px_rgba(0,0,0,0.5)] group-hover:-translate-y-1 transition-all duration-200"
            @click="selectAlbum(album)"
          >
            <img
              v-if="getCoverSrc(album)"
              :src="getCoverSrc(album)"
              :alt="album.title"
              class="w-full h-full object-cover group-hover:scale-[1.03] transition-transform duration-300 ease-out"
              loading="lazy"
            />
            <div v-else class="w-full h-full flex items-center justify-center bg-bg-hover">
              <Disc3 class="w-10 h-10 text-text-disabled" aria-hidden="true" />
            </div>

            <!-- 悬浮播放按钮 -->
            <div
              class="absolute inset-0 bg-black/0 group-hover:bg-black/25 dark:group-hover:bg-black/45 transition-colors-smooth flex items-center justify-center opacity-0 group-hover:opacity-100"
              @click.stop="playAlbum(album)"
            >
              <div class="w-11 h-11 rounded-full bg-brand-orange text-white flex items-center justify-center shadow-lg transform scale-90 group-hover:scale-100 transition-transform duration-200 hover:scale-105 active:scale-95">
                <Play class="w-5 h-5 fill-current ml-0.5" />
              </div>
            </div>
          </div>

          <!-- 标题 + 艺术家 -->
          <p
            class="text-[15px] font-medium text-text-primary truncate mb-0.5"
            :class="playerStore.activeAlbumId === album.id ? 'text-brand-orange' : ''"
            @click="selectAlbum(album)"
          >{{ album.title }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ album.artist }}<span v-if="album.year"> · {{ album.year }}</span></p>
        </div>
      </div>

      <!-- sentinel — IntersectionObserver 触发加载下一批 -->
      <div ref="sentinelRef" class="h-px" />

      <!-- 增量加载指示 -->
      <div v-if="isLoading && playerStore.albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
        <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
        <span class="text-[11px]">加载更多…</span>
      </div>

      <!-- 没有更多了 -->
      <div v-if="!hasMoreAlbums && playerStore.albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
        <span class="text-[11px]">已显示全部 {{ totalCount.toLocaleString() }} 张专辑</span>
      </div>

    </div>
  </div>
</template>

<style scoped>
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
