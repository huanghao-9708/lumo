import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useDesktopModeStore } from '../stores/desktopMode';

/**
 * 响应式平台检测。
 *
 * 断点策略（与 LDL 02-spatial 响应式断点一致）：
 *   Android    → 移动端（无论视口宽度，平板/横屏也不落入鼠标导向的桌面布局）
 *   原生桌面   → 桌面端（不因迷你窗口宽度切换到移动布局）
 *   浏览器预览 → 以 768px 为移动/桌面布局断点，迷你形态优先
 *
 * 768px 是 Tailwind `md` 断点，也是 LDL 定义的 Song Row 显示艺术家列的阈值。
 */
const MOBILE_BREAKPOINT = 768;

export const isAndroid =
  typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent);

export function usePlatform() {
  const desktopMode = useDesktopModeStore();
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
  const nativeDesktop = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__ && !isAndroid;
  const isMobile = computed(() => isAndroid || (!nativeDesktop && desktopMode.windowForm !== 'mini' && viewportWidth.value < MOBILE_BREAKPOINT));

  return { isMobile };
}
