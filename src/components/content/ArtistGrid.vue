<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { User, Loader2, Sparkles, CheckCircle2, X } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import { libraryGetArtistMatchTargets, libraryMatchSingleArtistCover } from '../../api/library';
import type { ArtistMatchTargetDTO } from '../../api/types';
import { useScrollRestore } from '../../composables/useScrollRestore';

const playerStore = usePlayerStore();

const artists = computed(() => playerStore.artists);
const isLoading = computed(() => playerStore.isLoadingArtists);
const hasMore = computed(() => playerStore.hasMoreArtists);

function selectArtist(artistId: number) {
  // 走 store 的导航函数（同 tick 改 id+tab，只记一条历史），返回时可精确还原
  playerStore.navigateToArtist(artistId);
}

function getColorClass(color: string): string {
  return color || 'from-warm-500 to-warm-700';
}

/* ============ 网络匹配艺人图片功能（全库覆盖） ============ */
const isMatching = ref(false);
const matchCompleted = ref(false);
const matchTotal = ref(0);
const matchProcessed = ref(0);
const matchSuccessCount = ref(0);
const currentMatchName = ref('');
const missingTotalCount = ref<number | null>(null);
let matchAborted = false;

const missingCount = computed(() => missingTotalCount.value !== null ? missingTotalCount.value : artists.value.filter(a => !a.avatar_artwork_id).length);
const matchPercent = computed(() => (matchTotal.value ? Math.min(100, (matchProcessed.value / matchTotal.value) * 100) : 0));

async function refreshMissingCount() {
  try {
    const missingTargets = await libraryGetArtistMatchTargets(true);
    missingTotalCount.value = missingTargets.length;
  } catch (e) {
    console.error('获取全库待匹配艺人数失败:', e);
  }
}

async function startMatchingCovers() {
  if (isMatching.value) return;

  // 默认从后端数据库拉取所有未拥有头像的艺人；若全库均已拥有头像，则匹配全库全部艺人
  let targets: ArtistMatchTargetDTO[] = [];
  try {
    targets = await libraryGetArtistMatchTargets(true);
    if (targets.length === 0) {
      targets = await libraryGetArtistMatchTargets(false);
    }
  } catch (e) {
    console.error('获取全量艺人匹配目标失败:', e);
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
      currentMatchName.value = cur.name;
      try {
        const res = await libraryMatchSingleArtistCover(cur.id, true);
        if (res) {
          matchSuccessCount.value++;
          if (missingTotalCount.value !== null && missingTotalCount.value > 0) {
            missingTotalCount.value = Math.max(0, missingTotalCount.value - 1);
          }
        }
      } catch (e) {
        console.error('匹配艺人图片失败:', cur.name, e);
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
// 滚动位置记忆：返回列表时还原到原位置（key 带上过滤条件，避免过滤后错位）
const scrollContainer = useScrollRestore(
  () => `artist-grid:${playerStore.hideMinorArtists ? 1 : 0}:${playerStore.searchQuery}`
);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

onMounted(() => {
  refreshMissingCount();
  if (!scrollContainer.value) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && !isLoading.value && hasMore.value) {
        playerStore.fetchArtists(false);
      }
    },
    { root: scrollContainer.value, rootMargin: '400px' }
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
  <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8">
    
    <!-- 顶部操作条与匹配控制 -->
    <div class="mb-5 flex items-center justify-between gap-4 flex-wrap select-none pt-1">
      <div class="flex items-center gap-2">
        <span class="text-[13px] font-medium text-text-primary">全部艺术家</span>
        <span class="text-[11px] font-mono text-text-muted">({{ playerStore.artistsTotalCount.toLocaleString() }} 位)</span>
        <span v-if="missingCount > 0" class="text-[11px] text-brand-orange bg-brand-orange/10 px-2.5 py-0.5 rounded-full font-medium">
          {{ missingCount }} 位待匹配头像
        </span>
      </div>

      <!-- 匹配按钮 -->
      <button
        v-if="!isMatching"
        @click="startMatchingCovers"
        class="h-[32px] px-3.5 rounded-[8px] text-[12px] font-medium bg-bg-content border border-border-color hover:border-brand-orange/40 hover:text-brand-orange transition-all flex items-center gap-2 shadow-sm"
        :title="missingCount === 0 ? '从互联网重新搜索并补全整个曲库所有艺人头像' : '从互联网自动搜索并补全未拥有头像的艺人'"
      >
        <Sparkles class="w-3.5 h-3.5 text-brand-orange" />
        {{ missingCount === 0 ? '重新匹配全部头像' : '匹配网络艺人图片' }}
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
              {{ isMatching ? `正在从网络匹配艺人图片：${currentMatchName}` : '网络艺人图片匹配完成！' }}
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
          <span>已成功匹配 {{ matchSuccessCount }} 位艺人图片</span>
          <span v-if="matchCompleted" class="text-text-muted/60">4秒后自动收起</span>
        </div>
      </div>
    </Transition>

    <!-- 艺人网格：统一圆形头像与精美居中排版 -->
    <div
      class="grid gap-6 pb-4"
      style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))"
    >
      <div
        v-for="artist in artists"
        :key="artist.id"
        class="group cursor-pointer flex flex-col items-center text-center"
        @click="selectArtist(artist.id)"
      >
        <!-- 统一圆形头像容器：尺寸一致、圆角一致、光影一致 -->
        <div
          class="w-full aspect-square mb-3 rounded-full overflow-hidden flex items-center justify-center relative shadow-sm ring-1 ring-black/5 dark:ring-white/10 group-hover:shadow-md group-hover:scale-[1.03] transition-all duration-300"
          :class="!artist.avatar_artwork_id ? `bg-gradient-to-br ${getColorClass(artist.avatarColor)}` : 'bg-bg-hover'"
        >
          <img 
            v-if="artist.avatar_artwork_id"
            :src="getArtworkUrl(artist.avatar_artwork_id)"
            class="w-full h-full object-cover rounded-full"
            :alt="artist.name"
            loading="lazy"
          />
          <div v-else class="w-full h-full flex items-center justify-center bg-black/10 group-hover:bg-black/20 transition-colors-smooth">
            <User class="w-[42px] h-[42px] text-white/70" />
          </div>
        </div>

        <p class="text-[14px] text-text-primary font-medium truncate w-full leading-tight mb-1 group-hover:text-brand-orange transition-colors-smooth">{{ artist.name }}</p>
        <p class="text-[12px] text-text-secondary truncate w-full">{{ artist.trackCount }} 首歌曲</p>
      </div>
    </div>

    <div v-if="artists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
      <User class="w-8 h-8 text-text-disabled" />
      <span class="text-[12px]">还没有扫描到艺术家</span>
    </div>

    <!-- sentinel — IntersectionObserver 触发加载下一批 -->
    <div ref="sentinelRef" class="h-px" />

    <!-- 增量加载指示 -->
    <div v-if="isLoading && artists.length > 0" class="flex items-center justify-center py-6 text-text-muted">
      <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
      <span class="text-[11px]">加载更多…</span>
    </div>

    <!-- 没有更多了 -->
    <div v-if="!hasMore && artists.length > 0" class="flex items-center justify-center py-6 text-text-muted">
      <span class="text-[11px]">已显示全部 {{ playerStore.artistsTotalCount.toLocaleString() }} 位艺术家</span>
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
