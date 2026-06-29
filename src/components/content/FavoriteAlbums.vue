<script setup lang="ts">
import { computed } from 'vue';
import { Disc3, Heart, Loader2 } from 'lucide-vue-next';
import { usePlayerStore, type Album } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();

const albums = computed(() => playerStore.favoriteAlbums);
const isLoading = computed(() => false);

function getCoverSrc(album: Album): string {
  if (album.cover_thumb) return album.cover_thumb;
  if (album.cover_artwork_id) return getArtworkUrl(album.cover_artwork_id);
  return '';
}

function selectAlbum(album: Album) {
  playerStore.activeLibraryTab = '专辑';
  playerStore.activeAlbumId = album.id;
}

function toggleFav(album: Album, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavoriteAlbum(album.id, false);
}
</script>

<template>
  <div class="flex-1 overflow-y-auto px-8">
    <!-- 加载态 -->
    <div v-if="isLoading && albums.length === 0" class="flex items-center justify-center py-20 text-text-muted">
      <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
    </div>

    <!-- 空态 -->
    <div v-else-if="albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
      <Heart class="w-8 h-8 text-text-disabled" />
      <span class="text-[12px]">还没有收藏的专辑</span>
      <p class="text-[11px] text-text-muted/70">在专辑上点击心形图标即可收藏</p>
    </div>

    <!-- 网格 -->
    <div
      v-else
      class="grid gap-6 pb-6"
      style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));"
    >
      <div
        v-for="album in albums"
        :key="album.id"
        class="group cursor-pointer min-w-0"
        @click="selectAlbum(album)"
      >
        <div class="relative w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-3">
          <img
            v-if="getCoverSrc(album)"
            :src="getCoverSrc(album)"
            :alt="album.title"
            class="w-full h-full object-cover"
            loading="lazy"
          />
          <div v-else class="w-full h-full flex items-center justify-center bg-bg-hover">
            <Disc3 class="w-10 h-10 text-text-disabled" />
          </div>

          <!-- 收藏按钮（hover 显示，点击取消收藏） -->
          <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              class="w-7 h-7 rounded-full bg-black/40 backdrop-blur-sm flex items-center justify-center hover:bg-black/60 transition-colors-smooth"
              @click.stop="toggleFav(album, $event)"
            >
              <Heart class="w-[14px] h-[14px] text-white fill-current" />
            </button>
          </div>
        </div>

        <p class="text-[15px] font-medium text-text-primary truncate mb-0.5">{{ album.title }}</p>
        <p class="text-[13px] text-text-muted truncate">{{ album.artist }}</p>
      </div>
    </div>

    <!-- Footer -->
    </div>

    <!-- Footer Status（固定在底部） -->
    <FooterStatus v-if="albums.length > 0" :count="`${albums.length.toLocaleString()} 张专辑`" />
  </div>
</template>
