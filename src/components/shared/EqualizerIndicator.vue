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
  height: 100%;
  border-radius: 1px;
  transform-origin: bottom;
  will-change: transform;
}

/* 静态时：自然起伏的高低姿态 */
.eq-bar-1 { transform: scaleY(0.4); }
.eq-bar-2 { transform: scaleY(0.85); }
.eq-bar-3 { transform: scaleY(0.55); }

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
  0% { transform: scaleY(0.25); }
  50% { transform: scaleY(0.85); }
  100% { transform: scaleY(0.35); }
}
@keyframes eq-bounce-2 {
  0% { transform: scaleY(0.9); }
  50% { transform: scaleY(0.3); }
  100% { transform: scaleY(1); }
}
@keyframes eq-bounce-3 {
  0% { transform: scaleY(0.3); }
  50% { transform: scaleY(0.7); }
  100% { transform: scaleY(0.2); }
}

@media (prefers-reduced-motion: reduce) {
  .eq-bar {
    animation: none !important;
  }
}
</style>
