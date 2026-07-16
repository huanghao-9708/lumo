import { ref, onMounted, onUnmounted } from 'vue';

/**
 * 响应式平台检测。
 *
 * 断点策略（与 LDL 02-spatial 响应式断点一致）：
 *   < 768px  → 移动端（MobileLayout：单栏 + Tab Bar）
 *   ≥ 768px  → 桌面端（DesktopLayout：五区工作台）
 *
 * 768px 是 Tailwind `md` 断点，也是 LDL 定义的 Song Row 显示艺术家列的阈值。
 * 移动端布局从 md 以下接管。
 */
const MOBILE_BREAKPOINT = 768;

export function usePlatform() {
  const isMobile = ref(
    typeof window !== 'undefined' ? window.innerWidth < MOBILE_BREAKPOINT : false
  );

  function update() {
    isMobile.value = window.innerWidth < MOBILE_BREAKPOINT;
  }

  onMounted(() => {
    window.addEventListener('resize', update);
  });

  onUnmounted(() => {
    window.removeEventListener('resize', update);
  });

  return { isMobile };
}
