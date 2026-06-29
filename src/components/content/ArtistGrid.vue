<script setup lang="ts">
import { computed } from 'vue';
import { User } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();

const artists = computed(() => playerStore.artists);

function selectArtist(artistId: number) {
  playerStore.activeArtistId = artistId;
}

function getColorClass(color: string): string {
  return color || 'from-gray-500 to-gray-700';
}
</script>

<template>
  <div class="flex-1 overflow-y-auto px-8">
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
          class="w-full aspect-square mb-3 overflow-hidden flex items-center justify-center bg-gradient-to-br"
          :class="[getColorClass(artist.avatarColor), index % 2 === 0 ? 'rounded-[10px]' : 'rounded-full']"
        >
          <div class="w-full h-full flex items-center justify-center bg-black/10 group-hover:bg-black/20 transition-colors-smooth">
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
  </div>
</template>
