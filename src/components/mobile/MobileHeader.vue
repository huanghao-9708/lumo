<script setup lang="ts">
import { computed } from 'vue';
import { ArrowLeft, MoreHorizontal } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();

/**
 * 移动端顶栏（56px）。
 *
 * 替代桌面端 TopBar（60px）。桌面 TopBar 承担窗口控制 + 主题切换，
 * 移动端无窗口控制，Header 聚焦于：当前页标题 + 返回 + 更多操作。
 *
 * 布局：
 *   [← 返回]    标题 / 副标题    [⋯ 更多]
 *
 * 返回按钮仅在「详情页」出现（有 activeAlbumId / activeArtistId / activePlaylistId）。
 * Tab 根级页面无返回按钮。
 */

// 是否处于详情页（需要显示返回按钮）
const isDetailPage = computed(() => {
  return !!playerStore.activeAlbumId
    || !!playerStore.activeArtistId
    || !!playerStore.activePlaylistId;
});

// 当前页标题
const pageTitle = computed(() => {
  switch (playerStore.activeLibraryTab) {
    case '最近播放': return '最近播放';
    case '喜欢的音乐': return '我喜欢的音乐';
    case '收藏的专辑': return '收藏的专辑';
    case '收藏的歌手': return '收藏的歌手';
    case '专辑': return isDetailPage.value ? '专辑详情' : '专辑';
    case '艺术家': return isDetailPage.value ? '艺术家详情' : '艺术家';
    case '文件夹': return '文件夹';
    case '播放列表': return isDetailPage.value ? '歌单详情' : '播放列表';
    case '智能歌单': return '智能歌单';
    case '设置': return '设置';
    default: return '全部歌曲';
  }
});

// 元信息（歌曲数 / 专辑数等）
const metaText = computed(() => {
  if (playerStore.activeLibraryTab === '专辑') return `${playerStore.albumsTotalCount.toLocaleString()} 张专辑`;
  if (playerStore.activeLibraryTab === '艺术家') return `${playerStore.artistsTotalCount.toLocaleString()} 位艺术家`;
  if (playerStore.activeLibraryTab === '文件夹') return `${playerStore.localSources.length} 个数据源`;
  if (playerStore.activeLibraryTab === '喜欢的音乐') return `${playerStore.libraryCounts.favorite_tracks.toLocaleString()} 首歌曲`;
  if (playerStore.activeLibraryTab === '收藏的专辑') return `${playerStore.libraryCounts.favorite_albums.toLocaleString()} 张专辑`;
  if (playerStore.activeLibraryTab === '收藏的歌手') return `${playerStore.libraryCounts.favorite_artists.toLocaleString()} 位艺术家`;
  return `${playerStore.tracksTotalCount.toLocaleString()} 首歌曲`;
});

// 是否显示元信息（详情页不显示）
const showMeta = computed(() => !isDetailPage.value);

function onBack() {
  // 返回 Tab 根级：清除所有详情 ID
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activePlaylistId = null;
}
</script>

<template>
  <header
    class="flex items-center bg-bg-canvas flex-shrink-0 select-none"
    style="height: var(--height-mobile-header); padding-top: env(safe-area-inset-top);"
  >
    <!-- 左：返回按钮（仅详情页显示） -->
    <div class="flex items-center flex-shrink-0" style="width: 56px;">
      <button
        v-if="isDetailPage"
        class="flex items-center justify-center transition-colors-smooth text-text-primary hover:text-brand-orange"
        style="width: 44px; height: 44px;"
        title="返回"
        aria-label="返回"
        @click="onBack"
      >
        <ArrowLeft class="w-[22px] h-[22px]" aria-hidden="true" />
      </button>
    </div>

    <!-- 中：标题 + 元信息 -->
    <div class="flex-1 min-w-0 flex flex-col justify-center">
      <h1
        class="font-bold text-text-primary tracking-tight leading-tight truncate"
        style="font-size: var(--text-mobile-page-title);"
      >{{ pageTitle }}</h1>
      <p
        v-if="showMeta"
        class="text-text-muted font-mono leading-relaxed truncate"
        style="font-size: var(--text-11);"
      >{{ metaText }}</p>
    </div>

    <!-- 右：更多操作（占位，M5 实现 Action Sheet） -->
    <div class="flex items-center justify-end flex-shrink-0" style="width: 56px;">
      <button
        class="flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
        style="width: 44px; height: 44px;"
        title="更多"
        aria-label="更多操作"
      >
        <MoreHorizontal class="w-[22px] h-[22px]" aria-hidden="true" />
      </button>
    </div>
  </header>
</template>
