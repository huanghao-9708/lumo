<script setup lang="ts">
import { store } from "../store/mockStore";

function togglePlay() {
  store.togglePlay();
}

function nextTrack() {
  store.nextTrack();
}

function prevTrack() {
  store.prevTrack();
}

function toggleFavorite() {
  store.currentTrack.isFavorite = !store.currentTrack.isFavorite;
}

function toggleRightPanel() {
  store.isRightPanelOpen = !store.isRightPanelOpen;
}

// 拖拽进度条处理
function onProgressChange(event: Event) {
  const target = event.target as HTMLInputElement;
  store.currentTime = parseInt(target.value);
}

// 拖拽音量处理
function onVolumeChange(event: Event) {
  const target = event.target as HTMLInputElement;
  store.volume = parseInt(target.value);
}

// 音量静音切换
const lastVolume = 75;
function toggleMute() {
  if (store.volume > 0) {
    store.volume = 0;
  } else {
    store.volume = lastVolume;
  }
}
</script>

<template>
  <footer class="h-20 border-t bg-white dark:bg-zinc-900 border-zinc-200/60 dark:border-zinc-800/60 px-6 flex items-center justify-between select-none transition-colors duration-300">
    <!-- 左侧：正在播放歌曲元数据 -->
    <div class="flex items-center gap-3 w-1/4 min-w-[200px]">
      <div 
        :class="['bg-gradient-to-br', store.currentTrack.coverColor]" 
        class="w-12 h-12 rounded-xl flex-shrink-0 shadow-md relative overflow-hidden flex items-center justify-center text-white font-extrabold text-xs tracking-wider"
      >
        <span class="absolute opacity-20 text-3xl">♪</span>
        <!-- 光盘转动动画效果 (播放时慢速旋转) -->
        <div 
          v-if="store.isPlaying" 
          class="absolute inset-0 bg-black/10 rounded-full border border-white/10 animate-spin" 
          style="animation-duration: 8s"
        ></div>
      </div>
      <div class="truncate pr-2">
        <div class="flex items-center gap-1.5">
          <span class="font-semibold text-sm text-zinc-900 dark:text-zinc-100 truncate cursor-pointer hover:underline">{{ store.currentTrack.title }}</span>
          <button 
            @click="toggleFavorite"
            class="text-zinc-400 dark:text-zinc-500 hover:text-red-500 dark:hover:text-red-400 transition-colors cursor-pointer"
          >
            <!-- 心形图标 -->
            <svg 
              class="w-4 h-4" 
              :fill="store.currentTrack.isFavorite ? 'currentColor' : 'none'" 
              :class="store.currentTrack.isFavorite ? 'text-red-500' : ''"
              stroke="currentColor" 
              viewBox="0 0 24 24" 
              xmlns="http://www.w3.org/2000/svg"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"></path>
            </svg>
          </button>
        </div>
        <p class="text-xs text-zinc-500 dark:text-zinc-400 truncate mt-0.5">
          {{ store.currentTrack.artist }} <span class="mx-1 text-zinc-300 dark:text-zinc-700">•</span> {{ store.currentTrack.album }}
        </p>
      </div>
    </div>

    <!-- 中间：播放器核心控制与进度 -->
    <div class="flex flex-col items-center flex-1 max-w-xl px-4">
      <!-- 控制按钮 -->
      <div class="flex items-center gap-5 mb-2">
        <!-- 随机播放 -->
        <button class="p-1.5 text-zinc-400 dark:text-zinc-500 hover:text-orange-500 dark:hover:text-orange-400 active:scale-95 transition-all cursor-pointer">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"></path>
          </svg>
        </button>

        <!-- 上一首 -->
        <button 
          @click="prevTrack" 
          class="p-1.5 text-zinc-700 dark:text-zinc-300 hover:text-orange-500 dark:hover:text-orange-400 active:scale-90 transition-all cursor-pointer"
        >
          <svg class="w-5 h-5 fill-current" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M6 19h2V5H6v14zm3.5-7L18 19V5L9.5 12z"/>
          </svg>
        </button>

        <!-- 播放/暂停大按钮 -->
        <button 
          @click="togglePlay"
          class="w-10 h-10 flex items-center justify-center bg-orange-500 hover:bg-orange-400 text-white rounded-full shadow-lg shadow-orange-500/20 active:scale-95 hover:scale-105 transition-all cursor-pointer"
        >
          <svg v-if="store.isPlaying" class="w-5 h-5 fill-current" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
          </svg>
          <svg v-else class="w-5 h-5 fill-current ml-0.5" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M8 5v14l11-7z"/>
          </svg>
        </button>

        <!-- 下一首 -->
        <button 
          @click="nextTrack" 
          class="p-1.5 text-zinc-700 dark:text-zinc-300 hover:text-orange-500 dark:hover:text-orange-400 active:scale-90 transition-all cursor-pointer"
        >
          <svg class="w-5 h-5 fill-current" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path d="M16 5v14h2V5h-2zm-10 14l8.5-7L6 5v14z"/>
          </svg>
        </button>

        <!-- 列表循环 -->
        <button class="p-1.5 text-zinc-400 dark:text-zinc-500 hover:text-orange-500 dark:hover:text-orange-400 active:scale-95 transition-all cursor-pointer">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 1121.21 8H18.2"></path>
          </svg>
        </button>
      </div>

      <!-- 进度条与时间 -->
      <div class="flex items-center gap-3 w-full text-[11px] font-mono text-zinc-400 dark:text-zinc-500">
        <span>{{ store.formatTime(store.currentTime) }}</span>
        <div class="relative flex-1 group py-2 cursor-pointer">
          <input 
            type="range" 
            min="0" 
            :max="store.currentTrack.durationSec" 
            :value="store.currentTime" 
            @input="onProgressChange"
            class="w-full h-1 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-orange-500 hover:accent-orange-400 outline-none"
            style="background: linear-gradient(to right, var(--color-orange-500) 0%, var(--color-orange-500) v-bind('(store.currentTime / store.currentTrack.durationSec) * 100 + `%`'), var(--color-zinc-200) v-bind('(store.currentTime / store.currentTrack.durationSec) * 100 + `%`'))"
          />
        </div>
        <span>{{ store.currentTrack.duration }}</span>
      </div>
    </div>

    <!-- 右侧：音量与布局面板选项 -->
    <div class="flex items-center justify-end gap-4 w-1/4 min-w-[200px]">
      <!-- 音量控制 -->
      <div class="flex items-center gap-2 group/volume max-w-[130px] flex-1">
        <button 
          @click="toggleMute" 
          class="text-zinc-500 dark:text-zinc-400 hover:text-orange-500 dark:hover:text-orange-400 transition-colors cursor-pointer"
        >
          <!-- 静音 / 低音量 / 高音量 图标 -->
          <svg v-if="store.volume === 0" class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"></path>
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2"></path>
          </svg>
          <svg v-else-if="store.volume < 50" class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"></path>
          </svg>
          <svg v-else class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z"></path>
          </svg>
        </button>
        <input 
          type="range" 
          min="0" 
          max="100" 
          :value="store.volume" 
          @input="onVolumeChange"
          class="w-full h-1 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-orange-500 outline-none"
        />
      </div>

      <!-- 播放队列 -->
      <button class="p-1.5 text-zinc-500 dark:text-zinc-400 hover:text-orange-500 dark:hover:text-orange-400 active:scale-95 transition-colors cursor-pointer">
        <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
        </svg>
      </button>

      <!-- 音效/等响度 -->
      <button class="p-1.5 text-zinc-500 dark:text-zinc-400 hover:text-orange-500 dark:hover:text-orange-400 active:scale-95 transition-colors cursor-pointer">
        <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"></path>
        </svg>
      </button>

      <!-- 折叠右侧面板 -->
      <button 
        @click="toggleRightPanel"
        :class="store.isRightPanelOpen ? 'text-orange-500' : 'text-zinc-500 dark:text-zinc-400 hover:text-orange-500'"
        class="p-1.5 active:scale-95 transition-colors cursor-pointer"
        title="切换歌词面板"
      >
        <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
        </svg>
      </button>
    </div>
  </footer>
</template>

<style scoped>
/* 可以在此处添加 range input 的兼容美化样式 */
input[type="range"]::-webkit-slider-thumb {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--color-orange-500);
  cursor: pointer;
  appearance: none;
  transition: transform 0.1s ease-in-out;
}
input[type="range"]::-webkit-slider-thumb:hover {
  transform: scale(1.3);
}
</style>
