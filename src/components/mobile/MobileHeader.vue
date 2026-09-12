<script setup lang="ts">
import { computed, ref, onBeforeUnmount } from 'vue';
import { ArrowLeft, MoreHorizontal, ChevronDown, Check, Music, Disc, Users, Folder, ListMusic, Clock, Heart } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { registerBackHandler } from '../../composables/useMobileBack';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

// 是否处于详情页（需要显示返回按钮）
const isDetailPage = computed(() => {
  return !!playerStore.activeAlbumId
    || !!playerStore.activeArtistId
    || !!playerStore.activePlaylistId;
});

// 当前页标题
const pageTitle = computed(() => {
  switch (playerStore.activeLibraryTab) {
    case '首页': return '首页';
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

// 是否显示元信息（详情页与首页不显示）
const showMeta = computed(() => !isDetailPage.value && playerStore.activeLibraryTab !== '首页');

function onBack() {
  if (isDetailPage.value) {
    // 详情页返回根级
    playerStore.activeAlbumId = null;
    playerStore.activeArtistId = null;
    playerStore.activePlaylistId = null;
  } else if (uiStore.activeMobileTab === 'library' && playerStore.activeLibraryTab !== '全部歌曲') {
    // 二级分类返回全部歌曲
    playerStore.activeLibraryTab = '全部歌曲';
  } else if (uiStore.activeMobileTab === 'favorites' && playerStore.activeLibraryTab !== '喜欢的音乐') {
    // 二级分类返回我喜欢的音乐
    playerStore.activeLibraryTab = '喜欢的音乐';
  }
}

// 是否显示返回按钮（详情页或二级分类非根页；首页是根页）
const showBackButton = computed(() => {
  if (isDetailPage.value) return true;
  if (playerStore.activeLibraryTab === '首页') return false;
  if (uiStore.activeMobileTab === 'library' && playerStore.activeLibraryTab !== '全部歌曲') return true;
  if (uiStore.activeMobileTab === 'favorites' && playerStore.activeLibraryTab !== '喜欢的音乐') return true;
  return false;
});

/* ============ 二级分类菜单（MOB-002） ============ */
const showCategoryMenu = ref(false);

const libraryCategories = [
  { key: '全部歌曲', label: '全部歌曲', icon: Music },
  { key: '专辑', label: '专辑', icon: Disc },
  { key: '艺术家', label: '艺术家', icon: Users },
  { key: '文件夹', label: '文件夹', icon: Folder },
  { key: '播放列表', label: '播放列表', icon: ListMusic },
  { key: '最近播放', label: '最近播放', icon: Clock },
];

const favoritesCategories = [
  { key: '喜欢的音乐', label: '我喜欢的音乐', icon: Heart },
  { key: '收藏的专辑', label: '收藏的专辑', icon: Disc },
  { key: '收藏的歌手', label: '收藏的歌手', icon: Users },
];

const canSwitchCategory = computed(() => {
  return !isDetailPage.value && (uiStore.activeMobileTab === 'library' || uiStore.activeMobileTab === 'favorites');
});

const activeCategories = computed(() => {
  if (uiStore.activeMobileTab === 'favorites') return favoritesCategories;
  return libraryCategories;
});

function selectCategory(catKey: string) {
  playerStore.activeLibraryTab = catKey as any;
  playerStore.activeAlbumId = null;
  playerStore.activeArtistId = null;
  playerStore.activePlaylistId = null;
  showCategoryMenu.value = false;
}

// 返回拦截：菜单打开时优先关闭
const unregisterBack = registerBackHandler(() => {
  if (showCategoryMenu.value) {
    showCategoryMenu.value = false;
    return true;
  }
  if (showBackButton.value) {
    onBack();
    return true;
  }
  return false;
});
onBeforeUnmount(unregisterBack);
</script>

<template>
  <header
    class="flex items-center bg-bg-canvas flex-shrink-0 select-none"
    style="height: calc(var(--height-mobile-header) + env(safe-area-inset-top, 0px)); padding-top: env(safe-area-inset-top, 0px);"
  >
    <!-- 左：返回按钮 -->
    <div class="flex items-center flex-shrink-0" style="width: 56px;">
      <button
        v-if="showBackButton"
        class="flex items-center justify-center transition-colors-smooth text-text-primary hover:text-brand-orange"
        style="width: 44px; height: 44px;"
        title="返回"
        aria-label="返回"
        @click="onBack"
      >
        <ArrowLeft class="w-[22px] h-[22px]" aria-hidden="true" />
      </button>
    </div>

    <!-- 中：标题 + 元信息（支持点击呼出切换菜单） -->
    <div
      class="flex-1 min-w-0 flex flex-col justify-center cursor-pointer"
      @click="canSwitchCategory ? (showCategoryMenu = true) : undefined"
    >
      <div class="flex items-center gap-1">
        <h1
          class="font-bold text-text-primary tracking-tight leading-tight truncate"
          style="font-size: var(--text-mobile-page-title);"
        >{{ pageTitle }}</h1>
        <ChevronDown
          v-if="canSwitchCategory"
          class="w-4 h-4 text-text-muted transition-transform duration-200"
          :class="{ 'rotate-180': showCategoryMenu }"
        />
      </div>
      <p
        v-if="showMeta"
        class="text-text-muted font-mono leading-relaxed truncate"
        style="font-size: var(--text-11);"
      >{{ metaText }}</p>
    </div>

    <!-- 右：更多操作 / 切换分类按钮 -->
    <div class="flex items-center justify-end flex-shrink-0" style="width: 56px;">
      <button
        v-if="canSwitchCategory"
        class="flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
        style="width: 44px; height: 44px;"
        title="切换分类"
        aria-label="切换分类"
        @click="showCategoryMenu = !showCategoryMenu"
      >
        <MoreHorizontal class="w-[22px] h-[22px]" aria-hidden="true" />
      </button>
    </div>

    <!-- ===== 分类切换抽屉（MOB-002） ===== -->
    <Teleport to="body">
      <div
        v-if="showCategoryMenu"
        class="fixed inset-0 z-[120] bg-black/50 flex items-end justify-center"
        @click.self="showCategoryMenu = false"
      >
        <div class="w-full max-w-[420px] bg-bg-canvas rounded-t-[16px] px-4 pt-4 pb-8 space-y-1 animate-rise">
          <div class="w-10 h-1 rounded-full bg-border-solid mx-auto mb-3"></div>
          <p class="text-[13px] font-semibold text-text-muted px-3 pb-2 uppercase tracking-wider">
            切换{{ uiStore.activeMobileTab === 'favorites' ? '收藏' : '曲库' }}视图
          </p>
          <button
            v-for="cat in activeCategories"
            :key="cat.key"
            class="w-full h-12 px-3 rounded-[10px] flex items-center justify-between text-left transition-colors-smooth active:bg-list-hover"
            :class="playerStore.activeLibraryTab === cat.key ? 'text-brand-orange font-medium bg-brand-orange/10' : 'text-text-primary'"
            @click="selectCategory(cat.key)"
          >
            <div class="flex items-center gap-3">
              <component :is="cat.icon" class="w-5 h-5 flex-shrink-0 opacity-80" />
              <span class="text-[15px]">{{ cat.label }}</span>
            </div>
            <Check v-if="playerStore.activeLibraryTab === cat.key" class="w-4 h-4 text-brand-orange" />
          </button>
        </div>
      </div>
    </Teleport>
  </header>
</template>
