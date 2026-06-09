<script setup lang="ts">
import { store } from "../store/mockStore";

function selectTab(tab: string) {
  store.activeLibraryTab = tab;
}

function selectSource(source: string) {
  store.activeSourceTab = source;
}

function toggleTheme() {
  store.isDarkMode = !store.isDarkMode;
}
</script>

<template>
  <aside class="w-64 border-r flex flex-col h-full bg-zinc-50/80 dark:bg-zinc-950/80 border-zinc-200/60 dark:border-zinc-800/60 transition-colors duration-300 select-none">
    <!-- 品牌 Logo -->
    <div class="px-6 py-5 flex items-center gap-2 border-b border-zinc-200/60 dark:border-zinc-800/60">
      <span class="flex items-center justify-center p-1.5 bg-gradient-to-tr from-orange-500 to-amber-400 rounded-xl shadow-md shadow-orange-500/20">
        <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"></path>
        </svg>
      </span>
      <h1 class="text-xl font-extrabold tracking-tight text-zinc-900 dark:text-zinc-50 font-sans">
        Lumo<span class="text-orange-500 font-medium text-sm ml-1">轻音</span>
      </h1>
    </div>

    <!-- 菜单导航 -->
    <div class="flex-1 overflow-y-auto px-4 py-4 space-y-6 scrollbar-thin">
      <!-- 曲库 -->
      <div>
        <div class="px-3 mb-2 text-xs font-semibold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider">曲库</div>
        <ul class="space-y-1">
          <li>
            <button
              @click="selectTab('全部歌曲')"
              :class="store.activeLibraryTab === '全部歌曲' 
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium' 
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2z"></path>
              </svg>
              <span>全部歌曲</span>
            </button>
          </li>
          <li>
            <button
              @click="selectTab('专辑')"
              :class="store.activeLibraryTab === '专辑'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 100-2 1 1 0 000 2zm6 6a1 1 0 100-2 1 1 0 000 2z"></path>
              </svg>
              <span>专辑</span>
            </button>
          </li>
          <li>
            <button
              @click="selectTab('艺人')"
              :class="store.activeLibraryTab === '艺人'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path>
              </svg>
              <span>艺人</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- 我的 -->
      <div>
        <div class="px-3 mb-2 text-xs font-semibold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider">我的</div>
        <ul class="space-y-1">
          <li>
            <button
              @click="selectTab('收藏歌曲')"
              :class="store.activeLibraryTab === '收藏歌曲'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"></path>
              </svg>
              <span>收藏歌曲</span>
            </button>
          </li>
          <li>
            <button
              @click="selectTab('最近播放')"
              :class="store.activeLibraryTab === '最近播放'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <span>最近播放</span>
            </button>
          </li>
          <li>
            <button
              @click="selectTab('播放队列')"
              :class="store.activeLibraryTab === '播放队列'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
              </svg>
              <span>播放队列</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- 歌单 -->
      <div>
        <div class="px-3 mb-2 flex items-center justify-between">
          <span class="text-xs font-semibold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider">歌单</span>
        </div>
        <ul class="space-y-1">
          <li v-for="playlist in store.playlists" :key="playlist.name">
            <button
              @click="selectTab(playlist.name)"
              :class="store.activeLibraryTab === playlist.name
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center justify-between px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <div class="flex items-center gap-3 truncate">
                <svg class="w-4.5 h-4.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"></path>
                </svg>
                <span class="truncate">{{ playlist.name }}</span>
              </div>
              <span class="text-xs text-zinc-400 dark:text-zinc-500 ml-2">{{ playlist.count }}</span>
            </button>
          </li>
          <li>
            <button class="w-full flex items-center gap-3 px-3 py-2 text-sm text-zinc-400 dark:text-zinc-500 hover:text-orange-500 dark:hover:text-orange-400 hover:bg-zinc-200/30 dark:hover:bg-zinc-900/30 rounded-xl transition-all duration-200 cursor-pointer">
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
              </svg>
              <span>新建歌单</span>
            </button>
          </li>
        </ul>
      </div>

      <!-- 来源 -->
      <div>
        <div class="px-3 mb-2 text-xs font-semibold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider">来源</div>
        <ul class="space-y-1">
          <li>
            <button
              @click="selectSource('本地音乐库')"
              :class="store.activeSourceTab === '本地音乐库'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
              </svg>
              <span>本地音乐库</span>
            </button>
          </li>
          <li>
            <button
              @click="selectSource('NAS (WebDAV)')"
              :class="store.activeSourceTab === 'NAS (WebDAV)'
                ? 'bg-orange-500/10 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 font-medium'
                : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50'"
              class="w-full flex items-center gap-3 px-3 py-2 text-sm rounded-xl transition-all duration-200 cursor-pointer"
            >
              <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"></path>
              </svg>
              <span>NAS (WebDAV)</span>
            </button>
          </li>
        </ul>
      </div>
    </div>

    <!-- 底部设置 & 主题切换 -->
    <div class="px-4 py-4 border-t border-zinc-200/60 dark:border-zinc-800/60 flex items-center justify-between gap-2">
      <!-- 设置 -->
      <button class="p-2.5 text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50 rounded-xl transition-all duration-200 cursor-pointer">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
        </svg>
      </button>

      <!-- 主题切换 -->
      <button 
        @click="toggleTheme" 
        class="p-2.5 text-zinc-500 dark:text-zinc-400 hover:text-orange-500 dark:hover:text-orange-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/50 rounded-xl transition-all duration-200 cursor-pointer"
        :title="store.isDarkMode ? '切换至浅色模式' : '切换至暗色模式'"
      >
        <!-- 太阳 (浅色) / 月亮 (暗色) -->
        <svg v-if="store.isDarkMode" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364-6.364l-.707.707M6.343 17.657l-.707.707m12.728 0l-.707-.707M6.343 6.343l-.707-.707m12.728 12.728A9 9 0 115.636 5.636a9 9 0 0112.728 12.728z"></path>
        </svg>
        <svg v-else class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"></path>
        </svg>
      </button>
    </div>
  </aside>
</template>
