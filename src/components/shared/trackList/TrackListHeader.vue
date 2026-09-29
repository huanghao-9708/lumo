<script setup lang="ts">
import { ref, computed } from 'vue';
import { Settings2, ListChecks } from 'lucide-vue-next';
import type { TrackColumnId, TrackColumnDef } from './columns';
import { columnCellStyle, columnAlignClass } from './columns';
import { useColumnPrefs } from './useColumnPrefs';

/**
 * 统一歌曲列表表头：与 TrackRow 由同一份列配置驱动（LDL v2 song-row）。
 * 右端的「显示列」菜单提供详细视图开关与可选列的单独增减，偏好保存在本机。
 * 尾部控件宽度与 TrackRow 的行尾占位（trailingWidth）严格配对，保证表头与行对齐。
 */
const props = defineProps<{
  columns: TrackColumnDef[];
  /** 菜单中可操作的列（页面上下文允许的 optional 列）；不传则不显示菜单按钮 */
  menuColumns?: TrackColumnDef[];
  /** 表头尾部批量选择入口（与行尾占位宽度配对） */
  showBatchEntry?: boolean;
  /** 多选模式激活态（高亮批量入口按钮） */
  batchActive?: boolean;
}>();

const emit = defineEmits<{
  toggleBatch: [];
}>();

const prefsApi = useColumnPrefs();
const menuOpen = ref(false);

const hasMenu = computed(() => (props.menuColumns?.length ?? 0) > 0);

function isColumnOn(col: TrackColumnDef): boolean {
  const pref = prefsApi.explicitPref(col.id as TrackColumnId);
  if (pref !== undefined) return pref;
  return prefsApi.prefs.value.detailedView || col.defaultVisible;
}

function onToggleColumn(col: TrackColumnDef) {
  prefsApi.setColumnPref(col.id as TrackColumnId, !isColumnOn(col));
}

function onToggleDetailed() {
  prefsApi.setDetailedView(!prefsApi.prefs.value.detailedView);
}
</script>

<template>
  <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
    <template v-for="col in columns" :key="col.id">
      <div :style="columnCellStyle(col)" :class="columnAlignClass(col)" class="shrink-0 min-w-0 px-0">
        <template v-if="col.id === 'title'"><span class="pl-1">{{ col.label }}</span></template>
        <template v-else-if="col.showLabel !== false">{{ col.label }}</template>
      </div>
    </template>

    <!-- 批量选择入口（w-8 + ml-2 = 40px，与行尾占位配对） -->
    <button
      v-if="showBatchEntry"
      class="ml-2 w-8 shrink-0 flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
      :class="batchActive ? 'text-brand-orange' : ''"
      :title="batchActive ? '退出多选' : '多选歌曲'"
      @click="emit('toggleBatch')"
    >
      <ListChecks class="w-[14px] h-[14px]" />
    </button>

    <!-- 显示列菜单（ml-1 + w-8 = 36px，与行尾占位配对） -->
    <div v-if="hasMenu" class="relative ml-1 w-8 shrink-0 flex justify-center">
      <button
        class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
        :class="menuOpen ? 'text-brand-orange bg-list-hover' : 'text-text-muted hover:text-text-primary hover:bg-list-hover'"
        title="显示列设置"
        @click="menuOpen = !menuOpen"
      >
        <Settings2 class="w-[14px] h-[14px]" />
      </button>

      <!-- 遮罩（点击关闭）+ 弹出菜单 -->
      <div v-if="menuOpen" class="fixed inset-0 z-40" @click="menuOpen = false" />
      <div
        v-if="menuOpen"
        class="absolute right-0 top-full mt-1 z-50 w-[168px] bg-bg-content border border-border-color rounded-[8px] shadow-lg py-1.5"
      >
        <button
          class="w-full flex items-center justify-between px-3 py-1.5 text-[12px] text-text-primary hover:bg-list-hover transition-colors-smooth"
          @click="onToggleDetailed"
        >
          <span>详细视图</span>
          <span
            class="w-[28px] h-[16px] rounded-full relative transition-colors-smooth"
            :class="prefsApi.prefs.value.detailedView ? 'bg-brand-orange' : 'bg-border-solid'"
          >
            <span
              class="absolute top-[2px] w-[12px] h-[12px] rounded-full bg-white shadow transition-all"
              :style="{ left: prefsApi.prefs.value.detailedView ? '14px' : '2px' }"
            />
          </span>
        </button>
        <div class="my-1 border-t border-border-color" />
        <button
          v-for="col in menuColumns"
          :key="col.id"
          class="w-full flex items-center gap-2 px-3 py-1.5 text-[12px] text-text-primary hover:bg-list-hover transition-colors-smooth"
          @click="onToggleColumn(col)"
        >
          <span
            class="w-[13px] h-[13px] rounded-[3px] border flex items-center justify-center text-[9px] leading-none"
            :class="isColumnOn(col) ? 'bg-brand-orange border-brand-orange text-white' : 'border-border-solid text-transparent'"
          >✓</span>
          <span>{{ col.label }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
