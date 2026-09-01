<script setup lang="ts">
import { LibraryBig, Search, Heart, Settings } from 'lucide-vue-next';
import { useUiStore, type MobileTab } from '../../stores/ui';

const uiStore = useUiStore();

const tabs: Array<{ key: MobileTab; label: string; icon: typeof LibraryBig }> = [
  { key: 'library', label: '曲库', icon: LibraryBig },
  { key: 'search', label: '搜索', icon: Search },
  { key: 'favorites', label: '收藏', icon: Heart },
  { key: 'settings', label: '设置', icon: Settings },
];

function onSelect(key: MobileTab) {
  uiStore.setMobileTab(key);
}
</script>

<template>
  <nav
    class="flex items-center justify-around bg-bg-canvas border-t border-border-color flex-shrink-0"
    style="height: calc(var(--height-mobile-tabbar) + env(safe-area-inset-bottom, 0px)); padding-bottom: env(safe-area-inset-bottom, 0px);"
    aria-label="主导航"
  >
    <button
      v-for="tab in tabs"
      :key="tab.key"
      class="flex flex-col items-center justify-center gap-1 flex-1 transition-colors-smooth"
      :class="uiStore.activeMobileTab === tab.key
        ? 'text-brand-orange'
        : 'text-text-muted hover:text-text-primary'"
      :aria-current="uiStore.activeMobileTab === tab.key ? 'page' : undefined"
      :aria-label="tab.label"
      @click="onSelect(tab.key)"
    >
      <component
        :is="tab.icon"
        class="flex-shrink-0"
        :style="{ width: 'var(--icon-tab)', height: 'var(--icon-tab)' }"
        aria-hidden="true"
      />
      <span
        class="font-medium"
        :style="{ fontSize: 'var(--text-11)' }"
        :class="uiStore.activeMobileTab === tab.key ? 'font-medium' : 'font-normal'"
      >{{ tab.label }}</span>
    </button>
  </nav>
</template>
