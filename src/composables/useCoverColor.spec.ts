import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { createPinia, setActivePinia } from 'pinia';
import { useCoverColor, type CoverColorResult } from './useCoverColor';

describe('封面取色资源释放', () => {
  let wrapper: VueWrapper;
  const images: FakeImage[] = [];
  class FakeImage {
    src = '';
    crossOrigin = '';
    onload: (() => void) | null = null;
    onerror: (() => void) | null = null;
    constructor() { images.push(this); }
  }
  beforeEach(() => {
    // DM-03：useCoverColor 依赖桌面模式 store（visualAllowed 门禁），默认 normal+full 恒放行
    setActivePinia(createPinia());
    images.length = 0;
    vi.stubGlobal('Image', FakeImage);
  });
  afterEach(() => { wrapper?.unmount(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
  function fixture() {
    const src = ref<string | null>('first-thumb');
    let colors!: CoverColorResult;
    wrapper = mount(defineComponent({ setup() {
      colors = useCoverColor(() => src.value);
      return () => h('div');
    } }));
    return { src, colors };
  }

  it('切换封面取消旧图，卸载取消新图，并清空事件回调', async () => {
    const { src, colors } = fixture();
    expect(images[0].crossOrigin).toBe('anonymous');
    src.value = 'second-thumb';
    await nextTick();
    expect(images[0].src).toBe('');
    expect(images[0].onload).toBeNull();
    expect(images[0].onerror).toBeNull();
    expect(images[1].src).toBe('second-thumb');
    wrapper.unmount();
    await flushPromises();
    expect(images[1].src).toBe('');
    expect(images[1].onload).toBeNull();
    expect(colors.ready.value).toBe(false);
  });

  it.each([false, true])('取色完成或读取失败后释放 canvas 和图片（失败=%s）', async (fail) => {
    const canvases: HTMLCanvasElement[] = [];
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(function (this: HTMLCanvasElement) {
      canvases.push(this);
      return {
        drawImage: vi.fn(),
        getImageData: () => {
          if (fail) throw new Error('unreadable pixels');
          return { data: new Uint8ClampedArray([220, 80, 20, 255]) };
        },
      } as unknown as CanvasRenderingContext2D;
    });
    const { colors } = fixture();
    images[0].onload!();
    await flushPromises();
    expect(colors.ready.value).toBe(!fail);
    expect(images[0].src).toBe('');
    expect(images[0].onload).toBeNull();
    expect(canvases[0].width).toBe(0);
    expect(canvases[0].height).toBe(0);
  });

  it('DM-03 门禁：visualAllowed=false 不发起取色，策略切换立即清空', async () => {
    const { useDesktopModeStore } = await import('../stores/desktopMode');
    const desktopMode = useDesktopModeStore();
    desktopMode.windowForm = 'mini'; // visualAllowed=false

    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(() => ({
      drawImage: vi.fn(),
      getImageData: () => ({ data: new Uint8ClampedArray([220, 80, 20, 255]) }),
    }) as unknown as CanvasRenderingContext2D);

    const src = ref<string | null>('thumb-1');
    let colors!: CoverColorResult;
    wrapper = mount(defineComponent({ setup() {
      colors = useCoverColor(() => src.value);
      return () => h('div');
    } }));
    await nextTick();

    // 迷你态：不创建任何图片请求（取色任务为 0 增量）
    expect(images.length).toBe(0);
    expect(colors.ready.value).toBe(false);

    // 切回正常×完整：策略放行，同一 src 重新发起取色
    desktopMode.windowForm = 'full';
    await nextTick();
    expect(images.length).toBe(1);
    expect(images[0].src).toBe('thumb-1');

    // 策略再切回不允许：立即清空既有颜色（生效策略先于在途结果）
    images[0].onload!();
    await flushPromises();
    expect(colors.ready.value).toBe(true);
    desktopMode.windowForm = 'mini';
    await nextTick();
    expect(colors.ready.value).toBe(false);
    expect(colors.primary.value).toBe('');
    expect(images[0].src).toBe('');
  });
});
