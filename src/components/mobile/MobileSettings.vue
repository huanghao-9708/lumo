<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { Sun, Moon, Monitor, Disc3, HardDrive, Server, Info, Scan, Volume2, Wifi, Plus, Trash2, Database, Image, Music2, CloudUpload, CloudDownload, RefreshCw, AlertCircle } from 'lucide-vue-next';
import { invoke } from '../../utils/tauriInvoke';
import { storageGetDbSize, libraryGetCacheSize } from '../../api/library';
import { playbackGetAudioCacheSize } from '../../api/playback';
import { getAppVersion, restartApp } from '../../api/platform';
import { syncGetConfig, syncUploadNow, syncRestoreNow, type SyncConfig } from '../../api/sync';
import { APP_NAME_CN, APP_VERSION_LABEL, APP_TAGLINE } from '../../config/appInfo';
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

/* ===== 添加来源（MA1 A1-3, MA3 A3-2） ===== */
const emit = defineEmits<{
  (e: 'add-local'): void;
  (e: 'add-webdav'): void;
}>();

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
  const [db, cover, audio] = await Promise.all([
    storageGetDbSize().catch(() => 0),
    libraryGetCacheSize().catch(() => 0),
    playbackGetAudioCacheSize().catch(() => 0),
  ]);
  storageInfo.value = { db, cover, audio };
}
function fmtBytes(n: number): string {
  if (n >= 1024 * 1024 * 1024) return (n / 1024 / 1024 / 1024).toFixed(2) + ' GB';
  if (n >= 1024 * 1024) return (n / 1024 / 1024).toFixed(1) + ' MB';
  if (n >= 1024) return (n / 1024).toFixed(0) + ' KB';
  return n + ' B';
}

/* ============ 云端备份与恢复（MA3 A3-5） ============ */

const syncConfig = ref<SyncConfig | null>(null);
const syncing = ref(false);
const syncFeedback = ref<{ type: 'ok' | 'err'; text: string } | null>(null);

async function refreshSyncConfig() {
  syncConfig.value = await syncGetConfig().catch(() => null);
}

async function onUploadBackup() {
  if (syncing.value) return;
  syncing.value = true;
  syncFeedback.value = null;
  try {
    const res = await syncUploadNow();
    syncFeedback.value = { type: 'ok', text: `备份成功（${fmtBytes(res.bytes_uploaded)}）` };
    await refreshSyncConfig();
  } catch (e: any) {
    syncFeedback.value = { type: 'err', text: e?.message || '备份失败，请先在电脑端或设置中配置 WebDAV 同步' };
  } finally {
    syncing.value = false;
  }
}

const showRestoreDialog = ref(false);
const restoreInput = ref('');
const restoring = ref(false);
const showRestartConfirm = ref(false);

function askRestore() {
  restoreInput.value = '';
  showRestoreDialog.value = true;
}

async function confirmRestore() {
  if (restoreInput.value.trim() !== '恢复' || restoring.value) return;
  restoring.value = true;
  try {
    await syncRestoreNow();
    showRestoreDialog.value = false;
    showRestartConfirm.value = true;
  } catch (e: any) {
    syncFeedback.value = { type: 'err', text: e?.message || '恢复失败，远端可能未备份或数据已损坏' };
  } finally {
    restoring.value = false;
  }
}

function onRestartNow() {
  restartApp();
}

/* ============ 检查更新与诊断（MA5 A5-4, A5-5） ============ */

const checkingUpdate = ref(false);
const updateInfo = ref<{ version: string; notes: string; url: string } | null>(null);
const updateMsg = ref('');

async function onCheckUpdate() {
  if (checkingUpdate.value) return;
  checkingUpdate.value = true;
  updateMsg.value = '';
  try {
    const res = await fetch('https://api.github.com/repos/huanghao-9708/lumo/releases/latest');
    if (!res.ok) {
      updateMsg.value = '暂无法连接到更新服务器';
      return;
    }
    const data = await res.json();
    const latestTag = (data.tag_name || '').replace(/^v/, '');
    const current = (appVersion.value || '').replace(/^v/, '');
    if (latestTag && latestTag !== current) {
      updateInfo.value = {
        version: data.tag_name,
        notes: data.body || '新版本已发布，包含体验改进与稳定性优化。',
        url: data.html_url || 'https://github.com/huanghao-9708/lumo/releases',
      };
    } else {
      updateMsg.value = '当前已是最新版本';
    }
  } catch (_e) {
    updateMsg.value = '检查失败：请检查网络连接';
  } finally {
    checkingUpdate.value = false;
  }
}

