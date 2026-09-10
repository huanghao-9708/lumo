<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue';
import { usePlayerStore } from '../../stores/player';

/**
 * 可复用的同步歌词视图。
 *
 * - 自动滚动到当前播放行（scrollIntoView smooth center）
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

function scrollToActiveLyric() {
  if (!lyricsContainer.value) return;
  const el = lyricsContainer.value.querySelector('[data-active-lyric="true"]') as HTMLElement | null;
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
}

watch(
  () => playerStore.activeLyricIndex,
  () => nextTick(scrollToActiveLyric),
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
