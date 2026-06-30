<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { Play, Loader2, Disc3 } from 'lucide-vue-next';
import { usePlayerStore, type Album } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import { libraryGetAlbumTracks } from '../../api/library';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();

const emit = defineEmits<{
  (e: 'select', album: Album): void;
}>();

/** 封面 src：优先使用 cover_thumb（后端内联 base64），否则走 artwork URL 协议 */
function getCoverSrc(album: Album): string {
  if (album.cover_thumb) return album.cover_thumb;
  if (album.cover_artwork_id) return getArtworkUrl(album.cover_artwork_id);
  return '';
}

function selectAlbum(album: Album) {
  emit('select', album);
}

/** 双击专辑封面 → 直接播放第一首 */
async function playAlbum(album: Album) {
  try {
    const result = await libraryGetAlbumTracks(album.id);
    const tracks = result.map(t => {
      const durationMs = t.duration_ms ?? 0;
      const sec = Math.floor(durationMs / 1000);
      return {
        id: t.id,
        title: t.title,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || album.title,
        duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
        durationSec: sec,
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        coverColor: '',
        isFavorite: false,
        primary_file_id: t.media_file_id,
        cover_artwork_id: t.cover_artwork_id,
        fileSize: t.file_size ?? null,
      };
    });
    if (tracks.length > 0) {
      await playerStore.playAll(tracks, 0);
    }
  } catch (e) {
    console.error('Failed to play album:', e);
  }
}

const totalCount = computed(() => playerStore.albumsTotalCount);
const isLoading = computed(() => playerStore.isLoadingAlbums);
const isError = computed(() => playerStore.isErrorAlbums);
const hasMoreAlbums = computed(() => playerStore.hasMoreAlbums);

/** IntersectionObserver: 滚动到底部自动加载下一批 */
const gridContainer = ref<HTMLElement | null>(null);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

onMounted(() => {
  if (!gridContainer.value) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && !isLoading.value && hasMoreAlbums.value) {
        playerStore.fetchAlbums(false);
      }
    },
    { root: gridContainer.value, rootMargin: '400px' }
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
  <div class="flex flex-col h-full overflow-hidden">

    <!-- 网格容器 -->
    <div ref="gridContainer" class="flex-1 overflow-y-auto px-8">

      <!-- 加载态 -->
      <div v-if="isLoading && playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
        <span class="text-[12px]">加载专辑…</span>
      </div>

      <!-- 错误态 -->
      <div v-else-if="isError && playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <span class="text-[12px]">加载失败，请稍后重试</span>
      </div>

      <!-- 空态 -->
      <div v-else-if="playerStore.albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">没有找到专辑</span>
      </div>

      <!-- 5 列网格 -->
      <div
        v-else
        class="grid gap-6 pb-6"
        style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));"
      >
        <div
          v-for="album in playerStore.albums"
          :key="album.id"
          class="group cursor-pointer min-w-0"
          @dblclick="playAlbum(album)"
        >
          <!-- 封面 -->
          <div
            class="relative w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-3"
            @click="selectAlbum(album)"
          >
            <img
              v-if="getCoverSrc(album)"
              :src="getCoverSrc(album)"
              :alt="album.title"
              class="w-full h-full object-cover"
              loading="lazy"
            />
            <div v-else class="w-full h-full flex items-center justify-center bg-bg-hover">
              <Disc3 class="w-10 h-10 text-text-disabled" aria-hidden="true" />
            </div>

            <!-- 悬浮播放按钮 -->
            <div
              class="absolute inset-0 bg-black/0 group-hover:bg-black/20 dark:group-hover:bg-black/40 transition-colors-smooth flex items-center justify-center opacity-0 group-hover:opacity-100"
              @click.stop="playAlbum(album)"
            >
              <div class="w-10 h-10 rounded-full bg-brand-orange text-white flex items-center justify-center shadow-lg">
                <Play class="w-4 h-4 fill-current ml-0.5" />
              </div>
            </div>
          </div>

          <!-- 标题 + 艺术家 -->
          <p
            class="text-[15px] font-medium text-text-primary truncate mb-0.5"
            :class="playerStore.activeAlbumId === album.id ? 'text-brand-orange' : ''"
            @click="selectAlbum(album)"
          >{{ album.title }}</p>
          <p class="text-[13px] text-text-muted truncate">{{ album.artist }}<span v-if="album.year"> · {{ album.year }}</span></p>
        </div>
      </div>

      <!-- sentinel — IntersectionObserver 触发加载下一批 -->
      <div ref="sentinelRef" class="h-px" />

      <!-- 增量加载指示 -->
      <div v-if="isLoading && playerStore.albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
        <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
        <span class="text-[11px]">加载更多…</span>
      </div>

      <!-- 没有更多了 -->
      <div v-if="!hasMoreAlbums && playerStore.albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
        <span class="text-[11px]">已显示全部 {{ totalCount.toLocaleString() }} 张专辑</span>
      </div>

      <!-- Footer -->
      </div>

      <!-- Footer Status（固定在底部） -->
      <FooterStatus :count="`${totalCount.toLocaleString()} 张专辑`" />

    </div>
</template>
