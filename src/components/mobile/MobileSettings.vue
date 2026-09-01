<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Sun, Moon, Monitor, Disc3, HardDrive, Server, Info, Scan, Volume2, Wifi, Plus, Trash2, Database, Image, Music2 } from 'lucide-vue-next';
import { invoke } from '../../utils/tauriInvoke';
import { libraryGetCacheSize } from '../../api/library';
import { playbackGetAudioCacheSize } from '../../api/playback';
import { getAppVersion } from '../../api/platform';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { registerBackHandler } from '../../composables/useMobileBack';

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

/* ===== 添加来源（MA1 A1-3） ===== */
const emit = defineEmits<{ (e: 'add-local'): void }>();

/* ===== 删除来源：二次确认（P1-10 移动端落地） ===== */
const removeTarget = ref<{ id: number; name: string; kind: string } | null>(null);
function askRemove(source: { id: number; name: string; kind: 'local' | 'webdav' }) {
  removeTarget.value = { id: source.id, name: source.name, kind: source.kind };
}
async function confirmRemove() {
  if (!removeTarget.value) return;
  const target = removeTarget.value;
  removeTarget.value = null;
  try {
    await playerStore.removeSource(target.id);
  } catch (e) {
    console.error('remove source failed', e);
  }
}

/* ============ 存储占用（MA1 A1-5） ============ */

const storageInfo = ref<{ db: number; cover: number; audio: number } | null>(null);
async function refreshStorage() {
  const [cover, audio] = await Promise.all([
    libraryGetCacheSize().catch(() => 0),
    playbackGetAudioCacheSize().catch(() => 0),
  ]);
  storageInfo.value = { db: 0, cover, audio };
}
function fmtBytes(n: number): string {
  if (n >= 1024 * 1024 * 1024) return (n / 1024 / 1024 / 1024).toFixed(2) + ' GB';
  if (n >= 1024 * 1024) return (n / 1024 / 1024).toFixed(1) + ' MB';
  if (n >= 1024) return (n / 1024).toFixed(0) + ' KB';
  return n + ' B';
}

/* ============ 关于 ============ */

const appVersion = ref('...');
onMounted(async () => {
  appVersion.value = await getAppVersion().catch(() => '');
  refreshStorage();
});

const isScanning = computed(() =>
  playerStore.sources.some(s => s.lastScanned === 'Scanning...')
);

/* ===== 返回拦截：删除确认弹层打开时优先关闭弹层 ===== */
registerBackHandler(() => {
  if (removeTarget.value) {
    removeTarget.value = null;
    return true;
  }
  return false;
});

/* ============ MA0 Spike（仅开发构建渲染） ============ */

const isDev = import.meta.env.DEV;
const toneBusy = ref(false);
const toneResult = ref('');

async function playTone() {
  toneBusy.value = true;
  toneResult.value = '';
  try {
    await invoke('debug_play_tone', {});
    toneResult.value = '✓ 测试音已播放 3 秒，听见了 = 音频链路 Go';
  } catch (e) {
    toneResult.value = '✗ 播放失败: ' + e;
  } finally {
    toneBusy.value = false;
  }
}

const probeUrl = ref('');
const probeUser = ref('');
const probePass = ref('');
const probeBusy = ref(false);
const probeResult = ref('');

