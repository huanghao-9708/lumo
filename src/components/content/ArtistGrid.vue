<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { User, Loader2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';

const playerStore = usePlayerStore();

const artists = computed(() => playerStore.artists);
const isLoading = computed(() => playerStore.isLoadingArtists);
const hasMore = computed(() => playerStore.hasMoreArtists);

function selectArtist(artistId: number) {
  playerStore.activeArtistId = artistId;
}

function getColorClass(color: string): string {
  return color || 'from-gray-500 to-gray-700';
}

/** IntersectionObserver: 滚动到底部自动加载下一批 */
const scrollContainer = ref<HTMLElement | null>(null);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

onMounted(() => {
  if (!scrollContainer.value) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && !isLoading.value && hasMore.value) {
        playerStore.fetchArtists(false);
      }
    },
    { root: scrollContainer.value, rootMargin: '400px' }
  );
  if (sentinelRef.value) {
    observer.observe(sentinelRef.value);
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
});
</script>

<template>
  <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8">
    <div
      class="grid gap-6 pb-4"
      style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))"
    >
      <div
        v-for="(artist, index) in artists"
        :key="artist.id"
        class="group cursor-pointer"
        @click="selectArtist(artist.id)"
      >
        <div
          class="w-full aspect-square mb-3 overflow-hidden flex items-center justify-center relative"
          :class="[index % 2 === 0 ? 'rounded-[10px]' : 'rounded-full', !artist.avatar_artwork_id ? `bg-gradient-to-br ${getColorClass(artist.avatarColor)}` : '']"
        >
          <img 
            v-if="artist.avatar_artwork_id"
            :src="getArtworkUrl(artist.avatar_artwork_id)"
            class="w-full h-full object-cover"
          />
          <div v-else class="w-full h-full flex items-center justify-center bg-black/10 group-hover:bg-black/20 transition-colors-smooth">
            <User class="w-[40px] h-[40px] text-white/60" />
          </div>
        </div>

        <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ artist.name }}</p>
        <p class="text-[13px] text-text-secondary truncate">{{ artist.trackCount }} 首歌曲</p>
      </div>
    </div>

    <div v-if="artists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
      <User class="w-8 h-8 text-text-disabled" />
      <span class="text-[12px]">还没有扫描到艺术家</span>
    </div>

    <!-- sentinel — IntersectionObserver 触发加载下一批 -->
    <div ref="sentinelRef" class="h-px" />

    <!-- 增量加载指示 -->
    <div v-if="isLoading && artists.length > 0" class="flex items-center justify-center py-6 text-text-muted">
      <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
      <span class="text-[11px]">加载更多…</span>
    </div>

    <!-- 没有更多了 -->
    <div v-if="!hasMore && artists.length > 0" class="flex items-center justify-center py-6 text-text-muted">
      <span class="text-[11px]">已显示全部 {{ playerStore.artistsTotalCount.toLocaleString() }} 位艺术家</span>
    </div>
  </div>
</template>
