<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { store } from "./store/mockStore";
import Sidebar from "./components/Sidebar.vue";
import MainContent from "./components/MainContent.vue";
import RightPanel from "./components/RightPanel.vue";
import PlayerBar from "./components/PlayerBar.vue";

let timer: number | null = null;

onMounted(() => {
  // 模拟播放进度递增逻辑
  timer = window.setInterval(() => {
    if (store.isPlaying) {
      if (store.currentTime < store.currentTrack.durationSec) {
        store.currentTime++;
        
        // 动态根据时间滚动高亮歌词 (简单模拟)
        const currentSec = store.currentTime;
        store.lyrics.forEach((line) => {
          line.isActive = false;
        });
        
        // 寻找最接近当前秒数的歌词行进行激活
        let activeIdx = 0;
        for (let i = 0; i < store.lyrics.length; i++) {
          if (currentSec >= store.lyrics[i].time) {
            activeIdx = i;
          }
        }
        if (store.lyrics[activeIdx]) {
          store.lyrics[activeIdx].isActive = true;
        }
      } else {
        // 放完自动切下一首
        store.nextTrack();
      }
    }
  }, 1000);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
  }
});
</script>

<template>
  <!-- 主容器绑定 dark 类实现 Tailwind 主题切换 -->
  <div :class="{ dark: store.isDarkMode }" class="h-screen w-screen overflow-hidden">
    <div class="flex flex-col h-full bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-50 transition-colors duration-300 font-sans">
      <!-- 主三栏布局 -->
      <div class="flex-1 flex overflow-hidden">
        <!-- 左侧导航栏 -->
        <Sidebar />
        
        <!-- 中间内容区 -->
        <MainContent />
        
        <!-- 右侧歌词面板 -->
        <RightPanel />
      </div>

      <!-- 底部播放控制栏 -->
      <PlayerBar />
    </div>
  </div>
</template>

<style>
/* 可以在这里加入全局的滚动条美化 */
.scrollbar-thin::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
.scrollbar-thin::-webkit-scrollbar-track {
  background: transparent;
}
.scrollbar-thin::-webkit-scrollbar-thumb {
  background: var(--color-zinc-200);
  border-radius: 9999px;
}
.dark .scrollbar-thin::-webkit-scrollbar-thumb {
  background: var(--color-zinc-800);
}
.scrollbar-thin::-webkit-scrollbar-thumb:hover {
  background: var(--color-zinc-300);
}
.dark .scrollbar-thin::-webkit-scrollbar-thumb:hover {
  background: var(--color-zinc-700);
}
</style>