async function runProbe() {
  if (!probeUrl.value) return;
  probeBusy.value = true;
  probeResult.value = '探测中…';
  try {
    const r = await invoke('debug_webdav_probe', {
      baseUrl: probeUrl.value,
      username: probeUser.value || null,
      password: probePass.value || null,
    });
    probeResult.value = '✓ ' + JSON.stringify(r, null, 2);
  } catch (e) {
    probeResult.value = '✗ ' + e;
  } finally {
    probeBusy.value = false;
  }
}
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
              {{ source.kind === 'webdav' ? 'WebDAV' : '本地' }} · {{ source.lastScanned }}
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
          <button
            class="flex-shrink-0 p-1.5 rounded-[6px] text-text-muted active:bg-list-hover transition-colors-smooth"
            :aria-label="`删除来源 ${source.name}`"
            @click="askRemove(source)"
          >
            <Trash2 class="w-[16px] h-[16px]" aria-hidden="true" />
          </button>
        </div>
      </div>

      <!-- 添加本地来源（MA1 A1-3） -->
      <button
        class="mt-3 w-full h-11 rounded-[10px] border border-dashed border-border-solid text-[14px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-2"
        @click="emit('add-local')"
      >
        <Plus class="w-[16px] h-[16px]" aria-hidden="true" />
        添加本地音乐目录
      </button>
    </section>

    <!-- ===== 存储占用（MA1 A1-5） ===== -->
    <section class="px-4 py-1">
      <div class="flex items-center justify-between px-1 py-2">
        <h2 class="text-text-muted font-semibold uppercase tracking-widest" style="font-size: var(--text-10);">
          存储占用
        </h2>
        <button class="text-brand-orange text-[12px] font-medium" @click="refreshStorage">刷新</button>
      </div>
      <div class="rounded-[10px] bg-bg-canvas border border-border-color px-4 py-3 space-y-2">
        <div class="flex items-center gap-3">
          <Database class="w-[18px] h-[18px] text-text-muted flex-shrink-0" aria-hidden="true" />
          <span class="text-[14px] text-text-primary flex-1">曲库数据库</span>
          <span class="text-[13px] text-text-muted font-mono">{{ storageInfo ? fmtBytes(storageInfo.db) : '…' }}</span>
        </div>
        <div class="flex items-center gap-3">
          <Image class="w-[18px] h-[18px] text-text-muted flex-shrink-0" aria-hidden="true" />
          <span class="text-[14px] text-text-primary flex-1">封面缓存</span>
          <span class="text-[13px] text-text-muted font-mono">{{ storageInfo ? fmtBytes(storageInfo.cover) : '…' }}</span>
        </div>
        <div class="flex items-center gap-3">
          <Music2 class="w-[18px] h-[18px] text-text-muted flex-shrink-0" aria-hidden="true" />
          <span class="text-[14px] text-text-primary flex-1">音频缓存</span>
          <span class="text-[13px] text-text-muted font-mono">{{ storageInfo ? fmtBytes(storageInfo.audio) : '…' }}</span>
        </div>
      </div>
    </section>

    <!-- ===== MA0 Spike（仅开发构建） ===== -->
    <section v-if="isDev" class="px-4 py-1">
      <h2 class="text-text-muted font-semibold uppercase tracking-widest px-1 py-2" style="font-size: var(--text-10);">
        MA0 Spike · 仅开发构建
      </h2>

      <div class="rounded-[10px] bg-bg-canvas border border-border-color px-4 py-3 space-y-2">
        <button
          class="w-full h-10 rounded-[8px] bg-brand-orange text-white text-[14px] font-medium active:opacity-80 transition-opacity flex items-center justify-center gap-2"
          :disabled="toneBusy"
          @click="playTone"
        >
          <Volume2 class="w-[16px] h-[16px]" aria-hidden="true" />
          {{ toneBusy ? '播放中…' : '播放测试音（440Hz · 3s）' }}
        </button>

        <div class="h-px bg-border-color"></div>

        <input
          v-model="probeUrl"
          placeholder="WebDAV 地址（https://…）"
          class="w-full h-9 px-3 rounded-[8px] bg-transparent border border-border-color text-[13px] text-text-primary placeholder:text-text-disabled outline-none focus:border-brand-orange"
        />
        <input
          v-model="probeUser"
          placeholder="用户名（可空）"
          autocomplete="off"
          class="w-full h-9 px-3 rounded-[8px] bg-transparent border border-border-color text-[13px] text-text-primary placeholder:text-text-disabled outline-none focus:border-brand-orange"
        />
        <input
          v-model="probePass"
          type="password"
          placeholder="密码（可空）"
          autocomplete="new-password"
          class="w-full h-9 px-3 rounded-[8px] bg-transparent border border-border-color text-[13px] text-text-primary placeholder:text-text-disabled outline-none focus:border-brand-orange"
        />
        <button
          class="w-full h-10 rounded-[8px] border border-border-solid text-[14px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-2"
          :disabled="probeBusy || !probeUrl"
          @click="runProbe"
        >
          <Wifi class="w-[16px] h-[16px]" aria-hidden="true" />
          {{ probeBusy ? '探测中…' : 'WebDAV 连通性探测（PROPFIND）' }}
        </button>

        <pre
          v-if="toneResult || probeResult"
          class="text-text-muted whitespace-pre-wrap break-all font-mono pt-1"
          style="font-size: var(--text-11);"
        >{{ toneResult }}{{ probeResult }}</pre>
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

    <!-- ===== 删除来源确认（P1-10 移动端落地） ===== -->
    <Teleport to="body">
      <div
        v-if="removeTarget"
        class="fixed inset-0 z-[100] bg-black/50 flex items-end justify-center"
        @click.self="removeTarget = null"
      >
        <div class="w-full max-w-[420px] bg-bg-canvas rounded-t-[16px] px-5 pt-5 pb-8 animate-rise">
          <div class="w-10 h-1 rounded-full bg-border-solid mx-auto mb-4"></div>
          <h3 class="text-[16px] font-semibold text-text-primary text-center">删除来源</h3>
          <p class="text-[13px] text-text-muted text-center mt-2 leading-relaxed">
            删除「{{ removeTarget.name }}」将从曲库移除其全部歌曲索引，
            本机已缓存的音频不会删除。此操作不可撤销。
          </p>
          <div class="flex gap-3 mt-5">
            <button
              class="flex-1 h-11 rounded-[10px] bg-bg-hover text-text-secondary text-[15px] font-medium active:opacity-85"
              @click="removeTarget = null"
            >
              取消
            </button>
            <button
              class="flex-1 h-11 rounded-[10px] bg-red-500/90 text-white text-[15px] font-semibold active:opacity-85"
              @click="confirmRemove"
            >
              确认删除
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
