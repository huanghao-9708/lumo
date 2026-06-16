<script setup lang="ts">
import { watch, ref, onBeforeUpdate } from 'vue';
import { usePlayerStore } from '../stores/player';
import { getArtworkUrl } from '../utils';
import { AudioLines, Play } from 'lucide-vue-next';

const playerStore = usePlayerStore();
const tabs = ['LYRICS', 'QUEUE', 'INFO'] as const;

const isLossless = (format: string) => {
  const f = format.toLowerCase();
  return ['flac', 'wav', 'alac', 'ape', 'dsf', 'dff'].includes(f);
};

const tabMapping = {
  'LYRICS': '歌词',
  'QUEUE': '播放队列',
  'INFO': '文件信息'
} as const;

const lyricRefs = ref<any[]>([]);

onBeforeUpdate(() => {
  lyricRefs.value = [];
});

watch(() => playerStore.activeLyricIndex, (newIdx) => {
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
  <aside class="w-[320px] flex flex-col h-full bg-transparent border-l border-[#e8e6df] shrink-0 relative px-10 py-10">
    <div data-tauri-drag-region class="absolute top-0 left-0 w-full h-10 z-0"></div>

    <!-- Tabs -->
    <div class="flex items-center justify-between border-b border-black pb-4 mb-8 relative z-10 uppercase text-[10px] font-bold tracking-[0.15em] shrink-0">
      <button 
        v-for="tab in tabs" 
        :key="tab"
        @click="playerStore.activeRightTab = tabMapping[tab]"
        class="transition-colors"
        :class="playerStore.activeRightTab === tabMapping[tab] ? 'text-accent' : 'text-text-muted hover:text-[#555]'"
      >
        {{ tabMapping[tab] }}
      </button>
    </div>

    <!-- Content Area: Each tab manages its own layout -->
    <div class="flex-1 min-h-0 relative z-10">
      <!-- 歌词 -->
      <template v-if="playerStore.activeRightTab === '歌词'">
        <div class="flex flex-col h-full">
          <!-- Track Info -->
          <div class="mb-6 flex flex-col shrink-0">
            <!-- Album Art Square -->
            <div class="w-full aspect-square bg-bg-panel  mb-5 overflow-hidden relative border border-border-color  shadow-sm transition-colors duration-700" :class="playerStore.currentTrack?.coverColor">
              <img 
                v-if="playerStore.currentTrack?.cover_artwork_id"
                :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
                class="absolute inset-0 w-full h-full object-cover"
              />
              <!-- dot pattern overlay -->
              <div class="absolute inset-0 opacity-20" style="background-image: radial-gradient(circle, #000 1px, transparent 1px); background-size: 8px 8px;"></div>
            </div>
            
            <p class="text-[9px] font-bold tracking-[0.2em] text-text-muted mb-2 uppercase">正在播放</p>
            <h2 class="font-serif italic text-3xl text-accent mb-1.5 truncate">{{ playerStore.currentTrack?.title || 'No Track' }}</h2>
            <p class="text-[11px] font-semibold tracking-widest text-text-main  mb-1 uppercase truncate">{{ playerStore.currentTrack?.artist || '-' }}</p>
            <p class="text-[11px] text-text-muted  italic truncate">{{ playerStore.currentTrack?.album || '-' }}</p>
            
            <div class="w-8 h-px bg-bg-active mt-4"></div>
          </div>

          <!-- Lyrics Scroll Area -->
          <div class="flex-1 overflow-y-auto custom-scrollbar -mr-4 pr-4">
            <div class="space-y-6 text-[13px] leading-relaxed pb-8">
              <p 
                v-for="(line, idx) in playerStore.lyrics" 
                :key="idx"
                :ref="el => { if (el) lyricRefs[idx] = el; }"
                class="transition-all duration-300"
                :class="[
                  idx === playerStore.activeLyricIndex ? 'font-serif italic font-bold text-[18px] text-accent tracking-wide' : 'text-text-muted',
                  line.text === '' ? 'h-4' : ''
                ]"
              >
                {{ line.text }}
              </p>
            </div>
          </div>
        </div>
      </template>

      <!-- 播放队列 -->
      <template v-else-if="playerStore.activeRightTab === '播放队列'">
        <div class="flex flex-col h-full">
          <!-- Queue Header -->
          <div class="mb-6 shrink-0">
            <p class="text-[9px] font-bold tracking-[0.2em] text-text-muted mb-1.5 uppercase">PLAYBACK QUEUE</p>
            <h2 class="font-serif italic text-3xl text-accent leading-none">播放队列</h2>
            <p class="text-[10px] text-text-muted  mt-2 tracking-wider">共 {{ playerStore.queue.length }} 首曲目</p>
            <div class="w-8 h-px bg-bg-active mt-4"></div>
          </div>

          <!-- Queue List Scroll Area -->
          <div class="flex-1 overflow-y-auto custom-scrollbar -mr-4 pr-4">
            <div class="space-y-4 pb-8">
              <div 
                v-for="(track, index) in playerStore.queue" 
                :key="`${track.id}-${index}`"
                @dblclick="playerStore.playQueue(playerStore.queue, index)"
                class="group cursor-pointer flex items-center justify-between py-1 border-b border-[#f0eee6]/40 hover:bg-black/5 rounded-sm px-2 transition-colors"
              >
                <div class="flex-1 min-w-0 pr-4">
                  <p 
                    class="text-[13px] transition-colors truncate" 
                    :class="playerStore.currentTrack?.id === track.id && playerStore.currentIndex === index ? 'font-serif italic font-bold text-accent text-[15px]' : 'text-text-muted group-hover:text-accent '"
                  >
                    {{ track.title }}
                  </p>
                  <p class="text-[10px] text-text-muted uppercase tracking-wider mt-1 truncate">{{ track.artist }}</p>
                </div>
                
                <!-- 正在播放的特效图标 -->
                <div v-if="playerStore.currentTrack?.id === track.id && playerStore.currentIndex === index" class="shrink-0 flex items-center justify-center pl-2">
                  <AudioLines v-if="playerStore.isPlaying" class="w-4 h-4 text-accent animate-pulse stroke-[1.5]" />
                  <Play v-else class="w-3.5 h-3.5 text-[#555] fill-current" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- 文件信息 -->
      <template v-else-if="playerStore.activeRightTab === '文件信息'">
        <div class="flex flex-col h-full">
          <!-- Info Header -->
          <div class="mb-6 shrink-0">
            <p class="text-[9px] font-bold tracking-[0.2em] text-text-muted mb-1.5 uppercase">FILE METADATA</p>
            <h2 class="font-serif italic text-3xl text-accent leading-none">文件信息</h2>
            <div class="w-8 h-px bg-bg-active mt-4"></div>
          </div>

          <!-- Info Scroll Area -->
          <div class="flex-1 overflow-y-auto custom-scrollbar -mr-4 pr-4">
            <!-- Small Album Art & Basic details inside Info Tab -->
            <div v-if="playerStore.currentTrack" class="flex items-center gap-4 mb-6 p-3 bg-black/[0.02] border border-[#f0eee6] rounded-md">
              <div class="w-16 h-16 bg-bg-panel  overflow-hidden relative shrink-0 border border-border-color  shadow-sm" :class="playerStore.currentTrack.coverColor">
                <img 
                  v-if="playerStore.currentTrack.cover_artwork_id"
                  :src="getArtworkUrl(playerStore.currentTrack.cover_artwork_id)"
                  class="absolute inset-0 w-full h-full object-cover"
                />
                <!-- dot pattern overlay -->
                <div class="absolute inset-0 opacity-15" style="background-image: radial-gradient(circle, #000 1px, transparent 1px); background-size: 6px 6px;"></div>
              </div>
              <div class="min-w-0">
                <h3 class="font-serif italic text-lg text-accent truncate leading-tight mb-1">{{ playerStore.currentTrack.title }}</h3>
                <p class="text-[10px] font-semibold text-[#555] truncate uppercase tracking-wider">{{ playerStore.currentTrack.artist }}</p>
                <p class="text-[9px] text-text-muted  truncate mt-0.5">{{ playerStore.currentTrack.album }}</p>
              </div>
            </div>

            <!-- Metadata List -->
            <div class="space-y-5 text-[11px] tracking-wider uppercase text-[#555] pb-8">
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">格式</span>
                <span class="text-accent">
                  {{ playerStore.currentTrackFileInfo?.format ? playerStore.currentTrackFileInfo.format.toUpperCase() : (playerStore.currentTrack?.format || '-').toUpperCase() }}
                  <span v-if="playerStore.currentTrackFileInfo?.format && isLossless(playerStore.currentTrackFileInfo.format)" class="text-[#d25050] ml-1 font-bold text-[9px]">(无损)</span>
                  <span v-else-if="playerStore.currentTrackFileInfo?.format" class="text-gray-500 ml-1 text-[9px]">(有损)</span>
                </span>
              </div>
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">比特率 / 采样率</span>
                <span class="text-accent">
                  <template v-if="playerStore.currentTrackFileInfo">
                    {{ playerStore.currentTrackFileInfo.bitrate ? `${Math.round(playerStore.currentTrackFileInfo.bitrate / 1000)} kbps` : '-' }} @ 
                    {{ playerStore.currentTrackFileInfo.sample_rate ? `${playerStore.currentTrackFileInfo.sample_rate.toLocaleString()} Hz` : '-' }}
                    <template v-if="playerStore.currentTrackFileInfo.bit_depth">
                       / {{ playerStore.currentTrackFileInfo.bit_depth }}-bit
                    </template>
                  </template>
                  <template v-else>-</template>
                </span>
              </div>
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">声道</span>
                <span class="text-accent">
                  {{ playerStore.currentTrackFileInfo?.channels === 1 ? '单声道' : (playerStore.currentTrackFileInfo?.channels === 2 ? '立体声' : (playerStore.currentTrackFileInfo?.channels ? `${playerStore.currentTrackFileInfo.channels} 声道` : '-')) }}
                </span>
              </div>
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">文件大小</span>
                <span class="text-accent">
                  {{ playerStore.currentTrackFileInfo?.file_size ? `${(playerStore.currentTrackFileInfo.file_size / (1024 * 1024)).toFixed(2)} MB` : '-' }}
                </span>
              </div>
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">时长</span>
                <span class="text-accent">{{ playerStore.currentTrack?.duration || '-' }}</span>
              </div>
              <div>
                <span class="text-text-muted block mb-1 text-[9px] font-bold">曲目编号</span>
                <span class="text-accent">ID: #{{ playerStore.currentTrack?.id || '-' }}</span>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </aside>
</template>
