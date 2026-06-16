<script setup lang="ts">
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';

const playerStore = usePlayerStore();
</script>

<template>
  <aside class="w-full h-full flex flex-col font-mono text-xs uppercase bg-bg-base overflow-hidden">
    <!-- Header -->
    <div class="h-12 shrink-0 flex items-center px-6 border-b border-border-color">
      <h2 class="font-bold tracking-widest text-text-main">NOW PLAYING</h2>
    </div>

    <div class="flex-1 overflow-y-auto custom-scrollbar flex flex-col">
      <!-- Artwork -->
      <div class="w-full aspect-square bg-bg-panel border-b border-border-color shrink-0 flex items-center justify-center">
        <img 
          v-if="playerStore.currentTrack?.cover_artwork_id"
          :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
          class="w-full h-full object-cover filter grayscale contrast-125"
        />
        <div v-else class="text-text-muted">NO ARTWORK</div>
      </div>

      <!-- Info -->
      <div class="p-6 border-b border-border-color shrink-0">
        <h3 class="text-lg font-bold tracking-widest text-text-main mb-2 truncate">
          {{ playerStore.currentTrack?.title || 'LUMO' }}
        </h3>
        <p class="font-bold text-text-main truncate mb-1">
          {{ playerStore.currentTrack?.artist || '-' }}
        </p>
        <p class="text-text-muted truncate mb-4">
          {{ playerStore.currentTrack?.album || '-' }}
        </p>
        
        <div class="flex items-center gap-4 text-[10px] text-text-muted tracking-widest">
          <span>{{ playerStore.currentTrack?.duration || '0:00' }}</span>
          <span>{{ playerStore.currentTrack?.format || 'FLAC' }}</span>
        </div>
      </div>

      <!-- Lyrics / Extra info placeholder -->
      <div class="p-6 border-b border-border-color shrink-0 min-h-[120px]">
        <h4 class="font-bold tracking-widest text-text-main mb-4">LYRICS</h4>
        <div class="text-text-muted space-y-2 text-[10px] leading-relaxed">
          <p>...</p>
        </div>
      </div>

      <!-- Up Next -->
      <div class="p-6 flex-1 flex flex-col">
        <div class="flex items-center justify-between mb-4">
          <h4 class="font-bold tracking-widest text-text-main">UP NEXT</h4>
          <button @click="playerStore.queue = []; playerStore.currentIndex = -1" class="text-[10px] text-text-muted hover:text-text-main">CLEAR</button>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar -mx-6 px-6">
          <div 
            v-for="(track, index) in playerStore.queue" 
            :key="`${track.id}-${index}`"
            class="flex items-center gap-4 py-2 cursor-pointer group"
            @click="playerStore.playQueue(playerStore.queue, index)"
          >
            <span class="w-4 text-[10px]" :class="playerStore.currentIndex === index ? 'text-text-main font-bold' : 'text-text-muted'">
              {{ (index + 1).toString().padStart(2, '0') }}
            </span>
            <div class="w-8 h-8 bg-bg-panel border border-border-color shrink-0">
              <img 
                v-if="track.cover_artwork_id"
                :src="getArtworkUrl(track.cover_artwork_id)"
                class="w-full h-full object-cover filter grayscale contrast-125"
              />
            </div>
            <span class="flex-1 truncate" :class="playerStore.currentIndex === index ? 'text-text-main font-bold' : 'text-text-muted group-hover:text-text-main'">
              {{ track.title }}
            </span>
            <span class="text-[10px] text-text-muted">{{ track.duration }}</span>
          </div>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 0;
}
</style>
