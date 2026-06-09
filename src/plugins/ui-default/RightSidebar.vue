<script setup lang="ts">
import { Minus, Square, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();
const tabs = ['歌词', '播放队列', '文件信息'] as const;

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
</script>

<template>
  <aside class="w-[320px] flex flex-col h-full bg-[#fafafa] border-l border-gray-100 shrink-0 relative">
    <div data-tauri-drag-region class="absolute top-0 left-0 w-full h-10 z-0"></div>

    <!-- Window Controls -->
    <div class="h-10 flex items-center justify-end px-4 gap-3 text-gray-500 relative z-10">
      <button @click="minimize" class="hover:text-gray-900 transition-smooth"><Minus class="w-4 h-4" /></button>
      <button @click="toggleMaximize" class="hover:text-gray-900 transition-smooth"><Square class="w-3.5 h-3.5" /></button>
      <button @click="close" class="hover:text-red-500 transition-smooth"><X class="w-4 h-4" /></button>
    </div>

    <!-- Tabs -->
    <div class="px-8 mt-2 flex items-center gap-6 border-b border-gray-200/60 pb-3">
      <button 
        v-for="tab in tabs" 
        :key="tab"
        @click="playerStore.activeRightTab = tab"
        class="text-sm font-medium transition-smooth relative pb-3 -mb-3"
        :class="playerStore.activeRightTab === tab ? 'text-brand-orange' : 'text-gray-500 hover:text-gray-800'"
      >
        {{ tab }}
        <div v-if="playerStore.activeRightTab === tab" class="absolute bottom-0 left-0 w-full h-[2px] bg-brand-orange rounded-t-full shadow-[0_-1px_4px_rgba(245,130,32,0.5)]"></div>
      </button>
    </div>

    <!-- Track Info -->
    <div class="p-8 pb-4 flex gap-4">
      <!-- Cover art with gradient -->
      <div 
        class="w-20 h-20 rounded-lg shadow-md overflow-hidden shrink-0 relative group"
        :class="['bg-gradient-to-br', playerStore.currentTrack.coverColor]"
      >
        <div class="absolute inset-0 bg-black/20 group-hover:bg-black/0 transition-smooth"></div>
      </div>
      <div class="flex flex-col justify-center overflow-hidden">
        <h2 class="font-bold text-gray-900 text-lg truncate">{{ playerStore.currentTrack.title }}</h2>
        <p class="text-sm text-gray-500 mt-0.5 truncate">{{ playerStore.currentTrack.artist }}</p>
        <p class="text-xs text-gray-400 mt-0.5 truncate">{{ playerStore.currentTrack.album }}</p>
        <p class="text-[11px] text-gray-400 mt-2 font-mono bg-white inline-block px-1.5 py-0.5 rounded border border-gray-100 shadow-sm w-fit">
          {{ playerStore.currentTrack.format }} 44.1kHz 16bit
        </p>
      </div>
    </div>

    <!-- Lyrics & Content -->
    <div class="flex-1 overflow-y-auto px-8 pb-8 space-y-5 text-[15px] leading-relaxed custom-scrollbar relative">
      <!-- Gradient mask for smooth top fade -->
      <div class="sticky top-0 w-full h-4 bg-gradient-to-b from-[#fafafa] to-transparent z-10 -mt-2"></div>
      
      <!-- 歌词内容 -->
      <template v-if="playerStore.activeRightTab === '歌词'">
        <p 
          v-for="(line, idx) in playerStore.lyrics" 
          :key="idx"
          class="transition-all duration-300 transform origin-left"
          :class="[
            line.isActive ? 'text-brand-orange font-medium text-[16px] drop-shadow-sm scale-105' : 'text-gray-500 hover:text-gray-700 cursor-pointer',
            line.text === '' ? 'h-2' : ''
          ]"
        >
          {{ line.text }}
        </p>
      </template>

      <!-- 播放队列内容 -->
      <template v-else-if="playerStore.activeRightTab === '播放队列'">
        <div class="space-y-3">
          <div 
            v-for="track in playerStore.tracks" 
            :key="track.id"
            @click="playerStore.playTrack(track.id)"
            class="flex items-center gap-3 p-2 rounded-lg cursor-pointer hover:bg-gray-100/50 transition-smooth"
            :class="playerStore.currentTrack.id === track.id ? 'bg-orange-50/40 text-brand-orange' : 'text-gray-600'"
          >
            <div 
              class="w-8 h-8 rounded shrink-0 bg-gradient-to-br" 
              :class="track.coverColor"
            ></div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold truncate" :class="playerStore.currentTrack.id === track.id ? 'text-brand-orange' : 'text-gray-800'">
                {{ track.title }}
              </p>
              <p class="text-[10px] text-gray-400 truncate">{{ track.artist }}</p>
            </div>
            <span class="text-[11px] font-mono text-gray-400">{{ track.duration }}</span>
          </div>
        </div>
      </template>

      <!-- 文件信息内容 -->
      <template v-else-if="playerStore.activeRightTab === '文件信息'">
        <div class="space-y-4 text-xs text-gray-600">
          <div>
            <span class="text-gray-400 block mb-1">文件格式</span>
            <span class="font-medium font-mono text-gray-800">{{ playerStore.currentTrack.format }} (Free Lossless Audio Codec)</span>
          </div>
          <div>
            <span class="text-gray-400 block mb-1">采样率 / 位深</span>
            <span class="font-medium font-mono text-gray-800">44,100 Hz / 16-bit</span>
          </div>
          <div>
            <span class="text-gray-400 block mb-1">比特率</span>
            <span class="font-medium font-mono text-gray-800">920 kbps (动态)</span>
          </div>
          <div>
            <span class="text-gray-400 block mb-1">声道</span>
            <span class="font-medium text-gray-800">2 声道 (立体声)</span>
          </div>
        </div>
      </template>

      <!-- Gradient mask for smooth bottom fade -->
      <div class="sticky bottom-0 w-full h-8 bg-gradient-to-t from-[#fafafa] to-transparent z-10 -mb-8"></div>
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
  background-color: transparent;
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background-color: #e5e7eb;
}
</style>
