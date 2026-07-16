<script setup lang="ts">
import { computed } from 'vue';
import { Sun, Moon, Monitor, Disc3, HardDrive, Server, Info, Scan } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 主题设置 ============ */

function onDarkModeToggle() {
  // 手动切换时关闭跟随系统
  if (uiStore.followSystem) {
    uiStore.setFollowSystem(false);
  }
  uiStore.toggleDarkMode();
}

function onFollowSystemToggle() {
  uiStore.setFollowSystem(!uiStore.followSystem);
}

/* ============ 数据源 ============ */

const sources = computed(() => playerStore.sources);

function scanSource(sourceId: number) {
  playerStore.scanSource(sourceId);
}

function getSourceIcon(kind: 'local' | 'webdav') {
  return kind === 'webdav' ? Server : HardDrive;
}

/* ============ 关于 ============ */

const appVersion = 'v1.1.0';
const isScanning = computed(() =>
  playerStore.sources.some(s => s.lastScanned === 'Scanning...')
);
</script>

<template>
  <div class="flex-1 overflow-y-auto bg-bg-content">

    <!-- ===== 外观 ===== -->
    <section class="px-4 pt-4 pb-1">
      <h2 class="text-text-muted font-semibold uppercase tracking-widest px-1 py-2" style="font-size: var(--text-10);">
        外观
      </h2>

      <div class="rounded-[10px] overflow-hidden bg-bg-canvas border border-border-color">
        <!-- 夜间模式 -->
        <button
          class="w-full flex items-center justify-between px-4 h-12 active:bg-list-hover transition-colors-smooth"
          @click="onDarkModeToggle"
        >
          <div class="flex items-center gap-3">
            <component
              :is="uiStore.isDarkMode ? Moon : Sun"
              class="w-[20px] h-[20px]"
              :class="uiStore.isDarkMode ? 'text-brand-orange' : 'text-text-muted'"
              aria-hidden="true"
            />
            <span class="text-[15px] text-text-primary">夜间模式</span>
          </div>
          <div
            class="w-10 h-6 rounded-full transition-colors-smooth relative"
            :class="uiStore.isDarkMode ? 'bg-brand-orange' : 'bg-text-disabled'"
          >
            <div
              class="w-4 h-4 bg-white rounded-full absolute top-1 transition-transform"
              :class="uiStore.isDarkMode ? 'translate-x-5' : 'translate-x-1'"
            ></div>
          </div>
        </button>

        <div class="h-px bg-border-color"></div>

        <!-- 跟随系统 -->
        <button
          class="w-full flex items-center justify-between px-4 h-12 active:bg-list-hover transition-colors-smooth"
          @click="onFollowSystemToggle"
        >
          <div class="flex items-center gap-3">
            <Monitor class="w-[20px] h-[20px] text-text-muted" aria-hidden="true" />
            <span class="text-[15px] text-text-primary">跟随系统</span>
          </div>
          <div
            class="w-10 h-6 rounded-full transition-colors-smooth relative"
            :class="uiStore.followSystem ? 'bg-brand-orange' : 'bg-text-disabled'"
          >
            <div
              class="w-4 h-4 bg-white rounded-full absolute top-1 transition-transform"
              :class="uiStore.followSystem ? 'translate-x-5' : 'translate-x-1'"
            ></div>
          </div>
        </button>
      </div>
    </section>

    <!-- ===== 数据源 ===== -->
    <section class="px-4 py-1">
      <div class="flex items-center justify-between px-1 py-2">
        <h2 class="text-text-muted font-semibold uppercase tracking-widest" style="font-size: var(--text-10);">
          数据源
        </h2>
        <span class="text-text-muted font-mono" style="font-size: var(--text-11);">
          {{ sources.length }} 个
        </span>
      </div>

      <div v-if="sources.length === 0" class="flex flex-col items-center justify-center py-8 gap-2 text-text-muted">
        <Disc3 class="w-8 h-8 text-text-disabled" aria-hidden="true" />
        <span class="text-[13px]">暂无数据源</span>
      </div>

      <div v-else class="space-y-[2px]">
        <div
          v-for="source in sources"
          :key="source.id"
          class="flex items-center gap-3 px-4 h-12 rounded-[10px] bg-bg-canvas border border-border-color"
        >
          <component
            :is="getSourceIcon(source.kind)"
            class="w-[18px] h-[18px] text-text-muted flex-shrink-0"
            aria-hidden="true"
          />
          <div class="flex-1 min-w-0">
            <p class="text-[15px] text-text-primary truncate">{{ source.name }}</p>
            <p class="text-text-muted font-mono uppercase truncate" style="font-size: var(--text-11);">
              {{ source.kind === 'webdav' ? 'WebDAV' : '本地' }}
            </p>
          </div>
          <button
            v-if="source.kind === 'local'"
            class="flex-shrink-0 px-3 py-1 rounded-[6px] border border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center gap-1"
            :disabled="isScanning"
            @click="scanSource(source.id)"
          >
            <Scan class="w-[14px] h-[14px]" :class="isScanning ? 'animate-spin' : ''" aria-hidden="true" />
            扫描
          </button>
        </div>
      </div>
    </section>

    <!-- ===== 关于 ===== -->
    <section class="px-4 py-1">
      <h2 class="text-text-muted font-semibold uppercase tracking-widest px-1 py-2" style="font-size: var(--text-10);">
        关于
      </h2>

      <div class="rounded-[10px] overflow-hidden bg-bg-canvas border border-border-color px-4 py-3">
        <div class="flex items-center gap-3">
          <Info class="w-[20px] h-[20px] text-text-muted flex-shrink-0" aria-hidden="true" />
          <div>
            <p class="text-[15px] font-semibold text-text-primary">LUMO 轻音</p>
            <p class="text-text-muted font-mono" style="font-size: var(--text-11);">{{ appVersion }}</p>
          </div>
        </div>
        <p class="text-text-muted mt-2" style="font-size: var(--text-12);">
          本地音乐播放器 · 你的音乐，只属于你
        </p>
      </div>
    </section>

    <!-- 底部留白 -->
    <div class="h-8"></div>
  </div>
</template>
