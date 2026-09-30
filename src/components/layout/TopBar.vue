 <script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { ChevronLeft, ChevronRight, Home, Search, Sun, Moon, PanelRight, Settings, Minus, Square, X, Sparkles, Check } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useUiStore } from '../../stores/ui';
import { usePlayerStore } from '../../stores/player';
import { useDesktopModeStore, type ExperienceMode } from '../../stores/desktopMode';
import { toggleWindowMaximize } from '../../composables/useWindowPersistence';

const appWindow = getCurrentWindow();
const uiStore = useUiStore();
const playerStore = usePlayerStore();
// 体验模式（DM-04）：正常/极简两维之一，独立于窗口形态与主题
const desktopMode = useDesktopModeStore();
const experienceMode = computed(() => desktopMode.experienceMode);

/* ============ 体验模式菜单（键盘可达：Esc 关闭、↑↓ 移动、Enter 选择、焦点归还） ============ */
const menuOpen = ref(false);
const menuButton = ref<HTMLElement | null>(null);
const menuRoot = ref<HTMLElement | null>(null);

function closeMenu(refocus = true) {
  if (!menuOpen.value) return;
  menuOpen.value = false;
  if (refocus) menuButton.value?.focus();
}
function toggleMenu() {
  menuOpen.value = !menuOpen.value;
  if (menuOpen.value) {
    // 焦点进菜单（首个未选项）
    requestAnimationFrame(() => {
      const items = menuRoot.value?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]');
      const current = Array.from(items ?? []).find(b => b.dataset.mode === experienceMode.value);
      (current ?? items?.[0])?.focus();
    });
  }
}
function pick(mode: ExperienceMode) {
  void desktopMode.setExperienceMode(mode);
  closeMenu();
}
function onMenuKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.stopPropagation();
    closeMenu();
    return;
  }
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault();
    const items = Array.from(menuRoot.value?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]') ?? []);
    const idx = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.key === 'ArrowDown' ? (idx + 1) % items.length : (idx - 1 + items.length) % items.length;
    items[next]?.focus();
  }
}
function onDocumentClick(e: MouseEvent) {
  if (menuOpen.value && !menuRoot.value?.contains(e.target as Node)) closeMenu(false);
}
if (typeof document !== 'undefined') {
  document.addEventListener('click', onDocumentClick);
}
onBeforeUnmount(() => {
  if (typeof document !== 'undefined') document.removeEventListener('click', onDocumentClick);
});

const minimize = () => appWindow.minimize();
const toggleMaximize = () => toggleWindowMaximize();
const close = () => appWindow.close();
</script>