function openDownloadUrl(url: string) {
  window.open(url, '_blank');
}

const logExported = ref(false);
async function onExportDiagnostics() {
  try {
    const info = [
      `=== LUMO 移动端诊断信息 ===`,
      `应用版本: ${appVersion.value}`,
      `数据源数量: ${playerStore.sources.length}`,
      `曲库歌曲总数: ${playerStore.tracksTotalCount}`,
      `存储占用: 封面 ${fmtBytes(storageInfo.value?.cover || 0)}, 音频缓存 ${fmtBytes(storageInfo.value?.audio || 0)}`,
      `网络环境: ${navigator.onLine ? '在线' : '离线'}`,
      `诊断生成时间: ${new Date().toISOString()}`,
    ].join('\n');
    await navigator.clipboard.writeText(info);
    logExported.value = true;
    setTimeout(() => { logExported.value = false; }, 3000);
  } catch (e) {
    console.error('Failed to copy diagnostics', e);
  }
}

/* ============ 关于 ============ */

const appVersion = ref('...');
onMounted(async () => {
  appVersion.value = await getAppVersion().catch(() => '');
  refreshStorage();
  refreshSyncConfig();
});

const isScanning = computed(() =>
  playerStore.sources.some(s => s.lastScanned === 'Scanning...')
);

