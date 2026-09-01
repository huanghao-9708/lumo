import { nextTick, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '../utils/tauriInvoke';
import { usePlayerStore } from '../stores/player';
import { useUiStore } from '../stores/ui';

/**
 * MA1（A1-7）：Android 返回导航。
 *
 * 返回键由 MainActivity 拦截并回注 `lumo-back-pressed` 事件，处理优先级：
 *   1. registerBackHandler 注册的临时拦截器（弹层/添加来源页等，返回 true 表示已消费）
 *   2. NowPlaying 沉浸视图 → 关闭
 *   3. 视图状态快照栈（uiStore.activeMobileTab + 曲库内部导航层级）逐级回退
 *   4. 栈到底 → platform_finish_app 退出应用
 */

interface NavSnapshot {
  mobileTab: string;
  libraryTab: string;
  albumId: number | null;
  artistId: number | null;
  playlistId: number | null;
}

type BackHandler = () => boolean;

const customHandlers: BackHandler[] = [];

/** 注册临时返回拦截器（如全屏弹层）。返回取消函数。 */
export function registerBackHandler(h: BackHandler): () => void {
  customHandlers.push(h);
  return () => {
    const i = customHandlers.indexOf(h);
    if (i >= 0) customHandlers.splice(i, 1);
  };
}

let initialized = false;

export async function initMobileBackNavigation(): Promise<void> {
  if (initialized) return;
  initialized = true;

  const playerStore = usePlayerStore();
  const uiStore = useUiStore();

  const snapshot = (): NavSnapshot => ({
    mobileTab: uiStore.activeMobileTab,
    libraryTab: playerStore.activeLibraryTab,
    albumId: playerStore.activeAlbumId,
    artistId: playerStore.activeArtistId,
    playlistId: playerStore.activePlaylistId,
  });

  const restore = (s: NavSnapshot) => {
    uiStore.setMobileTab(s.mobileTab as typeof uiStore.activeMobileTab);
    playerStore.activeLibraryTab = s.libraryTab;
    playerStore.activeAlbumId = s.albumId;
    playerStore.activeArtistId = s.artistId;
    playerStore.activePlaylistId = s.playlistId;
  };

  const stack: NavSnapshot[] = [snapshot()];
  let suppress = false;

  // 导航状态变化 → 压栈（连续相同状态去重）
  watch(
    () => [
      uiStore.activeMobileTab,
      playerStore.activeLibraryTab,
      playerStore.activeAlbumId,
      playerStore.activeArtistId,
      playerStore.activePlaylistId,
    ],
    () => {
      if (suppress) return;
      const s = snapshot();
      const last = stack[stack.length - 1];
      if (JSON.stringify(s) === JSON.stringify(last)) return;
      stack.push(s);
    },
  );

  await listen('lumo-back-pressed', () => {
    for (let i = customHandlers.length - 1; i >= 0; i--) {
      if (customHandlers[i]()) return;
    }

    if (uiStore.isImmersiveView) {
      uiStore.closeImmersiveView();
      return;
    }

    if (stack.length > 1) {
      stack.pop();
      const prev = stack[stack.length - 1];
      // restore 写回会再次触发 watch，压栈一拍内屏蔽
      suppress = true;
      restore(prev);
      nextTick(() => {
        suppress = false;
      });
    } else {
      invoke('platform_finish_app').catch(() => {});
    }
  });
}
