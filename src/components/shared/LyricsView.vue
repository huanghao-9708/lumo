<script setup lang="ts">
import { ref, watch, nextTick, computed, onMounted, onBeforeUnmount } from 'vue';
import { usePlayerStore } from '../../stores/player';

/**
 * 可复用的同步歌词视图。
 *
 * - 自动平滑滚动到当前播放行（自绘 rAF 动画，见下方说明）
 * - 点击任意行 seek 到该行时间
 * - 当前行 / 已唱行 / 未唱行三态着色
 *
 * variant：
 *   - 'sidebar'    右侧 Inspector 面板用，紧凑字号（13px）
 *   - 'immersive'  沉浸式播放页用，更大字号（17px）、更松行距、居中对齐
 *
 * 原行为来自 SidebarRight.vue 的歌词块，抽到此处共享。
 */
const props = withDefaults(defineProps<{ variant?: 'sidebar' | 'immersive' }>(), {
  variant: 'sidebar',
});

const playerStore = usePlayerStore();

const lyricsContainer = ref<HTMLElement | null>(null);

/* ===================== 平滑滚动 =====================
 *
 * 为什么不用 `scrollIntoView({ behavior: 'smooth' })`：
 *   1. 副歌那种连续的短句，行间跳变很快，原生动画被下一次调用打断时会直接"跳"过去，
 *      看起来就是生硬地闪一下；
 *   2. 无法干预它的滚动条表现——浏览器把程序化滚动同样算作"正在滚动"，
 *      全局 auto-hide 逻辑会给容器加 .scrolling，thumb 无预兆地闪出来。
 * 自绘 rAF 动画可以：从当前位置续接（不跳）、按距离给时长、并在动画期间
 * 给容器打上 data-suppress-scrollbar，让滚动条保持隐藏。
 */
const SCROLL_MIN_MS = 260;
const SCROLL_MAX_MS = 620;
/** 动画结束后再保留一小段抑制窗口：最后一次 scrollTop 写入的 scroll 事件是异步派发的 */
const SUPPRESS_TAIL_MS = 160;

let rafId = 0;
let suppressTimer: ReturnType<typeof setTimeout> | null = null;

function releaseSuppress() {
  if (suppressTimer) {
    clearTimeout(suppressTimer);
    suppressTimer = null;
  }
  const el = lyricsContainer.value;
  if (el?.dataset.suppressScrollbar) delete el.dataset.suppressScrollbar;
}

function scheduleSuppressRelease() {
  if (suppressTimer) clearTimeout(suppressTimer);
  suppressTimer = setTimeout(releaseSuppress, SUPPRESS_TAIL_MS);
}

function stopAnimation() {
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
  releaseSuppress();
}

/** 当前行中线对齐到容器中线的目标 scrollTop（用 rect 算，不依赖 offsetParent） */
function activeLineTargetTop(sc: HTMLElement, line: HTMLElement): number {
  const scRect = sc.getBoundingClientRect();
  const lineRect = line.getBoundingClientRect();
  const delta = (lineRect.top + lineRect.height / 2) - (scRect.top + sc.clientHeight / 2);
  const max = Math.max(0, sc.scrollHeight - sc.clientHeight);
  return Math.min(Math.max(0, sc.scrollTop + delta), max);
}

/** 立即定位（不播动画），用于换歌回到开头 */
function jumpTo(top: number) {
  const sc = lyricsContainer.value;
  if (!sc) return;
  stopAnimation();
  sc.dataset.suppressScrollbar = '1';
  sc.scrollTop = top;
  scheduleSuppressRelease();
}

function scrollToActiveLyric() {
  const sc = lyricsContainer.value;
  if (!sc) return;
  const line = sc.querySelector('[data-active-lyric="true"]') as HTMLElement | null;
  if (!line) return;

  const from = sc.scrollTop;
  const to = activeLineTargetTop(sc, line);
  const distance = Math.abs(to - from);
  if (distance < 1) return;

  if (rafId) cancelAnimationFrame(rafId);
  // 距离越长给越多时间，但收敛在 [260, 620]ms：长跳不拖沓、短跳不突兀
  const duration = Math.min(SCROLL_MAX_MS, SCROLL_MIN_MS + distance * 0.35);
  const startedAt = performance.now();
  sc.dataset.suppressScrollbar = '1';

  const step = (now: number) => {
    const node = lyricsContainer.value;
    if (!node) {
      rafId = 0;
      return;
    }
    const t = Math.min(1, (now - startedAt) / duration);
    const eased = 1 - Math.pow(1 - t, 3); // easeOutCubic：起步利落、收尾稳
    node.scrollTop = from + (to - from) * eased;
    if (t < 1) {
      rafId = requestAnimationFrame(step);
    } else {
      rafId = 0;
      scheduleSuppressRelease();
    }
  };
  rafId = requestAnimationFrame(step);
}