/* ===== 返回拦截：弹层打开时优先关闭弹层 ===== */
const unregisterBack = registerBackHandler(() => {
  if (updateInfo.value) {
    updateInfo.value = null;
    return true;
  }
  if (showRestartConfirm.value) {
    showRestartConfirm.value = false;
    return true;
  }
  if (showRestoreDialog.value) {
    showRestoreDialog.value = false;
    return true;
  }
  if (removeTarget.value) {
    removeTarget.value = null;
    return true;
  }
  return false;
});
onBeforeUnmount(unregisterBack);

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

    <!-- ===== 隐私（在线元数据，默认关闭） ===== -->
    <section class="px-4 py-1">
      <div class="flex items-center justify-between px-1 py-2">
        <h2 class="text-text-muted font-semibold uppercase tracking-widest" style="font-size: var(--text-10);">
          隐私
        </h2>
      </div>
      <div class="bg-bg-canvas border border-border-color rounded-[12px] overflow-hidden">
        <!-- 在线歌词匹配 -->
        <button
          class="w-full flex items-center justify-between px-4 h-12 active:bg-list-hover transition-colors-smooth"
          @click="uiStore.setFetchLyricsOnline(!uiStore.fetchLyricsOnline)"
        >
          <span class="text-[15px] text-text-primary text-left">在线歌词匹配</span>
          <div
            class="w-10 h-6 rounded-full transition-colors-smooth relative shrink-0"
            :class="uiStore.fetchLyricsOnline ? 'bg-brand-orange' : 'bg-text-disabled'"
          >
            <div
              class="w-4 h-4 bg-white rounded-full absolute top-1 transition-transform"
              :class="uiStore.fetchLyricsOnline ? 'translate-x-5' : 'translate-x-1'"
            ></div>
          </div>
        </button>
        <p class="px-4 pb-2 text-text-muted" style="font-size: var(--text-10);">
          未命中本地歌词时，向 LRCLIB 发送歌名、艺人、专辑与时长
        </p>
        <div class="h-px bg-border-color"></div>
        <!-- 在线封面匹配 -->
        <button
          class="w-full flex items-center justify-between px-4 h-12 active:bg-list-hover transition-colors-smooth"
          @click="uiStore.setFetchCoversOnline(!uiStore.fetchCoversOnline)"
        >
          <span class="text-[15px] text-text-primary text-left">在线封面匹配</span>
          <div
            class="w-10 h-6 rounded-full transition-colors-smooth relative shrink-0"
            :class="uiStore.fetchCoversOnline ? 'bg-brand-orange' : 'bg-text-disabled'"
          >
            <div
              class="w-4 h-4 bg-white rounded-full absolute top-1 transition-transform"
              :class="uiStore.fetchCoversOnline ? 'translate-x-5' : 'translate-x-1'"
            ></div>
          </div>
        </button>
        <p class="px-4 pb-2 text-text-muted" style="font-size: var(--text-10);">
          缺少封面时，向 iTunes 发送专辑/艺人名称
        </p>
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

      <!-- 添加来源（本地 / WebDAV） -->
      <div class="mt-3 grid grid-cols-2 gap-2">
        <button
          class="h-11 rounded-[10px] border border-dashed border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5"
          @click="emit('add-local')"
        >
          <Plus class="w-[15px] h-[15px]" aria-hidden="true" />
          添加本地目录
        </button>
        <button
          class="h-11 rounded-[10px] border border-dashed border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5"
          @click="emit('add-webdav')"
        >
          <Server class="w-[15px] h-[15px]" aria-hidden="true" />
          添加 WebDAV
        </button>
      </div>
    </section>

    <!-- ===== 云端备份与恢复（MA3 A3-5） ===== -->
    <section class="px-4 py-1">
      <div class="flex items-center justify-between px-1 py-2">
        <h2 class="text-text-muted font-semibold uppercase tracking-widest" style="font-size: var(--text-10);">
          云端备份与恢复 (WebDAV)
        </h2>
      </div>

      <div class="rounded-[10px] bg-bg-canvas border border-border-color p-4 space-y-3">
        <div class="flex items-center justify-between text-[13px]">
          <span class="text-text-muted">备份状态</span>
          <span v-if="syncConfig?.enabled" class="text-emerald-500 font-medium">已配置</span>
          <span v-else class="text-text-disabled">未配置</span>
        </div>

        <div v-if="syncConfig?.last_sync_at" class="flex items-center justify-between text-[12px]">
          <span class="text-text-muted">上次备份</span>
          <span class="text-text-secondary font-mono">{{ syncConfig.last_sync_at }}</span>
        </div>

        <!-- 提示信息 -->
        <div v-if="syncFeedback" class="p-2.5 rounded-[6px] text-[12px] flex items-center gap-2" :class="syncFeedback.type === 'ok' ? 'bg-emerald-500/10 text-emerald-600' : 'bg-red-500/10 text-red-600'">
          <AlertCircle class="w-3.5 h-3.5 flex-shrink-0" />
          <span>{{ syncFeedback.text }}</span>
        </div>

        <div class="grid grid-cols-2 gap-2 pt-1">
          <button
            class="h-10 rounded-[8px] border border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5 disabled:opacity-50"
            :disabled="syncing"
            @click="onUploadBackup"
          >
            <RefreshCw v-if="syncing" class="w-3.5 h-3.5 animate-spin" />
            <CloudUpload v-else class="w-3.5 h-3.5" />
            <span>{{ syncing ? '备份中…' : '备份到云端' }}</span>
          </button>

          <button
            class="h-10 rounded-[8px] border border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5 disabled:opacity-50"
            :disabled="syncing || restoring"
            @click="askRestore"
          >
            <CloudDownload class="w-3.5 h-3.5" />
            <span>从云端恢复</span>
          </button>
        </div>
      </div>
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
            <p class="text-[15px] font-semibold text-text-primary">{{ APP_NAME_CN }}</p>
            <p class="text-text-muted font-mono" style="font-size: var(--text-11);">{{ APP_VERSION_LABEL }}</p>
          </div>
        </div>
        <p class="text-text-muted mt-2" style="font-size: var(--text-12);">
          {{ APP_TAGLINE }}
        </p>

        <!-- 检查更新提示 -->
        <p v-if="updateMsg" class="text-text-secondary text-[12px] mt-2 pt-1 border-t border-border-color">
          {{ updateMsg }}
        </p>

        <!-- 操作按钮 -->
        <div class="grid grid-cols-2 gap-2 mt-3 pt-2 border-t border-border-color">
          <button
            class="h-9 rounded-[8px] border border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5 disabled:opacity-50"
            :disabled="checkingUpdate"
            @click="onCheckUpdate"
          >
            <RefreshCw v-if="checkingUpdate" class="w-3.5 h-3.5 animate-spin" />
            <span>{{ checkingUpdate ? '检查中…' : '检查更新' }}</span>
          </button>
          <button
            class="h-9 rounded-[8px] border border-border-solid text-[13px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-1.5"
            @click="onExportDiagnostics"
          >
            <span>{{ logExported ? '已复制诊断' : '导出诊断' }}</span>
          </button>
        </div>
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

      <!-- ===== 云端恢复二次防误触确认（MA3 A3-5） ===== -->
      <div
        v-if="showRestoreDialog"
        class="fixed inset-0 z-[100] bg-black/50 flex items-end justify-center"
        @click.self="showRestoreDialog = false"
      >
        <div class="w-full max-w-[420px] bg-bg-canvas rounded-t-[16px] px-5 pt-5 pb-8 animate-rise space-y-4">
          <div class="w-10 h-1 rounded-full bg-border-solid mx-auto"></div>
          <h3 class="text-[16px] font-semibold text-text-primary text-center">从云端恢复数据</h3>
          <p class="text-[13px] text-text-muted text-center leading-relaxed">
            恢复操作将从 WebDAV 下载最新备份快照，并<strong class="text-red-500">完全替换</strong>本机的歌单、收藏、历史和播放源记录。<br>
            恢复后桌面添加的本地路径来源在手机端需要重新授权绑定。
          </p>
          <div class="space-y-1.5">
            <label class="block text-[12px] text-text-muted">请输入「恢复」二字以确认操作：</label>
            <input
              v-model="restoreInput"
              type="text"
              placeholder="恢复"
              class="w-full h-11 px-3 rounded-[8px] bg-bg-content border border-border-solid text-[14px] text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-red-500 text-center font-medium"
            />
          </div>
          <div class="flex gap-3 pt-2">
            <button
              class="flex-1 h-11 rounded-[10px] bg-bg-hover text-text-secondary text-[15px] font-medium active:opacity-85"
              @click="showRestoreDialog = false"
            >
              取消
            </button>
            <button
              class="flex-1 h-11 rounded-[10px] bg-red-500 text-white text-[15px] font-semibold active:opacity-85 disabled:opacity-40"
              :disabled="restoreInput.trim() !== '恢复' || restoring"
              @click="confirmRestore"
            >
              {{ restoring ? '恢复中…' : '确认覆盖恢复' }}
            </button>
          </div>
        </div>
      </div>

      <!-- ===== 恢复成功重启提示（MA3 A3-5） ===== -->
      <div
        v-if="showRestartConfirm"
        class="fixed inset-0 z-[100] bg-black/50 flex items-center justify-center p-4"
      >
        <div class="w-full max-w-[340px] bg-bg-canvas rounded-[16px] p-5 text-center space-y-3 border border-border-color shadow-xl">
          <h3 class="text-[16px] font-semibold text-text-primary">数据恢复成功</h3>
          <p class="text-[13px] text-text-muted leading-relaxed">
            数据库已成功替换为云端快照。应用需要重新启动以加载新的音乐库与配置。
          </p>
          <button
            class="w-full h-11 rounded-[10px] bg-brand-orange text-white text-[15px] font-semibold active:opacity-85"
            @click="onRestartNow"
          >
            立即重启应用
          </button>
        </div>
      </div>

      <!-- ===== 发现新版本弹窗（MA5 A5-4） ===== -->
      <div
        v-if="updateInfo"
        class="fixed inset-0 z-[100] bg-black/50 flex items-center justify-center p-4"
        @click.self="updateInfo = null"
      >
        <div class="w-full max-w-[340px] bg-bg-canvas rounded-[16px] p-5 space-y-3 border border-border-color shadow-xl">
          <div class="flex items-center justify-between">
            <h3 class="text-[16px] font-semibold text-text-primary">发现新版本</h3>
            <span class="text-[12px] font-mono text-brand-orange bg-brand-orange/10 px-2 py-0.5 rounded-full font-medium">
              {{ updateInfo.version }}
            </span>
          </div>
          <div class="max-h-[160px] overflow-y-auto rounded-[8px] bg-bg-content p-3 text-[12px] text-text-muted whitespace-pre-wrap leading-relaxed">
            {{ updateInfo.notes }}
          </div>
          <div class="flex gap-2 pt-1">
            <button
              class="flex-1 h-10 rounded-[8px] bg-bg-hover text-text-secondary text-[14px] font-medium active:opacity-85"
              @click="updateInfo = null"
            >
              稍后再说
            </button>
            <button
              class="flex-1 h-10 rounded-[8px] bg-brand-orange text-white text-[14px] font-semibold active:opacity-85"
              @click="openDownloadUrl(updateInfo.url)"
            >
              前往下载
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
