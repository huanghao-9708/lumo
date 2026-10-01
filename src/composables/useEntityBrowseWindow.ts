import { computed, nextTick, onBeforeUnmount, onMounted, watch, type Ref } from 'vue';
import { usePlayerStore } from '../stores/player';
import { useDesktopModeStore } from '../stores/desktopMode';
import { resetScrollPosition } from './useScrollRestore';

/** Shared grid/text-list anchor capture and bidirectional result-window loading. */
export function useEntityBrowseWindow(options: {
  kind: 'album' | 'artist' | Ref<'album' | 'artist'>;
  containerRef: Ref<HTMLElement | null>;
  rowHeight: number | Ref<number>;
  columns?: Ref<number>;
  scrollOffset?: Ref<number>;
  scrollKey: () => string;
  enabled?: () => boolean;
}) {
  const player = usePlayerStore();
  const mode = useDesktopModeStore();
  const kind = computed(() => typeof options.kind === 'string' ? options.kind : options.kind.value);
  const enabled = () => options.enabled?.() ?? true;
  const height = () => typeof options.rowHeight === 'number' ? options.rowHeight : options.rowHeight.value;
  const columns = () => Math.max(1, options.columns?.value ?? 1);
  const offset = () => options.scrollOffset?.value ?? 0;
  const windowed = computed(() => enabled() && (kind.value === 'album' ? player.albumsWindowed : player.artistsWindowed));
  const itemOffset = computed(() => windowed.value ? (kind.value === 'album' ? player.albumsWindowStart : player.artistsWindowStart) : 0);
  const totalItems = computed(() => windowed.value ? (kind.value === 'album' ? player.albumsTotalCount : player.artistsTotalCount) : undefined);
  let needsRestore = player.pendingBrowseRestore && enabled();
  let disposed = false;
  let resizeObserver: ResizeObserver | null = null;
  // Rebase by the saved entity, rather than replaying old scroll memory while data is empty.
  if (needsRestore) resetScrollPosition(options.scrollKey());

  async function onScroll(retry = false) {
    const el = options.containerRef.value;
    if (disposed || needsRestore || mode.phase !== 'idle' || !el || el.clientHeight <= 0 || !windowed.value || player.pendingBrowseRestore) return;
    const top = Math.max(0, el.scrollTop - offset());
    const first = Math.floor(top / height()) * columns();
    const end = Math.ceil((top + el.clientHeight) / height()) * columns();
    await player.ensureEntityBrowseWindow(kind.value, first, end, columns(), retry);
  }

  // Native geometry changes before windowForm is committed. Capture the full
  // layout at transaction start, before ResizeObserver can change grid columns.
  watch(() => mode.phase, phase => {
    if (phase === 'idle') { void nextTick(() => onScroll()); return; }
    const el = options.containerRef.value;
    if (phase !== 'entering-mini' || !enabled() || !el || el.clientHeight <= 0) return;
    const top = Math.max(0, el.scrollTop - offset());
    player.recordEntityBrowseAnchor(kind.value, Math.floor(top / height()) * columns(), (top % height()) / height());
  }, { flush: 'sync' });

  watch(() => player.pendingBrowseRestore, async pending => {
    if (pending || !needsRestore || disposed) return;
    const target = player.entityBrowseRestore;
    if (target?.kind !== kind.value) { needsRestore = false; return; }
    await nextTick();
    if (disposed || !options.containerRef.value) return;
    options.containerRef.value.scrollTop = offset() + (Math.floor(target.index / columns()) + target.rowFraction) * height();
    needsRestore = false;
    await onScroll();
  });

  const itemsLength = computed(() => kind.value === 'album' ? player.albums.length : player.artists.length);
  watch([itemsLength, itemOffset, () => player.pendingBrowseRestore], () => { void nextTick(() => onScroll()); });
  onMounted(() => {
    if (typeof ResizeObserver !== 'undefined' && options.containerRef.value) {
      resizeObserver = new ResizeObserver(() => { void onScroll(); });
      resizeObserver.observe(options.containerRef.value);
    }
    void nextTick(() => onScroll());
  });
  onBeforeUnmount(() => { disposed = true; resizeObserver?.disconnect(); });
  return { itemOffset, totalItems, windowed, restoring: computed(() => player.pendingBrowseRestore), onScroll, retry: () => onScroll(true) };
}
