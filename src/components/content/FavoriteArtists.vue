<script setup lang="ts">
import { computed } from 'vue';
import { User, Star, Loader2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();

const artists = computed(() => playerStore.favoriteArtists);
const isLoading = computed(() => false);

function selectArtist(artistId: number) {
  playerStore.activeLibraryTab = '艺术家';
  playerStore.activeArtistId = artistId;
}

function toggleFav(artistId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavoriteArtist(artistId, false);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="flex-1 overflow-y-auto px-8">
      <!-- 加载态 -->
      <div v-if="isLoading && artists.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="artists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Star class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">还没有收藏的歌手</span>
        <p class="text-[11px] text-text-muted/70">在歌手页面点击星标即可收藏</p>
      </div>

      <!-- 网格 -->
      <div
        v-else
        class="grid gap-6 pb-6"
        style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));"
      >
        <div
          v-for="(artist, index) in artists"
          :key="artist.id"
          class="group cursor-pointer"
          @click="selectArtist(artist.id)"
        >
          <div
            class="relative w-full aspect-square mb-3 overflow-hidden flex items-center justify-center bg-gradient-to-br"
            :class="[artist.avatarColor, index % 2 === 0 ? 'rounded-[10px]' : 'rounded-full']"
          >
            <div class="w-full h-full flex items-center justify-center bg-black/10 group-hover:bg-black/20 transition-colors-smooth">
              <User class="w-[40px] h-[40px] text-white/60" />
            </div>

            <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                class="w-7 h-7 rounded-full bg-black/40 backdrop-blur-sm flex items-center justify-center hover:bg-black/60 transition-colors-smooth"
                @click.stop="toggleFav(artist.id, $event)"
              >
                <Star class="w-[14px] h-[14px] text-white fill-current" />
              </button>
            </div>
          </div>

          <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ artist.name }}</p>
          <p class="text-[13px] text-text-secondary truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
      </div>
    </div>

    <FooterStatus v-if="artists.length > 0" :count="`${artists.length.toLocaleString()} 位艺术家`" />
  </div>
</template>
