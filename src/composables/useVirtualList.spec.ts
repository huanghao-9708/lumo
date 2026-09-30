import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { useVirtualList } from './useVirtualList';

describe('虚拟网格内存边界', () => {
  let wrapper: VueWrapper;
  let resize: () => void;
  const disconnect = vi.fn();
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;

  beforeEach(() => {
    frames.clear();
    disconnect.mockClear();
    vi.stubGlobal('ResizeObserver', class {
      constructor(callback: () => void) { resize = callback; }
      observe() {}
      disconnect = disconnect;
    });
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.set(++frameId, callback);
      return frameId;
    });
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
  });
  afterEach(() => {
    wrapper?.unmount();
    vi.unstubAllGlobals();
  });

  async function fixture(count = 3819) {
    const container = ref<HTMLElement | null>(null);
    const items = ref(Array.from({ length: count }, (_, id) => ({ id })));
    const rowHeight = ref(100);
    const columns = ref(3);
    let list!: ReturnType<typeof useVirtualList>;
    wrapper = mount(defineComponent({ setup() {
      list = useVirtualList({ containerRef: container, items, itemHeight: rowHeight, columns, buffer: 1 });
      return () => h('div', { ref: container });
    } }));
    await nextTick();
    Object.defineProperty(container.value, 'clientHeight', { value: 255, configurable: true });
    resize();
    return { container, items, rowHeight, columns, list };
  }

  it('3,819 张专辑仍只持有视口和各一行缓冲，并覆盖半行边界', async () => {
    const { container, list } = await fixture();
    expect(list.visibleItems.value).toHaveLength(12);
    container.value!.scrollTop = 50;
    resize();
    expect(list.visibleItems.value[list.visibleItems.value.length - 1]?.index).toBe(14);
    container.value!.scrollTop = 550;
    resize();
    expect(list.visibleItems.value[0].index).toBe(12);
    expect(list.visibleItems.value[list.visibleItems.value.length - 1]?.index).toBe(29);
    expect(list.visibleItems.value.length).toBeLessThanOrEqual(18);
  });

  it('更换列数、缩小列表和恢复旧滚动位置时不会形成空白窗口', async () => {
    const { container, items, columns, rowHeight, list } = await fixture();
    container.value!.scrollTop = 10000;
    resize();
    items.value = items.value.slice(0, 5);
    columns.value = 2;
    rowHeight.value = 150;
    await nextTick();
    expect(list.visibleItems.value.length).toBeGreaterThan(0);
    expect(list.visibleItems.value[list.visibleItems.value.length - 1]?.index).toBe(4);
    expect(list.totalHeight.value).toBe(450);
  });

  it('合并连续滚动帧，卸载后取消待处理帧并释放观察器', async () => {
    const { container } = await fixture();
    container.value!.dispatchEvent(new Event('scroll'));
    container.value!.dispatchEvent(new Event('scroll'));
    expect(frames.size).toBe(1);
    wrapper.unmount();
    expect(frames.size).toBe(0);
    expect(disconnect).toHaveBeenCalledTimes(1);
  });
});
