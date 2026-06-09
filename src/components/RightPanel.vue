<script setup lang="ts">
import { store } from "../store/mockStore";

function selectTab(tab: "歌词" | "播放队列" | "文件信息") {
  store.activeRightTab = tab;
}

function playTrack(id: number) {
  store.playTrack(id);
}
</script>

<template>
  <aside 
    v-if="store.isRightPanelOpen"
    class="w-80 border-l flex flex-col h-full bg-white dark:bg-zinc-900 border-zinc-200/60 dark:border-zinc-800/60 transition-all duration-300 select-none"
  >
    <!-- Tab 头部 -->
    <div class="flex border-b border-zinc-200/60 dark:border-zinc-800/60 text-xs font-semibold px-4">
      <button 
        @click="selectTab('歌词')"
        :class="store.activeRightTab === '歌词' 
          ? 'text-orange-500 border-b-2 border-orange-500' 
          : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-100'"
        class="flex-1 py-4.5 text-center cursor-pointer transition-colors"
      >
        歌词
      </button>
      <button 
        @click="selectTab('播放队列')"
        :class="store.activeRightTab === '播放队列' 
          ? 'text-orange-500 border-b-2 border-orange-500' 
          : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-100'"
        class="flex-1 py-4.5 text-center cursor-pointer transition-colors"
      >
        播放队列
      </button>
      <button 
        @click="selectTab('文件信息')"
        :class="store.activeRightTab === '文件信息' 
          ? 'text-orange-500 border-b-2 border-orange-500' 
          : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-100'"
        class="flex-1 py-4.5 text-center cursor-pointer transition-colors"
      >
        文件信息
      </button>
    </div>

    <!-- 歌曲基本信息大卡片 -->
    <div class="p-6 flex flex-col items-center text-center border-b border-zinc-100 dark:border-zinc-800/50">
      <div 
        :class="['bg-gradient-to-br', store.currentTrack.coverColor]" 
        class="w-40 h-40 rounded-2xl shadow-lg shadow-zinc-950/15 mb-4 flex items-center justify-center text-white text-5xl font-extrabold"
      >
        <span>L</span>
      </div>
      <h2 class="font-bold text-zinc-900 dark:text-zinc-50 text-base truncate w-full px-2">
        {{ store.currentTrack.title }}
      </h2>
      <p class="text-xs text-zinc-500 dark:text-zinc-400 truncate w-full mt-1">
        {{ store.currentTrack.artist }}
      </p>
      <p class="text-[10px] text-zinc-400 dark:text-zinc-500 truncate w-full mt-0.5">
        {{ store.currentTrack.album }}
      </p>
      <div class="mt-3 px-3 py-1 bg-zinc-100 dark:bg-zinc-800 rounded-full text-[10px] font-semibold tracking-wider text-zinc-500 dark:text-zinc-400 font-mono">
        {{ store.currentTrack.format }} 44.1kHz 16bit
      </div>
    </div>

    <!-- Tab 内容区域 -->
    <div class="flex-1 overflow-y-auto p-6 scrollbar-thin">
      <!-- 1. 歌词面板 -->
      <div v-if="store.activeRightTab === '歌词'" class="space-y-4 text-center py-4 text-sm">
        <p 
          v-for="(line, idx) in store.lyrics" 
          :key="idx"
          :class="line.isActive 
            ? 'text-orange-500 dark:text-orange-400 font-bold scale-105' 
            : 'text-zinc-600 dark:text-zinc-400 opacity-40 hover:opacity-85'"
          class="transition-all duration-300 cursor-pointer leading-relaxed hover:scale-[1.02]"
        >
          {{ line.text }}
        </p>
      </div>

      <!-- 2. 播放队列 -->
      <div v-else-if="store.activeRightTab === '播放队列'" class="space-y-2">
        <h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-3">播放中</h3>
        <div class="flex items-center gap-3 p-2 bg-orange-500/5 dark:bg-orange-500/10 border border-orange-500/10 rounded-xl mb-4">
          <div :class="['bg-gradient-to-br', store.currentTrack.coverColor]" class="w-8 h-8 rounded-lg flex-shrink-0 text-white font-extrabold flex items-center justify-center text-[10px]">L</div>
          <div class="truncate flex-1">
            <p class="text-xs font-semibold text-orange-600 dark:text-orange-400 truncate">{{ store.currentTrack.title }}</p>
            <p class="text-[10px] text-zinc-500 dark:text-zinc-400 truncate">{{ store.currentTrack.artist }}</p>
          </div>
          <span class="text-[10px] text-orange-500 font-semibold font-mono pr-2">播放中</span>
        </div>

        <h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-3">接下来的歌曲</h3>
        <div class="space-y-1.5">
          <div 
            v-for="(track, idx) in store.tracks.filter((_, i) => i !== store.currentTrackIndex)"
            :key="track.id"
            @click="playTrack(track.id)"
            class="flex items-center gap-3 p-2 hover:bg-zinc-100 dark:hover:bg-zinc-800/50 rounded-xl cursor-pointer group transition-all duration-200"
          >
            <div :class="['bg-gradient-to-br', track.coverColor]" class="w-8 h-8 rounded-lg flex-shrink-0 text-white font-extrabold flex items-center justify-center text-[10px]">
              {{ track.title.substring(0,1) }}
            </div>
            <div class="truncate flex-1">
              <p class="text-xs font-medium text-zinc-700 dark:text-zinc-300 group-hover:text-zinc-950 dark:group-hover:text-zinc-50 truncate">{{ track.title }}</p>
              <p class="text-[10px] text-zinc-400 dark:text-zinc-500 truncate">{{ track.artist }}</p>
            </div>
            <span class="text-[10px] font-mono text-zinc-400 group-hover:text-zinc-600 dark:group-hover:text-zinc-400 pr-1">{{ track.duration }}</span>
          </div>
        </div>
      </div>

      <!-- 3. 文件信息 -->
      <div v-else-if="store.activeRightTab === '文件信息'" class="text-xs text-zinc-600 dark:text-zinc-400 space-y-4">
        <h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">流媒体与元数据</h3>
        
        <div class="bg-zinc-50 dark:bg-zinc-900/40 rounded-xl p-4 border border-zinc-200/40 dark:border-zinc-800/40 space-y-3 font-mono">
          <div class="flex justify-between border-b border-zinc-200/50 dark:border-zinc-800/50 pb-1.5">
            <span class="text-zinc-400">格式 (Format)</span>
            <span class="text-zinc-800 dark:text-zinc-200 font-semibold">{{ store.currentTrack.format }}</span>
          </div>
          <div class="flex justify-between border-b border-zinc-200/50 dark:border-zinc-800/50 pb-1.5">
            <span class="text-zinc-400">采样率 (Sample Rate)</span>
            <span class="text-zinc-800 dark:text-zinc-200">44.1 kHz</span>
          </div>
          <div class="flex justify-between border-b border-zinc-200/50 dark:border-zinc-800/50 pb-1.5">
            <span class="text-zinc-400">位深 (Bit Depth)</span>
            <span class="text-zinc-800 dark:text-zinc-200">16 bit</span>
          </div>
          <div class="flex justify-between border-b border-zinc-200/50 dark:border-zinc-800/50 pb-1.5">
            <span class="text-zinc-400">声道 (Channels)</span>
            <span class="text-zinc-800 dark:text-zinc-200">2 (Stereo)</span>
          </div>
          <div class="flex justify-between">
            <span class="text-zinc-400">文件大小 (File Size)</span>
            <span class="text-zinc-800 dark:text-zinc-200">34.2 MB</span>
          </div>
        </div>

        <h3 class="text-xs font-semibold text-zinc-400 uppercase tracking-wider mb-2">物理路径</h3>
        <div class="bg-zinc-50 dark:bg-zinc-900/40 border border-zinc-200/40 dark:border-zinc-800/40 rounded-xl p-3 text-[10px] break-all leading-normal font-mono">
          D:\code\rust\lumo\music\{{ store.currentTrack.artist.replace(/\s+/g, '') }}\{{ store.currentTrack.title.replace(/\s+/g, '') }}.{{ store.currentTrack.format.toLowerCase() }}
        </div>
      </div>
    </div>
  </aside>
</template>
