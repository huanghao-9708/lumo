<script setup lang="ts">
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
const tabs = ['LYRICS', 'QUEUE', 'INFO'] as const;

const tabMapping = {
  'LYRICS': '歌词',
  'QUEUE': '播放队列',
  'INFO': '文件信息'
} as const;
</script>

<template>
  <aside class="w-[320px] flex flex-col h-full bg-transparent border-l border-[#e8e6df] shrink-0 relative px-10 py-10">
    <div data-tauri-drag-region class="absolute top-0 left-0 w-full h-10 z-0"></div>

    <!-- Tabs -->
    <div class="flex items-center justify-between border-b border-black pb-4 mb-8 relative z-10 uppercase text-[10px] font-bold tracking-[0.15em]">
      <button 
        v-for="tab in tabs" 
        :key="tab"
        @click="playerStore.activeRightTab = tabMapping[tab]"
        class="transition-colors"
        :class="playerStore.activeRightTab === tabMapping[tab] ? 'text-black' : 'text-[#a0a0a0] hover:text-[#555]'"
      >
        {{ tabMapping[tab] }}
      </button>
    </div>

    <!-- Track Info -->
    <div class="mb-8 relative z-10 flex flex-col">
      <!-- Album Art Square -->
      <div class="w-full aspect-square bg-[#eae8e1] mb-6 overflow-hidden relative border border-[#dcdad1]/50 shadow-sm transition-colors duration-700" :class="playerStore.currentTrack?.coverColor">
        <!-- dot pattern overlay -->
        <div class="absolute inset-0 opacity-20" style="background-image: radial-gradient(circle, #000 1px, transparent 1px); background-size: 8px 8px;"></div>
      </div>
      
      <p class="text-[9px] font-bold tracking-[0.2em] text-[#a0a0a0] mb-2 uppercase">正在播放</p>
      <h2 class="font-serif italic text-4xl text-black mb-2 truncate">{{ playerStore.currentTrack?.title || 'No Track' }}</h2>
      <p class="text-[11px] font-semibold tracking-widest text-[#333] mb-1 uppercase truncate">{{ playerStore.currentTrack?.artist || '-' }}</p>
      <p class="text-[11px] text-[#888] italic truncate">{{ playerStore.currentTrack?.album || '-' }}</p>
      
      <div class="w-8 h-px bg-[#dcdad1] mt-6"></div>
    </div>

    <!-- Lyrics & Content -->
    <div class="flex-1 overflow-y-auto custom-scrollbar relative z-10 -mr-4 pr-4">
      <!-- 歌词 -->
      <template v-if="playerStore.activeRightTab === '歌词'">
        <div class="space-y-6 text-[13px] leading-relaxed">
          <p 
            v-for="(line, idx) in playerStore.lyrics" 
            :key="idx"
            class="transition-all duration-300"
            :class="[
              line.isActive ? 'font-serif italic font-bold text-[18px] text-black tracking-wide' : 'text-[#777]',
              line.text === '' ? 'h-4' : ''
            ]"
          >
            {{ line.text }}
          </p>
        </div>
      </template>

      <!-- 播放队列 -->
      <template v-else-if="playerStore.activeRightTab === '播放队列'">
        <div class="space-y-4">
          <div 
            v-for="track in playerStore.tracks" 
            :key="track.id"
            @click="playerStore.playTrack(track.id)"
            class="group cursor-pointer"
          >
            <p class="text-[13px] transition-colors truncate" :class="playerStore.currentTrack?.id === track.id ? 'font-serif italic font-bold text-black text-[15px]' : 'text-[#777] group-hover:text-black'">
              {{ track.title }}
            </p>
            <p class="text-[10px] text-[#a0a0a0] uppercase tracking-wider mt-1 truncate">{{ track.artist }}</p>
          </div>
        </div>
      </template>

      <!-- 文件信息 -->
      <template v-else-if="playerStore.activeRightTab === '文件信息'">
        <div class="space-y-6 text-[11px] tracking-wider uppercase text-[#555]">
          <div>
            <span class="text-[#a0a0a0] block mb-1 text-[9px] font-bold">格式</span>
            <span class="text-black">{{ playerStore.currentTrack?.format || '-' }} (无损)</span>
          </div>
          <div>
            <span class="text-[#a0a0a0] block mb-1 text-[9px] font-bold">采样率</span>
            <span class="text-black">44,100 Hz / 16-bit</span>
          </div>
          <div>
            <span class="text-[#a0a0a0] block mb-1 text-[9px] font-bold">声道</span>
            <span class="text-black">立体声</span>
          </div>
        </div>
      </template>
    </div>
  </aside>
</template>
