import { ref, computed, type Ref, onMounted, onUnmounted, watch } from 'vue';

/**
 * 轻量级虚拟列表 composable（零依赖）。
 *
 * 解决的问题：当列表项数量较大（数百~数千）时，一次性把所有项挂到 DOM 会导致
 * 渲染卡顿、滚动掉帧。虚拟列表只渲染可视窗口内的项 + 上下 buffer，其余用空白占位。
 *
 * 使用前提：列表项行高固定（或可按"每行固定、多列"近似）。
 *
 * 典型用法：
 * ```vue
 * const containerRef = ref<HTMLElement | null>(null);
 * const { totalHeight, translateY, visibleItems } = useVirtualList({
 *   containerRef,
 *   items: allItems,
 *   itemHeight: 64,   // 每行 64px
 *   buffer: 6,        // 上下各多渲染 6 行
 * });
 * ```
 */
export interface UseVirtualListOptions {
  /** 滚动容器的 ref（必须是 `overflow-y: auto` 的元素） */
  containerRef: Ref<HTMLElement | null>;
  /** 完整数据数组（响应式） */
  items: Ref<readonly any[]>;
  /** 单行高度（像素）。对于多列网格，传"行高"，并把 `columns` 设为对应列数 */
  itemHeight: number;
  /** 上下额外渲染的缓冲行数，避免滚动时边缘闪现 */
  buffer?: number;
  /** 网格列数。列表传 1（默认）。专辑网格传 2/3/4 等。
   *  此时 `items` 仍是扁平数组，内部按列数切片为行。
   *  支持传 Ref<number> 以响应容器宽度变化（动态列数）。 */
  columns?: number | Ref<number>;
}

export interface VirtualListItem<T = any> {
  /** 在原数组中的索引 */
  index: number;
  /** 原始数据 */
  data: T;
}

export interface UseVirtualListReturn<T = any> {
  /** 占位容器的总高度（px）。容器用这个高度撑出滚动条 */
  totalHeight: Ref<number>;
  /** 可视项集合需要向下平移的偏移量（px）。配合 transform: translateY 使用 */
  offsetY: Ref<number>;
  /** 当前应该渲染的项集合（已做 buffer 处理） */
  visibleItems: Ref<VirtualListItem<T>[]>;
}

export function useVirtualList<T = any>(options: UseVirtualListOptions): UseVirtualListReturn<T> {
  const { containerRef, items, itemHeight, buffer = 6 } = options;
  // columns 支持 number 或 Ref<number>，统一解包为 computed
  const columnsRef = typeof options.columns === 'object' && options.columns !== null && 'value' in options.columns
    ? computed(() => (options.columns as Ref<number>).value || 1)
    : computed(() => (options.columns as number | undefined) ?? 1);
  const safeItemHeight = Math.max(1, itemHeight);

  const startIndex = ref(0);
  const endIndex = ref(0);
  const viewportHeight = ref(0);

  // 真试行数 = ceil(项数 / 列数)
  const rowCount = computed(() => Math.ceil(items.value.length / Math.max(1, columnsRef.value)));
  const totalHeight = computed(() => rowCount.value * safeItemHeight);

  // 计算可视项。注意 startIndex/endIndex 是"行"的下标，转成项下标时要乘列数。
  const visibleItems = computed<VirtualListItem<T>[]>(() => {
    const cols = Math.max(1, columnsRef.value);
    const startRow = Math.max(0, startIndex.value);
    const endRow = Math.min(rowCount.value, endIndex.value);
    const startIdx = startRow * cols;
    const endIdx = Math.min(items.value.length, endRow * cols);
    const out: VirtualListItem<T>[] = [];
    for (let i = startIdx; i < endIdx; i++) {
      out.push({ index: i, data: items.value[i] });
    }
    return out;
  });

  // 可视项容器的 translateY：把渲染出的项整体下推到正确位置
  const offsetY = computed(() => Math.max(0, startIndex.value) * safeItemHeight);

  // 根据当前 scrollTop 重新计算窗口范围
  function recalc() {
    const el = containerRef.value;
    if (!el) return;
    const scrollTop = el.scrollTop;
    const viewH = el.clientHeight;
    viewportHeight.value = viewH;

    const startRow = Math.max(0, Math.floor(scrollTop / safeItemHeight) - buffer);
    const visibleRows = Math.ceil(viewH / safeItemHeight) + buffer * 2;
    const endRow = startRow + visibleRows;

    startIndex.value = startRow;
    endIndex.value = endRow;
  }

  // 节流：滚动事件触发非常频繁，用 rAF 合并到下一帧统一计算
  let ticking = false;
  function onScroll() {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      recalc();
      ticking = false;
    });
  }

  // 当容器尺寸变化时也要重算（窗口缩放、侧栏开合等）
  let resizeObserver: ResizeObserver | null = null;

  onMounted(() => {
    const el = containerRef.value;
    if (!el) return;
    el.addEventListener('scroll', onScroll, { passive: true });
    // 监听容器尺寸变化，保证 viewportHeight 同步
    if (typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(() => recalc());
      resizeObserver.observe(el);
    }
    recalc();
  });

  onUnmounted(() => {
    const el = containerRef.value;
    if (el) el.removeEventListener('scroll', onScroll);
    if (resizeObserver) {
      resizeObserver.disconnect();
      resizeObserver = null;
    }
  });

  // 数据变化时重算窗口（例如加载更多、切换 tab）
  watch(() => items.value.length, () => {
    requestAnimationFrame(recalc);
  });

  // 列数变化时（容器宽度改变导致重算列数）也要重算窗口
  watch(columnsRef, () => {
    requestAnimationFrame(recalc);
  });

  return { totalHeight, offsetY, visibleItems };
}
