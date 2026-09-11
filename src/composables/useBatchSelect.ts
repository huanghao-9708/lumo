import { computed, reactive, ref } from 'vue';
import type { Track } from '../stores/player';

/**
 * 歌曲列表多选状态机（迭代记录 2.4）。
 *
 * 每个列表视图各持一份实例，不做全局单例——避免跨视图状态串扰。
 * 用 reactive 包装返回，模板里 `batch.isActive` 直接拿到解包值。
 */
export function useBatchSelect() {
  const isActive = ref(false);
  /** 用整集合替换而非原地 mutate，保证依赖它的 computed/视图刷新 */
  const selectedIds = ref<ReadonlySet<number>>(new Set());
  const selectedList = ref<Track[]>([]);

  const count = computed(() => selectedList.value.length);

  /** 进入多选模式 */
  function enter() {
    isActive.value = true;
  }

  /** 退出多选并清空选择 */
  function exit() {
    isActive.value = false;
    selectedIds.value = new Set();
    selectedList.value = [];
  }

  /** 切换一首歌的选中态（保持选择顺序） */
  function toggle(track: Track) {
    const next = new Set(selectedIds.value);
    if (next.has(track.id)) {
      next.delete(track.id);
      selectedList.value = selectedList.value.filter(t => t.id !== track.id);
    } else {
      next.add(track.id);
      selectedList.value = [...selectedList.value, track];
    }
    selectedIds.value = next;
  }

  function isSelected(id: number): boolean {
    return selectedIds.value.has(id);
  }

  /** 全选（以视图当前可见/过滤后的列表为准） */
  function selectAll(list: Track[]) {
    selectedIds.value = new Set(list.map(t => t.id));
    selectedList.value = [...list];
  }

  /** 取消全选（保持多选模式） */
  function selectNone() {
    selectedIds.value = new Set();
    selectedList.value = [];
  }

  return reactive({
    isActive,
    selectedIds,
    selectedList,
    count,
    enter,
    exit,
    toggle,
    isSelected,
    selectAll,
    selectNone,
  });
}

export type BatchSelect = ReturnType<typeof useBatchSelect>;
