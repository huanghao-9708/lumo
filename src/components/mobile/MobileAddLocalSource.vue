<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import { ChevronLeft, FolderOpen, Scan, Settings, ShieldAlert } from 'lucide-vue-next';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '../../stores/player';
import { usePlatform } from '../../composables/usePlatform';
import { checkAudioPermission, requestAudioPermission, openAppSettings, storageSuggestions } from '../../api/platform';
import { registerBackHandler } from '../../composables/useMobileBack';

const emit = defineEmits<{ (e: 'close'): void }>();

const playerStore = usePlayerStore();
const { isMobile } = usePlatform();

/* ============ 权限门 ============ */

const permState = ref<'loading' | 'granted' | 'denied'>('loading');
const permRequesting = ref(false);

async function refreshPermission() {
  permState.value = 'loading';
  const r = await checkAudioPermission().catch(() => 'denied');
  permState.value = r === 'granted' ? 'granted' : 'denied';
}

async function onRequestPermission() {
  permRequesting.value = true;
  try {
    await requestAudioPermission();
    // 结果经 lumo-permission-result 事件回传，3 秒兜底轮询（真机弹窗可能被延迟）
    await new Promise<void>(resolve => {
      let settled = false;
      let unlisten: (() => void) | null = null;
      const finish = () => {
        if (!settled) {
          settled = true;
          unlisten?.();
          resolve();
        }
      };
      listen<boolean>('lumo-permission-result', e => {
        permState.value = e.payload ? 'granted' : 'denied';
        finish();
      }).then(fn => {
        unlisten = fn;
        setTimeout(async () => {
          await refreshPermission();
          finish();
        }, 3000);
      });
    });
  } finally {
    permRequesting.value = false;
  }
}

function onOpenSettings() {
  openAppSettings();
}

/* ============ 目录选择 ============ */

const suggestions = ref<string[]>([]);
const manualPath = ref('');
const scanning = ref(false);
const scanMsg = ref('');
const pathError = ref('');

onMounted(async () => {
  await refreshPermission();
  suggestions.value = await storageSuggestions().catch(() => []);
});

// 返回拦截：此页在返回键/导航时关闭
const unregisterBack = registerBackHandler(() => {
  emit('close');
  return true;
});
onBeforeUnmount(unregisterBack);

function pickSuggestion(p: string) {
  manualPath.value = p;
  pathError.value = '';
}

async function startScan() {
  const path = manualPath.value.trim();
  if (!path) return;
  scanning.value = true;
  pathError.value = '';
  scanMsg.value = '';
  try {
    await playerStore.addLocalSource(path);
    scanMsg.value = '扫描已启动，可在设置页查看进度';
    setTimeout(() => emit('close'), 1500);
  } catch (e) {
    pathError.value = String(e);
  } finally {
    scanning.value = false;
  }
}

const isReady = () => permState.value === 'granted' && manualPath.value.trim().length > 0;
</script>

<template>
  <div class="h-full w-full flex flex-col bg-bg-canvas" v-if="isMobile">
    <!-- 顶栏 -->
    <div class="flex items-center h-12 px-2 flex-shrink-0">
      <button class="w-9 h-9 flex items-center justify-center rounded-full active:bg-list-hover" aria-label="返回" @click="emit('close')">
        <ChevronLeft class="w-6 h-6 text-text-primary" aria-hidden="true" />
      </button>
      <h1 class="text-[17px] font-semibold text-text-primary">添加本地音乐</h1>
    </div>

    <div class="flex-1 overflow-y-auto px-4 pb-8">
      <!-- 权限门 -->
      <div class="rounded-[12px] border border-border-color bg-bg-content p-4 mb-4">
        <div class="flex items-center gap-3">
          <ShieldAlert v-if="permState !== 'granted'" class="w-6 h-6 text-brand-orange flex-shrink-0" aria-hidden="true" />
          <FolderOpen v-else class="w-6 h-6 text-brand-orange flex-shrink-0" aria-hidden="true" />
          <div class="flex-1 min-w-0">
            <p class="text-[15px] font-semibold text-text-primary">读取音乐文件权限</p>
            <p class="text-[13px] text-text-muted mt-0.5 leading-snug">
              Lumo 需要访问你设备上的音频文件才能建立曲库。所有数据仅保存在本机。
            </p>
          </div>
        </div>

        <div class="mt-3">
          <button
            v-if="permState === 'granted'"
            class="w-full h-11 rounded-[10px] bg-bg-hover text-text-secondary text-[14px] font-medium flex items-center justify-center gap-2"
            @click="onOpenSettings"
          >
            <span class="text-[13px]">✓ 已授权</span>
            <span class="text-[13px] text-text-muted">（点此到系统设置查看）</span>
          </button>
          <button
            v-else-if="permState === 'denied'"
            class="w-full h-11 rounded-[10px] bg-brand-orange text-white text-[14px] font-medium active:opacity-85 transition-opacity"
            :disabled="permRequesting"
            @click="onRequestPermission"
          >
            {{ permRequesting ? '请求中…' : '允许访问音乐文件' }}
          </button>
          <button
            v-else
            class="w-full h-11 rounded-[10px] bg-bg-hover text-text-secondary text-[14px] font-medium"
            disabled
          >
            检查中…
          </button>

          <p v-if="permState === 'denied'" class="text-[12px] text-text-muted mt-2 flex items-center gap-1">
            <Settings class="w-3.5 h-3.5" aria-hidden="true" />
            若已在系统里允许但这里仍提示拒绝，请到应用详情页开启「媒体和照片」权限。
          </p>
        </div>
      </div>

      <!-- 目录选择 -->
      <template v-if="permState === 'granted'">
        <p class="text-[13px] font-semibold text-text-muted uppercase tracking-widest mb-2">选择目录</p>

        <div class="space-y-2 mb-3">
          <button
            v-for="p in suggestions"
            :key="p"
            class="w-full text-left rounded-[10px] border border-border-color bg-bg-content px-3 py-2.5 active:bg-list-hover transition-colors"
            :class="manualPath === p ? 'border-brand-orange' : ''"
            @click="pickSuggestion(p)"
          >
            <span class="text-[14px] text-text-primary truncate">{{ p }}</span>
          </button>
          <p v-if="suggestions.length === 0" class="text-[13px] text-text-muted px-1">
            未发现常见音乐目录，请手动输入路径（如 /storage/emulated/0/Music）。
          </p>
        </div>

        <input
          v-model="manualPath"
          type="text"
          placeholder="/storage/emulated/0/Music"
          class="w-full h-11 px-3 rounded-[10px] bg-bg-content border border-border-color text-[14px] text-text-primary placeholder:text-text-disabled outline-none focus:border-brand-orange"
        />
        <p v-if="pathError" class="text-[12px] text-red-400 mt-1.5">{{ pathError }}</p>
        <p v-else-if="scanMsg" class="text-[12px] text-brand-orange mt-1.5">{{ scanMsg }}</p>

        <button
          class="mt-4 w-full h-12 rounded-[10px] bg-brand-orange text-white text-[15px] font-semibold active:opacity-85 transition-opacity flex items-center justify-center gap-2"
          :disabled="!isReady() || scanning"
          @click="startScan"
        >
          <Scan class="w-5 h-5" :class="scanning ? 'animate-spin' : ''" aria-hidden="true" />
          {{ scanning ? '正在添加…' : '添加并扫描' }}
        </button>
      </template>
    </div>
  </div>
</template>