/** 用户一旦自己滚（滚轮/触摸/按住拖动），立刻让出控制权，不跟用户抢 */
function onUserScrollIntent() {
  stopAnimation();
}

onMounted(() => {
  const sc = lyricsContainer.value;
  sc?.addEventListener('wheel', onUserScrollIntent, { passive: true });
  sc?.addEventListener('touchstart', onUserScrollIntent, { passive: true });
  sc?.addEventListener('pointerdown', onUserScrollIntent, { passive: true });
});

onBeforeUnmount(() => {
  stopAnimation();
  const sc = lyricsContainer.value;
  sc?.removeEventListener('wheel', onUserScrollIntent);
  sc?.removeEventListener('touchstart', onUserScrollIntent);
  sc?.removeEventListener('pointerdown', onUserScrollIntent);
});

watch(
  () => playerStore.activeLyricIndex,
  () => nextTick(scrollToActiveLyric),
);

// 换歌：立刻回到开头（不播动画），避免上一首的滚动位置残留到新歌词上
watch(
  () => playerStore.currentTrack?.id,
  () => nextTick(() => jumpTo(0)),
);

/* variant → 样式映射 */
const isImmersive = computed(() => props.variant === 'immersive');

// 滚动容器：沉浸式外层已限高（= 封面高度），此处只负责滚动，靠上下等距内边距把当前行顶到中心
const containerClass = computed(() =>
  isImmersive.value
    ? 'h-full overflow-y-auto px-2'
    : 'flex-1 overflow-y-auto min-h-0',
);

// 沉浸式：内边距取外层高度的 1/2，保证当前行始终居中（--np-cover 由 NowPlayingImmersive 注入）
const immersivePad = computed(() =>
  isImmersive.value
    ? {
        paddingTop: 'calc(var(--np-cover, 80vh) / 2)',
        paddingBottom: 'calc(var(--np-cover, 80vh) / 2)',
      }
    : {},
);
const titleClass = computed(() =>
  isImmersive.value
    ? 'sr-only' // 沉浸式不显示标题，让歌词本身成为主角
    : 'text-[10px] font-semibold text-text-muted uppercase tracking-widest mb-3',
);
const lineClass = computed(() =>
  isImmersive.value
    ? 'text-[17px] leading-[2.4] transition-colors-smooth cursor-pointer px-2 text-center'
    : 'text-[13px] leading-[1.8] transition-colors-smooth cursor-pointer',
);
const emptyClass = computed(() =>
  isImmersive.value
    ? 'text-[15px] text-white/50 italic'
    : 'text-[13px] text-text-muted/70 italic',
);

function lineColor(i: number) {
  const active = i === playerStore.activeLyricIndex;
  if (active) return isImmersive.value ? 'text-white font-semibold' : 'text-brand-orange font-medium';
  if (i < playerStore.activeLyricIndex) {
    return isImmersive.value ? 'text-white/30' : 'text-text-muted';
  }
  return isImmersive.value ? 'text-white/55 hover:text-white/80' : 'text-text-secondary hover:text-text-primary';
}

function onSeek(line: { time?: number }) {
  playerStore.seek((line.time || 0) * 1000);
}
</script>

<template>
  <div ref="lyricsContainer" :class="containerClass" :style="immersivePad">
    <h3 v-if="!isImmersive" :class="titleClass">Lyrics</h3>

    <div v-if="playerStore.lyrics.length === 0" :class="emptyClass">暂无歌词</div>

    <div :class="isImmersive ? 'space-y-4' : 'space-y-3'">
      <p
        v-for="(line, i) in playerStore.lyrics"
        :key="i"
        :data-active-lyric="i === playerStore.activeLyricIndex"
        :class="[lineClass, lineColor(i)]"
        @click="onSeek(line)"
      >
        {{ line.text }}
      </p>
    </div>
  </div>
</template>
