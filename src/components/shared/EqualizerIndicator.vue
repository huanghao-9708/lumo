<script setup lang="ts">
withDefaults(defineProps<{
  playing?: boolean;
  size?: 'sm' | 'md';
  color?: string;
}>(), {
  playing: true,
  size: 'sm',
  color: 'bg-brand-orange',
});
</script>

<template>
  <div
    class="equalizer-indicator inline-flex items-end justify-center gap-[2px]"
    :class="[
      size === 'sm' ? 'w-[14px] h-[14px]' : 'w-[16px] h-[16px]',
      { 'is-playing': playing }
    ]"
    aria-label="正在播放"
  >
    <span class="eq-bar eq-bar-1" :class="color"></span>
    <span class="eq-bar eq-bar-2" :class="color"></span>
    <span class="eq-bar eq-bar-3" :class="color"></span>
  </div>
</template>

<style scoped>
.eq-bar {
  width: 2px;
  border-radius: 1px;
  transform-origin: bottom;
  will-change: height;
}

/* 静态时：自然起伏的高低姿态 */
.eq-bar-1 { height: 40%; }
.eq-bar-2 { height: 85%; }
.eq-bar-3 { height: 55%; }

/* 播放中：交错优雅呼吸律动动画 */
.is-playing .eq-bar-1 {
  animation: eq-bounce-1 0.95s ease-in-out infinite alternate;
}
.is-playing .eq-bar-2 {
  animation: eq-bounce-2 0.8s ease-in-out infinite alternate 0.12s;
}
.is-playing .eq-bar-3 {
  animation: eq-bounce-3 1.1s ease-in-out infinite alternate 0.25s;
}

@keyframes eq-bounce-1 {
  0% { height: 25%; }
  50% { height: 85%; }
  100% { height: 35%; }
}
@keyframes eq-bounce-2 {
  0% { height: 90%; }
  50% { height: 30%; }
  100% { height: 100%; }
}
@keyframes eq-bounce-3 {
  0% { height: 30%; }
  50% { height: 70%; }
  100% { height: 20%; }
}

@media (prefers-reduced-motion: reduce) {
  .eq-bar {
    animation: none !important;
  }
}
</style>
