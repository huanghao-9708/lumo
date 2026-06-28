<script setup lang="ts">
import { 
  Activity, Disc, User, Heart, Folder, Clock, ListMusic, List, Plus, Music
} from 'lucide-vue-next';
</script>

<template>
  <div class="w-[240px] h-full bg-bg-canvas flex flex-col flex-shrink-0 select-none">
    <!-- Logo -->
    <div class="px-8 pt-8 pb-6 cursor-pointer" data-tauri-drag-region>
      <h1 class="text-xl font-bold tracking-[0.15em] text-text-primary mb-2">LUMO</h1>
      <p class="text-[9px] tracking-widest text-text-muted font-mono uppercase leading-tight">Local Music System</p>
      <p class="text-[9px] text-text-muted/60 font-mono mt-0.5">v1.0.0</p>
    </div>

    <!-- Scrollable Nav -->
    <div class="flex-1 overflow-y-auto px-5 pb-4">
      
      <!-- LIBRARY -->
      <div class="mb-6">
        <h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">Library</h2>
        
        <ul class="space-y-[2px]">
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] bg-list-selected text-text-primary transition-smooth">
              <Activity class="w-[16px] h-[16px] mr-3 flex-shrink-0" />
              <span class="text-[13px] font-medium flex-1">全部歌曲</span>
              <span class="text-[11px] font-mono text-text-secondary tabular-nums">12,483</span>
              <div class="w-[6px] h-[6px] rounded-full bg-brand-orange ml-2 flex-shrink-0"></div>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <Disc class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">专辑</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">1,246</span>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <User class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">艺术家</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">643</span>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <Heart class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">作曲家</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">128</span>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <Folder class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">文件夹</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">892</span>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <Clock class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">最近播放</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">200</span>
            </a>
          </li>
          
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <ListMusic class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">播放列表</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">28</span>
            </a>
          </li>
        </ul>
      </div>

      <!-- PLAYLISTS -->
      <div>
        <h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">Playlists</h2>
        
        <ul class="space-y-[2px]">
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">日常音乐</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">58</span>
            </a>
          </li>
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">工作专注</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">24</span>
            </a>
          </li>
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">放松时刻</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">36</span>
            </a>
          </li>
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">90s 精选</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">57</span>
            </a>
          </li>
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">旅行音乐</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">42</span>
            </a>
          </li>
          <li>
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-primary hover:bg-list-hover transition-smooth">
              <Music class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" />
              <span class="text-[13px] flex-1">古典精选</span>
              <span class="text-[11px] font-mono text-text-muted tabular-nums">31</span>
            </a>
          </li>
          
          <li class="mt-3">
            <a href="#" class="flex items-center px-3 py-[7px] rounded-[6px] text-text-muted hover:bg-list-hover hover:text-text-primary transition-smooth">
              <Plus class="w-[16px] h-[16px] mr-3 flex-shrink-0" />
              <span class="text-[13px] flex-1">新建播放列表</span>
            </a>
          </li>
        </ul>
      </div>

    </div>
  </div>
</template>
