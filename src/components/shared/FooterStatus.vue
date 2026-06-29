<script setup lang="ts">
defineProps<{
  count: string;
  hint?: string;
}>();
</script>

<template>
  <div class="flex items-center justify-between py-4 border-t border-border-color mt-2 text-[11px] text-text-muted font-mono">
    <span>{{ count }}</span>
    <span>{{ hint ?? '双击播放' }}</span>
  </div>
</template>
