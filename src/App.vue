<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { usePlayerStore } from './stores/player';

import SidebarLeft from './components/layout/SidebarLeft.vue';
import TopBar from './components/layout/TopBar.vue';
import MainContent from './components/layout/MainContent.vue';
import SidebarRight from './components/layout/SidebarRight.vue';
import BottomPlayer from './components/layout/BottomPlayer.vue';
import NowPlayingImmersive from './components/layout/NowPlayingImmersive.vue';
import CreatePlaylistModal from './components/shared/CreatePlaylistModal.vue';
import { useUiStore } from './stores/ui';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

// 键盘快捷键监听
const handleGlobalKeyDown = (e: KeyboardEvent) => {
  // 判断当前焦点是否在输入框
  const activeEl = document.activeElement;
  if (activeEl && (
    activeEl.tagName === 'INPUT' || 
    activeEl.tagName === 'TEXTAREA' || 
    activeEl.getAttribute('contenteditable') === 'true'
  )) {
    return;
  }

  // 1. 空格键播放/暂停
  if (e.code === 'Space') {
    e.preventDefault();
    playerStore.togglePlay();
  }

  // 2. Ctrl + 左右箭头：切歌
  if (e.ctrlKey && e.code === 'ArrowRight') {
    e.preventDefault();
    playerStore.nextTrack();
  } else if (e.ctrlKey && e.code === 'ArrowLeft') {
    e.preventDefault();
    playerStore.prevTrack();
  }

  // 3. 左右箭头：快退 / 快进 5 秒
  else if (e.code === 'ArrowRight') {
    e.preventDefault();
    const newPos = Math.min(playerStore.durationMs, playerStore.progressMs + 5000);
    playerStore.seek(newPos);
  } else if (e.code === 'ArrowLeft') {
    e.preventDefault();
    const newPos = Math.max(0, playerStore.progressMs - 5000);
    playerStore.seek(newPos);
  }

  // 4. 上下箭头：增减音量
  else if (e.code === 'ArrowUp') {
    e.preventDefault();
    const newVol = Math.min(100, playerStore.volume + 5);
    playerStore.setVolume(newVol);
  } else if (e.code === 'ArrowDown') {
    e.preventDefault();
    const newVol = Math.max(0, playerStore.volume - 5);
    playerStore.setVolume(newVol);
  }
};

onMounted(async () => {
  window.addEventListener('keydown', handleGlobalKeyDown);
  // 1. 恢复播放会话（队列 / 进度 / 音量）
  await playerStore.restoreSession();
  // 2. 拉取侧边栏与库的基础数据（并行）
  await Promise.all([
    playerStore.fetchPlaylists(),
    playerStore.fetchSources(),
    playerStore.fetchAlbums(true),
    playerStore.fetchArtists(true),
  ]);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleGlobalKeyDown);
});
</script>

<template>
  <!-- Global Workspace -->
  <div class="h-screen w-screen flex flex-col bg-bg-canvas text-text-primary overflow-hidden font-sans">
    
    <!-- Top Area -->
    <div class="flex-1 flex overflow-hidden">
      
      <!-- Region 01: Sidebar (w: 240px) -->
      <SidebarLeft />
      
      <!-- Divider A (Sidebar ↓ Content/TopBar) -->
      <div class="w-px h-full bg-border-color shrink-0"></div>

      <!-- Right Side Container -->
      <div class="flex-1 flex flex-col min-w-0">
        
        <!-- Region 02: Top Bar (h: 60px) -->
        <TopBar />
        
        <!-- Divider B (Top Bar ↓ Content) -->
        <div class="h-px w-full bg-border-color shrink-0"></div>

        <!-- Content & Inspector Container -->
        <div class="flex-1 flex overflow-hidden relative">

          <!-- Region 03: Content Area (flex-1) -->
          <MainContent />

          <!-- Region 04: Inspector Panel (浮层，不占内容区空间) -->
          <SidebarRight />
        </div>
      </div>
    </div>

    <!-- Divider D (Playback ↓ Workspace) -->
    <div class="h-px w-full bg-border-color shrink-0"></div>

    <!-- Region 05: Playback Bar (h: 110px) -->
    <BottomPlayer />

    <!-- 沉浸式播放页（覆盖整窗，z-[200]；进/出为抽屉式上下滑动） -->
    <Transition name="np-drawer">
      <NowPlayingImmersive v-if="uiStore.isImmersiveView" />
    </Transition>

    <CreatePlaylistModal v-if="playerStore.isCreatePlaylistModalOpen" />

  </div>
</template>

<style>
/* 全局重置或覆盖样式 */

/* 沉浸式播放页抽屉式进/出动画（LDL：250ms ease-out，禁弹簧/缩放）。
   进入：从下往上滑入；退出：从上往下滑出（打开的反方向）。 */
.np-drawer-enter-active,
.np-drawer-leave-active {
  transition: transform 0.25s ease-out, opacity 0.25s ease-out;
}
.np-drawer-enter-from {
  transform: translateY(100%);
  opacity: 0;
}
.np-drawer-enter-to {
  transform: translateY(0);
  opacity: 1;
}
.np-drawer-leave-from {
  transform: translateY(0);
  opacity: 1;
}
.np-drawer-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>