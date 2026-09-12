/**
 * 滚动条 auto-hide 的"滚动中"信号（迭代记录 2.5，第四轮）。
 *
 * 单一全局 scroll 监听（capture 捕获阶段，scroll 事件不冒泡但可捕获；
 * passive 不阻塞滚动）：给触发滚动的 `.overflow-y-auto` / `.overflow-x-auto`
 * 容器临时加 `.scrolling` 类——滚动中滚动条立即可见（不论鼠标是否 hover），
 * 停止滚动 600ms 后移除、thumb 淡出。配合 style.css「Auto-Hide Scrollbar」块。
 *
 * 在 App.vue 调用一次即可。WeakMap 维护每个容器的 setTimeout id，无需手动清理。
 */
export function useAutoScrollbar() {
  const timers = new WeakMap<HTMLElement, ReturnType<typeof setTimeout>>();

  document.addEventListener('scroll', (e) => {
    const target = e.target;
    if (!(target instanceof HTMLElement)) return;
    if (!target.classList.contains('overflow-y-auto') && !target.classList.contains('overflow-x-auto')) return;

    target.classList.add('scrolling');
    const prev = timers.get(target);
    if (prev) clearTimeout(prev);
    timers.set(target, setTimeout(() => target.classList.remove('scrolling'), 600));
  }, { capture: true, passive: true });
}
