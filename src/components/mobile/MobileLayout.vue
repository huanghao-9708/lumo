<script setup lang="ts">
import { computed, watch } from 'vue';
import { Disc3, Play, Pause, SkipBack, SkipForward, Loader2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useArtworkSrc } from '../../composables/useArtworkSrc';
import { initMobileBackNavigation } from '../../composables/useMobileBack';
import MobileHeader from './MobileHeader.vue';
import MobileContentView from './MobileContentView.vue';
import MobileSearch from './MobileSearch.vue';
import MobileTabBar from './MobileTabBar.vue';

// A1-7：Android 返回键 → 视图栈（在 setup 中初始化一次）
void initMobileBackNavigation();

/**
 * 移动端布局外壳。
 *
 * 结构（从上到下）：
 *
 *   ┌──────────────────────┐
 *   │    MobileHeader       │  56px + safe-area-top
 *   ├──────────────────────┤
 *   │                       │
 *   │    MobileContentView  │  flex-1（列表/网格/详情/搜索/Settings）
 *   │                       │
 *   ├──────────────────────┤
 *   │    Mini Player        │  64px（播放时显示，点击展开 Now Playing）
 *   ├──────────────────────┤
 *   │    MobileTabBar       │  56px + safe-area-bottom
 *   └──────────────────────┘
 */

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ Tab → activeLibraryTab 同步 ============ */

watch(() => uiStore.activeMobileTab, (tab) => {
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activePlaylistId = null;

  switch (tab) {
    case 'home':
      playerStore.activeLibraryTab = '首页';
      break;
    case 'library':
      if (!isLibraryTab(playerStore.activeLibraryTab)) {
        playerStore.activeLibraryTab = '全部歌曲';
      }
      break;
    case 'search':
      break;
    case 'favorites':
      if (!isFavoritesTab(playerStore.activeLibraryTab)) {
        playerStore.activeLibraryTab = '喜欢的音乐';
      }
      break;
    case 'settings':
      playerStore.activeLibraryTab = '设置';
      break;
  }
}, { immediate: true });

function isLibraryTab(tab: string): boolean {
  return ['全部歌曲', '专辑', '艺术家', '文件夹', '播放列表', '最近播放'].includes(tab);
}

function isFavoritesTab(tab: string): boolean {
  return ['喜欢的音乐', '收藏的专辑', '收藏的歌手'].includes(tab);
}

/* ============ 视图判定 ============ */

const hasCurrentTrack = computed(() => !!playerStore.currentTrack);
const isSearchTab = computed(() => uiStore.activeMobileTab === 'search');
const showContent = computed(() => uiStore.activeMobileTab !== 'search');

/* ============ Mini Player：封面图 ============ */

const miniCoverSrc = useArtworkSrc(() => playerStore.currentTrack?.cover_artwork_id ?? null);

/* ============ Mini Player → Now Playing ============ */

function openNowPlaying() {
  uiStore.openImmersiveView();
}
</script>

<template>
  <div class="h-full w-full flex flex-col bg-bg-canvas overflow-hidden">

    <!-- Top Header -->
    <MobileHeader />

    <!-- Divider B (Header ↓ Content) -->
    <div class="h-px w-full bg-border-color shrink-0"></div>

    <!-- Content Area -->
    <div class="flex-1 flex flex-col bg-bg-content overflow-hidden" style="min-height: 0;">

      <!-- 搜索 Tab → MobileSearch -->
      <MobileSearch v-if="isSearchTab" />

      <!-- 曲库 / 收藏 / 设置 Tab → 真实内容 -->
      <MobileContentView v-if="showContent" />

    </div>

    <!-- Mini Player（播放时显示） -->
    <template v-if="hasCurrentTrack">
      <div class="h-px w-full bg-border-color shrink-0"></div>
      <div
        class="flex items-center bg-bg-canvas flex-shrink-0 px-4 gap-2 cursor-pointer active:bg-list-hover transition-colors-smooth"
        style="height: var(--height-mobile-miniplayer);"
        role="button"
        :aria-label="`正在播放：${playerStore.currentTrack?.title}`"
        @click="openNowPlaying"
      >
        <!-- 封面 -->
        <div class="w-10 h-10 rounded-[6px] bg-bg-hover overflow-hidden flex-shrink-0 flex items-center justify-center">
          <img v-if="miniCoverSrc" :src="miniCoverSrc" class="w-full h-full object-cover" alt="cover" />
          <Disc3 v-else class="w-5 h-5 text-text-disabled" aria-hidden="true" />
        </div>

        <!-- 曲名 + 艺术家 -->
        <div class="flex-1 min-w-0">
          <p class="text-[15px] font-semibold text-text-primary truncate leading-tight">
            {{ playerStore.currentTrack?.title ?? '未在播放' }}
          </p>
          <p class="text-[13px] text-text-muted truncate leading-tight">
            {{ playerStore.currentTrack?.artist }}
          </p>
        </div>

        <!-- Transport -->
        <button
          class="text-text-primary active:text-brand-orange transition-colors-smooth flex items-center justify-center flex-shrink-0"
          style="width: 44px; height: 44px;"
          aria-label="上一首"
          @click.stop="playerStore.prevTrack()"
        >
          <SkipBack class="w-[20px] h-[20px] fill-current" aria-hidden="true" />
        </button>
        <button
          class="text-text-primary active:text-brand-orange transition-colors-smooth flex items-center justify-center flex-shrink-0"
          style="width: 44px; height: 44px;"
          :aria-label="playerStore.isPlaying ? '暂停' : '播放'"
          @click.stop="playerStore.togglePlay()"
        >
          <Loader2 v-if="playerStore.isBuffering" class="w-[22px] h-[22px] animate-spin text-brand-orange" aria-hidden="true" />
          <Pause v-else-if="playerStore.isPlaying" class="w-[24px] h-[24px] fill-current" aria-hidden="true" />
          <Play v-else class="w-[24px] h-[24px] fill-current ml-0.5" aria-hidden="true" />
        </button>
        <button
          class="text-text-primary active:text-brand-orange transition-colors-smooth flex items-center justify-center flex-shrink-0"
          style="width: 44px; height: 44px;"
          aria-label="下一首"
          @click.stop="playerStore.nextTrack()"
        >
          <SkipForward class="w-[20px] h-[20px] fill-current" aria-hidden="true" />
        </button>
      </div>
    </template>

    <!-- Tab Bar -->
    <MobileTabBar />

  </div>
</template>
