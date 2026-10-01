<script setup lang="ts">
import { computed, ref, onBeforeUnmount, nextTick } from 'vue';
import { Play, Pause, SkipBack, SkipForward, Maximize2, MoreHorizontal, X, Minus, Pin } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { usePlayerStore } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';
import { useUiStore } from '../../stores/ui';

const player = usePlayerStore();
const mode = useDesktopModeStore();
const ui = useUiStore();
const menuOpen = ref(false);
const menuRoot = ref<HTMLElement | null>(null);
const menuButton = ref<HTMLButtonElement | null>(null);
const draftProgress = ref<number | null>(null);
const progress = computed(() => draftProgress.value ?? player.progressMs);
const track = computed(() => player.currentTrack);
const busy = computed(() => mode.phase !== 'idle');
const status = computed(() => player.playbackError || ui.toast?.message || (mode.prefsDirty ? '偏好未保存' : player.isBuffering ? '正在缓冲…' : player.isPlaying ? '正在播放' : '已暂停'));
const formatMs = (ms: number) => {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
};
function commitSeek(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  draftProgress.value = null;
  void player.seek(value);
}
function closeMenu(focus = true) {
  menuOpen.value = false;
  if (focus) menuButton.value?.focus();
}
async function toggleMenu() {
  if (menuOpen.value) return closeMenu();
  menuOpen.value = true;
  await nextTick();
  menuRoot.value?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
}
function menuKeys(e: KeyboardEvent) {
  if (e.key === 'Escape') { e.preventDefault(); closeMenu(); return; }
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return;
  e.preventDefault();
  const items = Array.from(menuRoot.value?.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]') ?? []);
  const i = items.indexOf(document.activeElement as HTMLButtonElement);
  const next = e.key === 'Home' ? 0 : e.key === 'End' ? items.length - 1 : (i + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
  items[next]?.focus();
}
function outsideClick(event: MouseEvent) {
  if (menuOpen.value && !menuRoot.value?.contains(event.target as Node)) closeMenu(false);
}
document.addEventListener('click', outsideClick);
onBeforeUnmount(() => document.removeEventListener('click', outsideClick));
const nativeWindow = () => typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
function minimize() { if (nativeWindow()) void getCurrentWindow().minimize(); closeMenu(false); }
function closeWindow() { if (nativeWindow()) void getCurrentWindow().close(); }
</script>

<template>
  <main class="mini-player" aria-label="迷你播放栏">
    <div class="mini-top">
      <div class="mini-info" data-tauri-drag-region :title="track ? `${track.title} · ${track.artist}` : 'Lumo'">
        <strong data-tauri-drag-region>{{ track?.title || 'Lumo · 队列为空' }}</strong>
        <span data-tauri-drag-region>{{ track?.artist || '返回完整界面选择音乐' }}</span>
      </div>
      <div class="mini-controls">
        <button aria-label="上一首" title="上一首" :disabled="!player.sessionQueueLength" @click="player.prevTrack()"><SkipBack /></button>
        <button class="mini-play" :aria-label="player.isPlaying ? '暂停' : '播放'" :title="player.isPlaying ? '暂停' : '播放'" :disabled="!player.sessionQueueLength" @click="player.togglePlay()"><Pause v-if="player.isPlaying" /><Play v-else /></button>
        <button aria-label="下一首" title="下一首" :disabled="!player.sessionQueueLength" @click="player.nextTrack()"><SkipForward /></button>
        <button aria-label="返回完整界面" title="返回完整界面" :disabled="busy" @click="mode.exitMini()"><Maximize2 /></button>
        <div ref="menuRoot" class="mini-menu-root">
          <button ref="menuButton" aria-label="更多操作" title="更多操作" aria-haspopup="menu" :aria-expanded="menuOpen" @click="toggleMenu" @keydown.down.prevent="toggleMenu"><MoreHorizontal /></button>
          <div v-if="menuOpen" class="mini-menu" role="menu" aria-label="迷你播放栏选项" @keydown="menuKeys">
            <button role="menuitemcheckbox" :aria-checked="mode.miniAlwaysOnTop" :disabled="busy" @click="mode.setMiniAlwaysOnTop(!mode.miniAlwaysOnTop)"><Pin />{{ mode.miniAlwaysOnTop ? '取消置顶' : '窗口置顶' }}</button>
            <button role="menuitem" :disabled="busy" @click="mode.setExperienceMode(mode.experienceMode === 'normal' ? 'minimal' : 'normal')">返回时：{{ mode.experienceMode === 'normal' ? '正常模式' : '极简模式' }}</button>
            <button role="menuitem" @click="ui.toggleDarkMode()">{{ ui.isDarkMode ? '切换日间主题' : '切换夜间主题' }}</button>
            <button role="menuitem" @click="minimize"><Minus />最小化</button>
            <button role="menuitem" @click="closeWindow"><X />关闭</button>
          </div>
        </div>
      </div>
    </div>
    <div class="mini-bottom">
      <span class="mini-time">{{ formatMs(progress) }} / {{ formatMs(player.durationMs) }}</span>
      <input type="range" aria-label="播放进度" :aria-valuetext="formatMs(progress)" min="0" :max="Math.max(1, player.durationMs)" step="1000" :value="progress" :disabled="!track || player.durationMs <= 0" @input="draftProgress = Number(($event.target as HTMLInputElement).value)" @change="commitSeek" />
      <label class="mini-volume">音量<input type="range" aria-label="音量" min="0" max="100" step="1" :value="player.volume" @change="player.setVolume(Number(($event.target as HTMLInputElement).value))" /></label>
      <span class="mini-status" :class="{ 'mini-error': player.playbackError }" :title="status" role="status">{{ status }}</span>
    </div>
  </main>
</template>

<style scoped>
.mini-player { height: 100%; width: 100%; padding: 8px 10px; box-sizing: border-box; background: var(--bg-content); color: var(--text-primary); display: flex; flex-direction: column; justify-content: center; gap: 6px; }
.mini-top, .mini-controls, .mini-bottom, .mini-volume { display: flex; align-items: center; }
.mini-top { gap: 8px; }
.mini-info { flex: 1; min-width: 0; cursor: default; }
.mini-info strong, .mini-info span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mini-info strong { font-size: 13px; font-weight: 600; }
.mini-info span { font-size: 11px; color: var(--text-secondary); }
.mini-controls { flex-shrink: 0; gap: 2px; }
button { display: flex; align-items: center; justify-content: center; width: 32px; height: 32px; border-radius: 6px; cursor: pointer; }
button:hover, button:focus-visible { background: var(--bg-hover); }
button:focus-visible, input:focus-visible { outline: 2px solid var(--brand-orange); outline-offset: 1px; }
button:disabled, input:disabled { opacity: .4; cursor: default; }
button svg { width: 16px; height: 16px; flex-shrink: 0; }
.mini-play { color: var(--brand-orange); }
.mini-bottom { gap: 8px; min-width: 0; font-size: 10px; color: var(--text-secondary); }
.mini-time { font-variant-numeric: tabular-nums; white-space: nowrap; }
input { min-width: 0; flex: 1; height: 24px; accent-color: var(--brand-orange); }
.mini-volume { gap: 4px; width: 82px; flex-shrink: 0; }
.mini-status { width: 64px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; flex-shrink: 0; }
.mini-error { color: var(--brand-orange); }
.mini-menu-root { position: relative; }
.mini-menu { position: fixed; right: 8px; top: 6px; z-index: 20; max-height: calc(100vh - 12px); width: 192px; overflow-y: auto; background: var(--bg-content); border: 1px solid var(--border-color); border-radius: 8px; padding: 4px; }
.mini-menu button { width: 100%; gap: 8px; justify-content: flex-start; padding: 0 8px; font-size: 12px; white-space: nowrap; }
</style>
