<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch } from 'vue';
import { Disc3, Mic2 } from 'lucide-vue-next';
import { useVirtualList } from '../../composables/useVirtualList';
import { useScrollRestore } from '../../composables/useScrollRestore';

/**
 * MinimalEntityList —— 极简体验模式的文字列表（DM-04）。
 *
 * 专辑网格/艺人网格/收藏专辑/收藏歌手在极简下的共同替代视图：
 * 纯文字行（名称 + 副行 + 计数），无封面、无装饰，点击行为与原网格一致
 * （由父级复用既有 store 动作处理）。虚拟列表渲染，大曲库不整表挂载。
 */
export interface MinimalEntityItem {
  id: number;
  primary: string;
  secondary?: string;
  hint?: string;
}

const props = defineProps<{
  items: MinimalEntityItem[];
  kind: 'album' | 'artist';
  /** 滚动位置记忆键（与原网格语义一致） */
  scrollKey: string;
  emptyText?: string;
  hasMore?: boolean;
  loading?: boolean;
  loadError?: boolean;
  loadMore?: () => Promise<void>;
}>();

const emit = defineEmits<{ select: [id: number] }>();

const ROW_HEIGHT = 44;
const scrollContainer = useScrollRestore(() => `minimal:${props.scrollKey}`);
const { totalHeight, offsetY, visibleItems } = useVirtualList({
  containerRef: scrollContainer,
  items: toRef(props, 'items'),
  itemHeight: ROW_HEIGHT,
  buffer: 10,
});

const iconComponent = computed(() => (props.kind === 'album' ? Disc3 : Mic2));
const requestPending = ref(false);
const requestFailed = ref(false);
const busy = computed(() => props.loading || requestPending.value);
const failed = computed(() => props.loadError || requestFailed.value);
let disposed = false;
let resizeObserver: ResizeObserver | null = null;

async function loadNextPage() {
  if (disposed || busy.value || !props.hasMore || !props.loadMore) return;
  requestPending.value = true;
  requestFailed.value = false;
  const previousLength = props.items.length;
  try {
    await props.loadMore();
  } catch {
    requestFailed.value = true;
  } finally {
    requestPending.value = false;
    // A short page may still leave room in a tall window. Recheck after layout,
    // but never spin on a failed request or a request that made no progress.
    if (!disposed && props.items.length > previousLength) await nextTick(checkNearBottom);
  }
}

function checkNearBottom() {
  const el = scrollContainer.value;
  if (disposed || !el || el.clientHeight <= 0 || failed.value) return;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 150) void loadNextPage();
}

watch(() => props.scrollKey, () => {
  requestFailed.value = false;
  if (scrollContainer.value) scrollContainer.value.scrollTop = 0;
});
watch([() => props.items.length, () => props.loading, () => props.hasMore, () => props.scrollKey], () => {
  void nextTick(checkNearBottom);
});
onMounted(() => {
  if (typeof ResizeObserver !== 'undefined' && scrollContainer.value) {
    resizeObserver = new ResizeObserver(checkNearBottom);
    resizeObserver.observe(scrollContainer.value);
  }
  void nextTick(checkNearBottom);
});
onBeforeUnmount(() => {
  disposed = true;
  resizeObserver?.disconnect();
});
</script>

<template>
  <div
    ref="scrollContainer"
    class="flex-1 overflow-y-auto px-8 pb-6"
    data-experience-minimal-list
    @scroll.passive="checkNearBottom"
  >
    <!-- 空态 -->
    <div
      v-if="items.length === 0 && !busy && !failed"
      class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted"
    >
      <component :is="iconComponent" class="w-8 h-8 text-text-disabled" />
      <span class="text-[12px]">{{ emptyText || '没有内容' }}</span>
    </div>

    <!-- 虚拟列表 -->
    <div v-else :style="{ height: totalHeight + 'px', position: 'relative' }">
      <div :style="{ transform: `translateY(${offsetY}px)` }">
        <button
          v-for="{ data: item } in visibleItems"
          :key="item.id"
          class="w-full flex items-center gap-4 text-left rounded-[8px] px-3 hover:bg-list-hover focus-visible:bg-list-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-orange/40 transition-colors-smooth"
          :style="{ height: ROW_HEIGHT + 'px' }"
          :title="item.primary"
          @click="emit('select', item.id)"
        >
          <component :is="iconComponent" class="w-4 h-4 text-text-disabled flex-shrink-0" />
          <span class="text-[14px] text-text-primary truncate min-w-0 flex-1 leading-tight">{{ item.primary }}</span>
          <span v-if="item.secondary" class="text-[12px] text-text-secondary truncate max-w-[40%] flex-shrink-0">{{ item.secondary }}</span>
          <span v-if="item.hint" class="text-[12px] text-text-muted font-mono flex-shrink-0 tabular-nums">{{ item.hint }}</span>
        </button>
      </div>
    </div>
    <div v-if="busy" role="status" class="py-4 text-center text-[12px] text-text-muted">加载中…</div>
    <div v-else-if="loadMore && (hasMore || failed)" class="py-4 text-center">
      <span v-if="failed" role="status" class="block mb-2 text-[12px] text-text-muted">加载失败，请重试</span>
      <button class="px-3 py-2 rounded-[8px] text-[12px] text-text-secondary hover:bg-list-hover focus-visible:ring-2 focus-visible:ring-brand-orange/40" @click="loadNextPage">
        {{ failed ? '重试加载' : '加载更多' }}
      </button>
    </div>
  </div>
</template>
