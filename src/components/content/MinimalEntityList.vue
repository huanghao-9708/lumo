<script setup lang="ts">
import { computed, toRef } from 'vue';
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
}>();

const emit = defineEmits<{ select: [id: number] }>();

const ROW_HEIGHT = 44;
const scrollContainer = useScrollRestore(() => `minimal:${props.scrollKey}`);
const { totalHeight, offsetY, visibleItems } = useVirtualList({
  containerRef: scrollContainer,
  items: toRef(props, 'items') as any,
  itemHeight: ROW_HEIGHT,
  buffer: 10,
});

const iconComponent = computed(() => (props.kind === 'album' ? Disc3 : Mic2));
</script>

<template>
  <div
    ref="scrollContainer"
    class="flex-1 overflow-y-auto px-8 pb-6"
    data-experience-minimal-list
  >
    <!-- 空态 -->
    <div
      v-if="items.length === 0"
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
  </div>
</template>