<template>
  <div
    class="h-(--height-topbar) w-full bg-bg-content/90 backdrop-blur-md flex items-center justify-between px-4 flex-shrink-0 select-none border-b border-border-color/50 transition-colors-smooth"
    data-tauri-drag-region
  >

    <!-- 左侧：全局前后导航 -->
    <div class="flex items-center gap-1 pointer-events-auto">
      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
        :class="playerStore.canGoBack ? 'text-text-secondary hover:text-text-primary hover:bg-bg-hover' : 'text-text-disabled'"
        :disabled="!playerStore.canGoBack"
        title="后退"
        @click="playerStore.goBack()"
      >
        <ChevronLeft class="w-[18px] h-[18px]" />
      </button>
      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
        :class="playerStore.canGoForward ? 'text-text-secondary hover:text-text-primary hover:bg-bg-hover' : 'text-text-disabled'"
        :disabled="!playerStore.canGoForward"
        title="前进"
        @click="playerStore.goForward()"
      >
        <ChevronRight class="w-[18px] h-[18px]" />
      </button>
      <!-- 首页 -->
      <button
        class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
        :class="playerStore.activeLibraryTab === '首页' ? 'bg-list-selected text-text-primary' : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
        title="首页"
        @click="playerStore.goHome()"
      >
        <Home class="w-[18px] h-[18px]" />
      </button>
    </div>

    <!-- 中间：全局搜索占位 -->
    <div class="flex-1 flex justify-center px-4 pointer-events-auto">
      <div class="relative w-full max-w-[420px]">
        <Search class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        <input
          v-model="playerStore.globalSearchQuery"
          type="text"
          placeholder="搜索全局…"
          class="w-full h-[34px] pl-8 pr-3 text-[13px] bg-bg-hover border border-transparent rounded-[8px] text-text-primary placeholder:text-text-muted transition-all duration-200 focus:bg-bg-canvas focus:border-brand-orange/40 focus:ring-2 focus:ring-brand-orange/15 shadow-sm"
        />
      </div>
    </div>

    <!-- 右侧：工具组 + 窗口控制 -->
    <div class="flex items-center gap-1 pointer-events-auto">
      <div class="flex items-center gap-1 mr-3">
        <!-- 体验模式菜单（DM-04）：正常/极简，与主题独立 -->
        <div ref="menuRoot" class="relative">
          <button
            ref="menuButton"
            class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
            :class="menuOpen || experienceMode === 'minimal' ? 'text-brand-orange bg-bg-active' : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
            title="体验模式"
            aria-haspopup="menu"
            :aria-expanded="menuOpen"
            @click="toggleMenu"
            @keydown.down.prevent="menuOpen ? undefined : toggleMenu()"
          >
            <Sparkles class="w-[18px] h-[18px]" />
          </button>
          <div
            v-if="menuOpen"
            role="menu"
            aria-label="体验模式"
            class="absolute right-0 top-[38px] z-50 min-w-[180px] bg-bg-content border border-border-color rounded-[10px] shadow-lg py-1.5"
            @keydown="onMenuKeydown"
          >
            <button
              v-for="m in [
                { mode: 'normal' as ExperienceMode, label: '正常模式', desc: '完整视觉与封面' },
                { mode: 'minimal' as ExperienceMode, label: '极简模式', desc: '纯文字，不加载图片' },
              ]"
              :key="m.mode"
              role="menuitemradio"
              :aria-checked="experienceMode === m.mode"
              :data-mode="m.mode"
              class="w-full flex items-center gap-2.5 px-3 py-2 text-left hover:bg-list-hover focus-visible:bg-list-hover focus-visible:outline-none"
              @click="pick(m.mode)"
            >
              <Check
                class="w-4 h-4 flex-shrink-0"
                :class="experienceMode === m.mode ? 'text-brand-orange' : 'text-transparent'"
              />
              <span class="min-w-0">
                <span class="block text-[13px] text-text-primary leading-tight">{{ m.label }}</span>
                <span class="block text-[11px] text-text-muted leading-tight">{{ m.desc }}</span>
              </span>
            </button>
          </div>
        </div>

        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
          :class="uiStore.isDarkMode ? 'text-brand-orange bg-bg-active' : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
          :title="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
          @click="uiStore.toggleDarkMode()"
        >
          <Moon v-if="!uiStore.isDarkMode" class="w-[18px] h-[18px]" />
          <Sun v-else class="w-[18px] h-[18px]" />
        </button>

        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
          :class="uiStore.isRightSidebarVisible ? 'text-text-secondary bg-bg-hover' : 'text-text-muted hover:text-text-primary hover:bg-bg-hover'"
          title="显示/隐藏信息面板"
          @click="uiStore.toggleRightSidebar()"
        >
          <PanelRight class="w-[18px] h-[18px]" />
        </button>

        <button
          class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
          :class="playerStore.activeLibraryTab === '设置' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary hover:bg-bg-hover'"
          title="设置"
          @click="playerStore.navigateToTab('设置')"
        >
          <Settings class="w-[18px] h-[18px]" />
        </button>
      </div>

      <div class="w-px h-4 bg-border-color mx-1"></div>

      <div class="flex items-center gap-1">
        <button
          @click="minimize"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
          title="最小化"
        >
          <Minus class="w-4 h-4" />
        </button>
        <button
          @click="toggleMaximize"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
          title="最大化"
        >
          <Square class="w-3.5 h-3.5" />
        </button>
        <button
          @click="close"
          class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-white hover:bg-[#E81123] transition-colors-smooth"
          title="关闭"
        >
          <X class="w-4 h-4" />
        </button>
      </div>
    </div>

  </div>
</template>
