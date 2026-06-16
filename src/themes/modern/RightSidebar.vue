<script setup lang="ts">
import { watch, ref, onBeforeUpdate } from 'vue';
import { usePlayerStore } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import { Heart, MoreHorizontal, AudioLines, Play } from 'lucide-vue-next';

const playerStore = usePlayerStore();
const tabs = ['INFO', 'LYRICS', 'QUEUE'] as const;

const tabMapping = {
  'INFO': '文件信息',
  'LYRICS': '歌词',
  'QUEUE': '播放队列'
} as const;

// Ensure right tab is set to something valid, or use INFO by default
if (!playerStore.activeRightTab) {
  playerStore.activeRightTab = '文件信息';
}

const lyricRefs = ref<any[]>([]);

onBeforeUpdate(() => {
  lyricRefs.value = [];
});

watch(() => playerStore.activeLyricIndex, (newIdx) => {
  if (playerStore.activeRightTab !== '歌词') return;
  if (newIdx !== undefined && newIdx !== null && newIdx >= 0) {
    const activeEl = lyricRefs.value[newIdx];
    if (activeEl) {
      activeEl.scrollIntoView({
        behavior: 'smooth',
        block: 'center'
      });
    }
  }
});
</script>

<template>
  <aside class="w-full h-full flex flex-col font-sans text-xs bg-bg-base overflow-hidden px-8 py-8">
    
    <!-- Tabs Header -->
    <div class="mb-6 flex items-center justify-between uppercase tracking-widest text-[10px] font-bold border-b border-border-color/60 pb-3">
      <button 
        v-for="tab in tabs" 
        :key="tab"
        @click="playerStore.activeRightTab = tabMapping[tab]"
        class="transition-colors relative"
        :class="playerStore.activeRightTab === tabMapping[tab] ? 'text-text-main' : 'text-text-muted hover:text-text-main'"
      >
        {{ tab }}
        <div v-if="playerStore.activeRightTab === tabMapping[tab]" class="absolute -bottom-3 left-1/2 -translate-x-1/2 w-1 h-1 bg-accent rounded-full"></div>
      </button>
    </div>

    <!-- Tab Content -->
    <div class="flex-1 overflow-y-auto custom-scrollbar -mr-4 pr-4 pb-8 flex flex-col relative">
      
      <!-- INFO TAB -->
      <transition name="fade">
        <div v-if="playerStore.activeRightTab === '文件信息'" class="flex flex-col h-full absolute inset-0">
          <div class="w-full aspect-square rounded-xl bg-bg-panel overflow-hidden shadow-sm mb-6 relative group shrink-0">
            <img 
              v-if="playerStore.currentTrack?.cover_artwork_id"
              :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
              class="absolute inset-0 w-full h-full object-cover transition-transform duration-700 group-hover:scale-105"
            />
          </div>
          
          <div class="flex items-start justify-between gap-4 shrink-0">
            <div class="flex-1 min-w-0">
              <h2 class="text-xl font-bold text-text-main truncate mb-1">
                {{ playerStore.currentTrack?.title || 'No Track' }}
              </h2>
              <p class="text-[13px] text-text-main font-medium truncate mb-1">
                {{ playerStore.currentTrack?.artist || 'Unknown Artist' }}
              </p>
              <p class="text-[11px] text-text-muted truncate mb-4">
                {{ playerStore.currentTrack?.album || 'Unknown Album' }}
              </p>
              <div class="text-[9px] text-text-muted uppercase tracking-widest flex flex-col gap-1">
                <span>FORMAT: {{ playerStore.currentTrack?.format || 'FLAC' }}</span>
                <span>DURATION: {{ playerStore.currentTrack?.duration || '0:00' }}</span>
                <span>SAMPLE RATE: 96kHz / 24bit</span>
              </div>
            </div>
            <div class="flex flex-col items-center gap-4 shrink-0 pt-1">
              <button @click="playerStore.toggleFavorite(playerStore.currentTrack?.id || 0)" class="text-text-muted hover:text-accent transition-colors">
                <Heart class="w-5 h-5 stroke-[2]" :class="{'fill-accent text-accent': playerStore.currentTrack?.isFavorite}" />
              </button>
              <button class="text-text-muted hover:text-text-main transition-colors">
                <MoreHorizontal class="w-5 h-5" />
              </button>
            </div>
          </div>
        </div>
      </transition>

      <!-- LYRICS TAB -->
      <transition name="fade">
        <div v-if="playerStore.activeRightTab === '歌词'" class="flex flex-col h-full absolute inset-0">
          <div class="mb-4 shrink-0">
            <h2 class="text-lg font-bold text-text-main truncate mb-1">
              {{ playerStore.currentTrack?.title || 'No Track' }}
            </h2>
            <p class="text-[11px] text-text-muted truncate">
              {{ playerStore.currentTrack?.artist || 'Unknown Artist' }}
            </p>
          </div>
          <div class="flex-1 overflow-y-auto custom-scrollbar pr-2">
            <div class="space-y-4 text-[12px] leading-relaxed pb-8 pt-4">
              <p 
                v-for="(line, idx) in playerStore.lyrics" 
                :key="idx"
                :ref="el => { if (el) lyricRefs[idx] = el; }"
                class="transition-all duration-300"
                :class="[
                  idx === playerStore.activeLyricIndex ? 'text-accent font-medium text-[14px]' : 'text-text-muted',
                  line.text === '' ? 'h-3' : ''
                ]"
              >
                {{ line.text }}
              </p>
              <p v-if="playerStore.lyrics.length === 0" class="text-text-muted italic">
                No lyrics available.
              </p>
            </div>
          </div>
        </div>
      </transition>

      <!-- QUEUE TAB -->
      <transition name="fade">
        <div v-if="playerStore.activeRightTab === '播放队列'" class="flex flex-col h-full absolute inset-0">
          <div class="flex items-center justify-between mb-4 shrink-0">
            <div class="text-[10px] text-text-muted">共 {{ playerStore.queue.length }} 首曲目</div>
            <button @click="playerStore.queue = []; playerStore.currentIndex = -1" class="text-[10px] text-text-muted hover:text-text-main">清空</button>
          </div>
          
          <div class="flex-1 overflow-y-auto custom-scrollbar pr-2">
            <div class="flex flex-col pb-8">
              <div 
                v-for="(track, index) in playerStore.queue" 
                :key="`${track.id}-${index}`"
                class="flex items-center gap-3 py-2.5 cursor-pointer group rounded-lg px-2 -mx-2 hover:bg-bg-active/30 transition-colors border-b border-border-color/30"
                @click="playerStore.playQueue(playerStore.queue, index)"
              >
                <!-- Indicator / Number -->
                <div class="w-4 flex justify-center items-center shrink-0">
                  <div v-if="playerStore.currentTrack?.id === track.id && playerStore.currentIndex === index">
                    <AudioLines v-if="playerStore.isPlaying" class="w-3.5 h-3.5 text-accent animate-pulse stroke-[2]" />
                    <Play v-else class="w-3 h-3 text-text-main fill-current" />
                  </div>
                  <span v-else class="text-[9px] font-mono text-text-muted group-hover:text-text-main">
                    {{ (index + 1).toString().padStart(2, '0') }}
                  </span>
                </div>

                <div class="flex-1 min-w-0 pr-2">
                  <p class="text-[12px] truncate" :class="playerStore.currentIndex === index ? 'text-text-main font-bold' : 'text-text-main font-medium group-hover:text-text-main'">
                    {{ track.title }}
                  </p>
                  <p class="text-[10px] text-text-muted truncate mt-0.5">{{ track.artist }}</p>
                </div>
                
                <span class="text-[10px] font-mono text-text-muted">{{ track.duration }}</span>
              </div>
            </div>
          </div>
        </div>
      </transition>

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
  border-radius: 4px;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
