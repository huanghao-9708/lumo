import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue';
import type { TrackListContext } from './columns';
import { computeTrailingExtraWidth, resolveTrackColumns } from './columns';
import { resolveTrackColumnsWithPrefs, useColumnPrefs } from './useColumnPrefs';

/**
 * 第 2 层「列表外壳」的列解析入口：
 * 用 ResizeObserver 测量列表容器的实际宽度（而非浏览器视口），
 * 结合页面上下文与用户偏好，解析出当前应渲染的列集合。
 *
 * 这取代了旧实现里按浏览器视口 `sm/md/lg` 隐藏列的做法——
 * 右侧 Inspector 打开时视口宽度不变，但内容区变窄，列应随之收纳。
 */
export function useTrackColumns(options: {
  /** 列表滚动容器（宽度基准） */
  containerRef: Ref<HTMLElement | null>;
  /** 页面上下文（hidden/pinned 可响应） */
  context?: Ref<TrackListContext> | (() => TrackListContext);
}) {
  const containerWidth = ref(0);
  const prefsApi = useColumnPrefs();
  let observer: ResizeObserver | null = null;
  watch(options.containerRef, (el) => {
    if (observer) {
      observer.disconnect();
      observer = null;
    }
    if (!el) return;
    containerWidth.value = el.clientWidth;
    if (typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          containerWidth.value = entry.contentRect.width;
        }
      });
      observer.observe(el);
    }
  }, { immediate: true });

  onBeforeUnmount(() => {
    observer?.disconnect();
    observer = null;
  });

  const context = computed<TrackListContext>(() => {
    const raw = options.context;
    if (!raw) return {};
    return typeof raw === 'function' ? raw() : raw.value;
  });

  /** 「显示列」菜单里可操作的列（有定义的 optional 列） */
  const menuColumns = computed(() =>
    resolveTrackColumns(Number.MAX_SAFE_INTEGER, context.value)
      .filter(c => c.optional),
  );

  /**
   * 表头尾部额外宽度：TrackRow 需要渲染等宽的行尾占位来对齐表头
   * （批量入口 40 + 显示列菜单 36，见 computeTrailingExtraWidth）。
   */
  const trailingExtraWidth = computed(() =>
    computeTrailingExtraWidth(context.value.batchEntry === true, menuColumns.value.length > 0),
  );

  const resolvedColumns = computed(() =>
    resolveTrackColumnsWithPrefs(
      containerWidth.value,
      context.value,
      prefsApi.prefs.value,
      trailingExtraWidth.value,
    )
  );

  return {
    containerWidth,
    resolvedColumns,
    menuColumns,
    trailingExtraWidth,
    ...prefsApi,
  };
}
