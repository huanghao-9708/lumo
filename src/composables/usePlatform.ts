import { ref, computed, onMounted, onUnmounted } from 'vue';

/**
 * 响应式平台检测。
 *
 * 断点策略（与 LDL 02-spatial 响应式断点一致）：
 *   Android    → 移动端（无论视口宽度，平板/横屏也不落入鼠标导向的桌面布局）
 *   < 768px    → 移动端（MobileLayout：单栏 + Tab Bar）
 *   ≥ 768px    → 桌面端（DesktopLayout：五区工作台）
 *
 * 768px 是 Tailwind `md` 断点，也是 LDL 定义的 Song Row 显示艺术家列的阈值。
 */
const MOBILE_BREAKPOINT = 768;

export const isAndroid =
  typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent);

export function usePlatform() {
  const viewportWidth = ref(
    typeof window !== 'undefined' ? window.innerWidth : 1280
  );

  function update() {
    viewportWidth.value = window.innerWidth;
  }

  onMounted(() => {
    window.addEventListener('resize', update);
  });

  onUnmounted(() => {
    window.removeEventListener('resize', update);
  });

  // ADR-5：Android 强制移动布局；桌面维持 768px 断点
  const isMobile = computed(() => isAndroid || viewportWidth.value < MOBILE_BREAKPOINT);

  return { isMobile };
}
