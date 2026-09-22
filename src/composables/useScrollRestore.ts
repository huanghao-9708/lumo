import { ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue';

/**
 * 滚动位置记忆（跨组件挂载/卸载保持）。
 *
 * 背景：列表视图（艺术家网格 / 专辑网格 / 艺人详情 …）由 MainContent 用 v-if 切换，
 * 进入详情页时列表组件被销毁、返回时重新创建，滚动位置随之归零。
 * 这里用模块级 Map 把 scrollTop 记在视图 key 上，组件重建后自动写回。
 *
 * 用法：
 *   const scrollEl = useScrollRestore(() => `album-grid:${keyword}`);
 *   <div ref="scrollEl" class="overflow-y-auto">
 *
 * 注意：
 * - key 必须能唯一标识一个「列表内容」。同一组件实例切换数据（如换一个艺人）时，
 *   key 变化会先保存旧 key 的位置，再恢复新 key 的位置。
 * - 恢复时内容可能是异步渲染的（分页数据刚回来），因此用 rAF 轮询等待
 *   scrollHeight 足够高再落位，最多重试 MAX_TRIES 帧后放弃（此时写值会被浏览器钳制）。
 */

/** 模块级缓存：视图 key → scrollTop */
const positions = new Map<string, number>();

/** 单次恢复最多等待的帧数（约 1.5s @60fps；详情页数据异步到位耗时更长也不至于放弃） */
const MAX_TRIES = 90;

/** 清除指定 key（或全部）的记忆位置 */
export function resetScrollPosition(key?: string) {
  if (key === undefined) positions.clear();
  else positions.delete(key);
}

export function useScrollRestore(keySource: () => string) {
  const el = ref<HTMLElement | null>(null);
  let raf = 0;
  let currentKey = '';
  let restoreSeq = 0;

  /** 把当前滚动位置写回当前 key */
  function save() {
    const node = el.value;
    if (node && currentKey) positions.set(currentKey, node.scrollTop);
  }

  /** 把 key 对应的历史位置写回 DOM（等待内容高度足够） */
  function restore() {
    currentKey = keySource();
    const seq = ++restoreSeq;
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    const target = positions.get(currentKey) ?? 0;
    if (target <= 0) return;

    let tries = 0;
    const step = () => {
      // 期间又切换了 key / 已卸载 → 放弃本次恢复
      if (seq !== restoreSeq) return;
      const node = el.value;
      if (!node) return;
      const max = node.scrollHeight - node.clientHeight;
      if (max >= target || tries++ >= MAX_TRIES) {
        node.scrollTop = target;
        return;
      }
      // 内容还不够高：先顶到当前底部，触发列表的懒加载，下一帧再看
      node.scrollTop = Math.max(0, max);
      raf = requestAnimationFrame(step);
    };
    step();
  }

  onMounted(() => {
    nextTick(restore);
  });

  onBeforeUnmount(() => {
    if (raf) cancelAnimationFrame(raf);
    save();
  });

  // 同一实例内切换数据源（换艺人 / 换关键词）时：存旧的，恢复新的
  watch(keySource, () => {
    save();
    nextTick(restore);
  });

  // 容器可能晚于组件挂载才出现（详情数据异步到位后 v-else 分支才渲染），
  // 此时 onMounted 里的恢复会因为拿不到元素而静默失效，这里补一次。
  watch(el, (node) => {
    if (node) nextTick(restore);
  });

  return el;
}
