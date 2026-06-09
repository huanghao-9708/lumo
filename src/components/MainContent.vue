<script setup lang="ts">
import { ref, computed } from "vue";
import { store, Track } from "../store/mockStore";

const searchQuery = ref("");

const filteredTracks = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return store.tracks;
  return store.tracks.filter(t => 
    t.title.toLowerCase().includes(q) || 
    t.artist.toLowerCase().includes(q) || 
    t.album.toLowerCase().includes(q)
  );
});

function playTrack(id: number) {
  store.playTrack(id);
}

function toggleFavorite(track: Track, event: Event) {
  event.stopPropagation(); // 阻止触发播放行点击
  track.isFavorite = !track.isFavorite;
}
</script>

<template>
  <main class="flex-1 flex flex-col h-full bg-white dark:bg-zinc-900 transition-colors duration-300 select-none overflow-hidden">
    <!-- 顶部搜索栏 -->
    <div class="px-8 py-4 flex items-center justify-between border-b border-zinc-100 dark:border-zinc-800/50">
      <!-- 搜索框 -->
      <div class="relative w-96 max-w-lg">
        <span class="absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-400 dark:text-zinc-500">
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
        </span>
        <input 
          v-model="searchQuery"
          type="text" 
          placeholder="搜索歌曲、专辑、艺人..." 
          class="w-full pl-10 pr-4 py-2 text-sm bg-zinc-100 dark:bg-zinc-800/60 border border-transparent focus:border-zinc-200 dark:focus:border-zinc-700/80 focus:bg-white dark:focus:bg-zinc-950 text-zinc-800 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 rounded-full outline-none transition-all duration-200"
        />
        <span 
          v-if="searchQuery" 
          @click="searchQuery = ''"
          class="absolute right-3.5 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600 dark:text-zinc-500 dark:hover:text-zinc-300 cursor-pointer text-xs"
        >
          ✕
        </span>
      </div>

      <!-- 右侧视图选项 -->
      <div class="flex items-center gap-1">
        <!-- 过滤 -->
        <button class="p-2 text-zinc-500 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg cursor-pointer transition-colors">
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path>
          </svg>
        </button>
        <!-- 列表视图 -->
        <button class="p-2 text-orange-500 bg-orange-500/10 rounded-lg cursor-pointer transition-colors">
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"></path>
          </svg>
        </button>
        <!-- 网格视图 -->
        <button class="p-2 text-zinc-500 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg cursor-pointer transition-colors">
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path>
          </svg>
        </button>
        <!-- 选择/复选 -->
        <button class="p-2 text-zinc-500 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg cursor-pointer transition-colors">
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"></path>
          </svg>
        </button>
      </div>
    </div>

    <!-- 列表标题 -->
    <div class="px-8 pt-6 pb-2 flex items-baseline justify-between">
      <div class="flex items-baseline gap-3">
        <h2 class="text-2xl font-extrabold text-zinc-900 dark:text-zinc-50 tracking-tight">
          {{ store.activeLibraryTab }}
        </h2>
        <span class="text-xs font-semibold text-zinc-400 dark:text-zinc-500 font-mono">
          {{ filteredTracks.length }} 首歌曲
        </span>
      </div>
    </div>

    <!-- 歌曲列表主体 -->
    <div class="flex-1 overflow-y-auto px-8 pb-4 scrollbar-thin">
      <table class="w-full border-collapse text-left">
        <thead>
          <tr class="text-[11px] font-semibold text-zinc-400 dark:text-zinc-500 border-b border-zinc-100 dark:border-zinc-800/60 uppercase tracking-wider">
            <th class="py-3 pl-3 w-12 text-center">#</th>
            <th class="py-3 pl-6">标题</th>
            <th class="py-3">艺人</th>
            <th class="py-3">专辑</th>
            <th class="py-3 w-20 text-center">时长</th>
            <th class="py-3 w-24 text-center">文件格式</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-zinc-50 dark:divide-zinc-800/30">
          <tr 
            v-for="(track, index) in filteredTracks" 
            :key="track.id"
            @click="playTrack(track.id)"
            :class="[
              store.currentTrackIndex === store.tracks.findIndex(t => t.id === track.id)
                ? 'bg-orange-500/[0.04] dark:bg-orange-500/[0.07] border-l-3 border-l-orange-500 pl-[13px]' 
                : 'hover:bg-zinc-50/80 dark:hover:bg-zinc-800/30'
            ]"
            class="group text-sm cursor-pointer transition-all duration-200"
          >
            <!-- # 行号 / 播放状态波形 -->
            <td class="py-3.5 text-center font-mono font-medium text-zinc-400 dark:text-zinc-500">
              <!-- 如果正在播放，显示波形动画 -->
              <div 
                v-if="store.currentTrackIndex === store.tracks.findIndex(t => t.id === track.id)"
                class="flex items-end justify-center gap-[2.5px] h-3 w-5 mx-auto"
              >
                <div :class="store.isPlaying ? 'animate-[wave_0.8s_ease-in-out_infinite]' : 'h-1'" class="w-[3px] bg-orange-500 rounded-full" style="height: 60%"></div>
                <div :class="store.isPlaying ? 'animate-[wave_0.8s_ease-in-out_0.2s_infinite]' : 'h-2'" class="w-[3px] bg-orange-500 rounded-full" style="height: 100%"></div>
                <div :class="store.isPlaying ? 'animate-[wave_0.8s_ease-in-out_0.4s_infinite]' : 'h-1.5'" class="w-[3px] bg-orange-500 rounded-full" style="height: 80%"></div>
              </div>
              <span v-else class="group-hover:hidden">{{ index + 1 }}</span>
              <!-- 悬停时显示播放三角按钮 -->
              <span v-if="store.currentTrackIndex !== store.tracks.findIndex(t => t.id === track.id)" class="hidden group-hover:inline-block text-orange-500">
                ▶
              </span>
            </td>

            <!-- 标题 & 收藏按钮 -->
            <td class="py-3.5 pl-6 font-medium">
              <div class="flex items-center gap-3">
                <button 
                  @click="toggleFavorite(track, $event)"
                  class="text-zinc-300 dark:text-zinc-700 hover:text-red-500 dark:hover:text-red-400 transition-colors cursor-pointer"
                >
                  <svg 
                    class="w-4 h-4" 
                    :fill="track.isFavorite ? 'currentColor' : 'none'" 
                    :class="track.isFavorite ? 'text-red-500' : ''"
                    stroke="currentColor" 
                    viewBox="0 0 24 24" 
                    xmlns="http://www.w3.org/2000/svg"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"></path>
                  </svg>
                </button>
                <span 
                  :class="store.currentTrackIndex === store.tracks.findIndex(t => t.id === track.id) 
                    ? 'text-orange-600 dark:text-orange-400 font-bold' 
                    : 'text-zinc-800 dark:text-zinc-150'"
                  class="truncate max-w-[240px]"
                >
                  {{ track.title }}
                </span>
              </div>
            </td>

            <!-- 艺人 -->
            <td 
              :class="store.currentTrackIndex === store.tracks.findIndex(t => t.id === track.id)
                ? 'text-orange-600 dark:text-orange-400 font-semibold' 
                : 'text-zinc-600 dark:text-zinc-400'"
              class="py-3.5 truncate max-w-[180px]"
            >
              {{ track.artist }}
            </td>

            <!-- 专辑 -->
            <td 
              :class="store.currentTrackIndex === store.tracks.findIndex(t => t.id === track.id)
                ? 'text-orange-600 dark:text-orange-400 font-semibold' 
                : 'text-zinc-500 dark:text-zinc-400'"
              class="py-3.5 truncate max-w-[180px]"
            >
              {{ track.album }}
            </td>

            <!-- 时长 -->
            <td class="py-3.5 text-center font-mono text-xs text-zinc-400 dark:text-zinc-500">
              {{ track.duration }}
            </td>

            <!-- 文件格式 -->
            <td class="py-3.5 text-center">
              <span class="px-2 py-0.5 bg-zinc-100 dark:bg-zinc-800/80 text-[10px] font-bold tracking-wider text-zinc-500 dark:text-zinc-400 rounded-md font-mono">
                {{ track.format }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- 搜索空状态 -->
      <div v-if="filteredTracks.length === 0" class="flex flex-col items-center justify-center py-20 text-zinc-400">
        <svg class="w-12 h-12 stroke-current mb-4 opacity-50" fill="none" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
        <p class="text-sm">未找到与“{{ searchQuery }}”相关的歌曲</p>
      </div>
    </div>

    <!-- 底部状态统计栏 -->
    <div class="px-8 py-3 border-t border-zinc-150 dark:border-zinc-800/60 text-xs font-medium text-zinc-400 dark:text-zinc-500 font-sans flex items-center justify-between">
      <span>1248 首歌曲，98.6 GB，累计时长 3 天 14 小时</span>
      <span class="text-[10px] bg-zinc-100 dark:bg-zinc-850 px-2 py-0.5 rounded-full font-mono">本地优先 · 零缓存</span>
    </div>
  </main>
</template>

<style>
/* 歌曲播放指示器波形动画 */
@keyframes wave {
  0%, 100% { transform: scaleY(0.3); }
  50% { transform: scaleY(1); }
}
</style>
