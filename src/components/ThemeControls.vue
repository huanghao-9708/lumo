<script setup lang="ts">
/**
 * 统一的主题控制组件：夜间模式切换 + UI 主题选择下拉。
 *
 * 抽成共享组件的原因：原来主题切换/UI 选择只存在于 App.vue 全局浮层，
 * 与窗口控制按钮挤在一起；现在要求统一放到各主题侧边栏的「设置」按钮旁边。
 * 4 套主题 (simple / advanced / te / modern) 都引用本组件，保证行为一致。
 *
 * 通过 `variant` 控制细微样式差异（TE 工业风是等宽大写风格）。
 */
import { ref, onMounted, onUnmounted } from 'vue';
import { Sun, Moon, Palette, ChevronDown } from 'lucide-vue-next';
import { useUiStore } from '../stores/ui';

const props = withDefaults(defineProps<{
  /** 渲染风格：'default' 普通风；'mono' 等宽大写（TE 工业风） */
  variant?: 'default' | 'mono';
  /** 图标尺寸 (px)，默认 18 */
  size?: number;
}>(), {
  variant: 'default',
  size: 18,
});

const uiStore = useUiStore();
const isUiDropdownOpen = ref(false);
const root = ref<HTMLElement | null>(null);

const closeDropdown = () => { isUiDropdownOpen.value = false; };

const onClickOutside = (e: MouseEvent) => {
  if (isUiDropdownOpen.value && root.value && !root.value.contains(e.target as Node)) {
    isUiDropdownOpen.value = false;
  }
};

onMounted(() => document.addEventListener('click', onClickOutside));
onUnmounted(() => document.removeEventListener('click', onClickOutside));

const themes: Array<{ key: string; label: string }> = [
  { key: 'theme-simple', label: '极简视图 (Simple)' },
  { key: 'theme-advanced', label: '高级沉浸 (Advanced)' },
  { key: 'theme-te', label: 'TE 工业风 (Teenage)' },
  { key: 'theme-modern', label: '现代优雅 (Modern)' },
];

const isMono = props.variant === 'mono';
const btnBase = isMono
  ? 'uppercase tracking-widest font-mono'
  : '';
</script>

<template>
  <div ref="root" class="flex items-center gap-3 relative">
    <!-- 夜间模式切换 -->
    <button
      @click="uiStore.toggleDarkMode()"
      class="transition-colors hover:text-accent"
      :class="[{ [btnBase]: isMono }, uiStore.isDarkMode ? 'text-accent' : 'text-current']"
      :title="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
    >
      <Sun v-if="uiStore.isDarkMode" :size="size" class="stroke-[1.5]" />
      <Moon v-else :size="size" class="stroke-[1.5]" />
    </button>

    <!-- UI 主题选择下拉 -->
    <div class="relative">
      <button
        @click="isUiDropdownOpen = !isUiDropdownOpen"
        class="flex items-center gap-1.5 transition-colors hover:text-accent"
        :class="btnBase"
        :title="isMono ? 'UI THEME' : '切换 UI 主题'"
      >
        <Palette :size="size" class="stroke-[1.5]" />
        <ChevronDown
          :size="Math.max(12, Math.round(size * 0.7))"
          class="transition-transform duration-200"
          :class="isUiDropdownOpen ? 'rotate-180' : ''"
        />
      </button>

      <transition
        enter-active-class="transition duration-200 ease-out"
        enter-from-class="transform scale-95 opacity-0"
        enter-to-class="transform scale-100 opacity-100"
        leave-active-class="transition duration-150 ease-in"
        leave-from-class="transform scale-100 opacity-100"
        leave-to-class="transform scale-95 opacity-0">
        <div
          v-if="isUiDropdownOpen"
          class="absolute bottom-full mb-2 left-0 w-40 rounded-xl shadow-lg border overflow-hidden backdrop-blur-md bg-bg-base/95 border-border-color z-50">
          <div class="p-1 flex flex-col gap-0.5">
            <button
              v-for="t in themes"
              :key="t.key"
              @click="uiStore.setActiveTheme(t.key); closeDropdown()"
              class="w-full text-left px-3 py-1.5 rounded-lg text-[12px] font-medium transition-colors"
              :class="[
                isMono ? 'font-mono' : '',
                uiStore.activeTheme === t.key
                  ? 'bg-bg-active text-accent'
                  : 'text-text-muted hover:bg-bg-panel'
              ]">
              {{ isMono ? t.label.toUpperCase() : t.label }}
            </button>
          </div>
        </div>
      </transition>
    </div>
  </div>
</template>
