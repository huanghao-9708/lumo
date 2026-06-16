<script setup lang="ts">
import { computed } from 'vue';
import { useArtworkSrc } from '../../../composables/useArtworkSrc';

/**
 * 专辑/曲目封面图。
 *
 * 抽成独立子组件的原因：每个封面都需要自己一份 useArtworkSrc 的响应式状态
 * （composable 不能在 v-for 循环体里直接调用）。封装后，
 * AlbumsView / FoldersView / AlbumDetailView 只需要 <ArtworkImage :id="..." />
 * 即可，并且会自动享受内存 dataURL 缓存。
 *
 * 与直接用 <img :src="getArtworkUrl(id)"> 的差异：
 *   - 命中内存缓存时零网络请求（解决滚动时反复请求 lumo:// 的卡顿）
 *   - 未命中时仍走原始 URL（后端 ETag + Cache-Control 兜底）
 */
const props = defineProps<{
  artworkId?: number | null;
  /** 无封面时使用的 Tailwind 渐变色 class（如 'from-gray-500 to-gray-800'） */
  fallbackColor?: string;
  /** 额外附加到 <img> 的 class（用于 hover 缩放等效果） */
  imgClass?: string;
}>();

const coverSrc = useArtworkSrc(() => props.artworkId);
const hasCover = computed(() => !!props.artworkId && !!coverSrc.value);
</script>

<template>
  <div class="absolute inset-0 overflow-hidden">
    <img
      v-if="hasCover"
      :src="coverSrc"
      loading="lazy"
      decoding="async"
      class="w-full h-full object-cover"
      :class="imgClass"
    />
    <div
      v-else
      class="w-full h-full bg-gradient-to-br opacity-80"
      :class="fallbackColor || 'from-gray-400 to-gray-600'"
    ></div>
  </div>
</template>

