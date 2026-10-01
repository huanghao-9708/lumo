import { onBeforeUnmount, watch } from 'vue';
import { useUiStore } from '../stores/ui';

/** Keep drafts and sensitive operations mounted until their owner is ready to leave. */
export function useMiniModeBlocker(reason: () => string) {
  const ui = useUiStore();
  const key = Symbol('mini-mode-block');
  watch(reason, value => ui.setMiniModeBlock(key, value), { immediate: true, flush: 'sync' });
  onBeforeUnmount(() => ui.setMiniModeBlock(key, ''));
}
