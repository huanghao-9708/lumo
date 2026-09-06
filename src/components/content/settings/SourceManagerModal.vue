<script setup lang="ts">
import { ref, computed } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import {
  HardDrive, Server, ChevronLeft, Scan, Trash2, Loader2,
  Plus, CheckCircle2, XCircle, FolderOpen, Database,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../../stores/player';
import type { MusicSource } from '../../../stores/player';
import { scannerTestWebdav } from '../../../api/scanner';
import type { WebdavProbeResult } from '../../../api/scanner';
import AppModal from '../../../components/shared/AppModal.vue';
import ConfirmDialog from '../../../components/shared/ConfirmDialog.vue';

/**
 * 数据源管理弹窗：列表态（扫描 / 删除）+ 两步添加流程（选类型 → 填表单）。
 * 全部操作复用 playerStore 现有 actions；删除走 ConfirmDialog 二次确认。
 */
const playerStore = usePlayerStore();
const emit = defineEmits<{ close: [] }>();

const sources = computed(() => playerStore.sources);

type View = 'list' | 'type' | 'form';
const view = ref<View>('list');
const form = ref({ kind: 'local' as 'local' | 'webdav', name: '', path: '', url: '', username: '', password: '' });
const isAdding = ref(false);
const addError = ref('');
const isTesting = ref(false);
const testResult = ref<WebdavProbeResult | null>(null);

const pendingDelete = ref<MusicSource | null>(null);
const isDeleting = ref(false);

const headerTitle = computed(() => {
  if (view.value === 'list') return '数据源管理';
  if (view.value === 'type') return '添加数据源';
  return form.value.kind === 'local' ? '本地文件夹' : 'WebDAV';
});
const headerDesc = computed(() => {
  if (view.value === 'list') return '管理本地文件夹与 WebDAV 连接';
  if (view.value === 'type') return '选择要添加的数据源类型';
  return form.value.kind === 'local' ? '选择电脑上的音乐文件夹' : '填写 NAS 或远程音乐库的连接信息';
});

function isScanning(s: MusicSource) {
  return s.lastScanned === 'Scanning...' || s.lastScanned.startsWith('扫描中');
}

function lastScannedText(s: MusicSource): string {
  if (s.lastScanned === 'Never') return '未扫描';
  if (s.lastScanned === 'Error') return '上次扫描失败';
  return s.lastScanned;
}

function startAdd() {
  form.value = { kind: 'local', name: '', path: '', url: '', username: '', password: '' };
  addError.value = '';
  testResult.value = null;
  view.value = 'type';
}

function chooseKind(kind: 'local' | 'webdav') {
  form.value.kind = kind;
  view.value = 'form';
}

function goBack() {
  if (view.value === 'form') {
    view.value = 'type';
  } else {
    view.value = 'list';
  }
  addError.value = '';
  testResult.value = null;
}

async function selectFolder() {
  const selected = await open({ directory: true, multiple: false, title: '选择音乐文件夹' });
  if (selected) form.value.path = selected;
}

async function testConnection() {
  if (!form.value.url.trim()) return;
  isTesting.value = true;
  testResult.value = null;
  try {
    testResult.value = await scannerTestWebdav(
      form.value.url.trim(),
      form.value.username || undefined,
      form.value.password || undefined,
    );
  } catch (e: any) {
    testResult.value = { ok: false, latencyMs: 0, error: typeof e === 'string' ? e : e?.message || '测试请求失败' };
  }
  isTesting.value = false;
}

async function submitAdd() {
  const f = form.value;
  if (!f.name.trim()) { addError.value = '请输入数据源名称'; return; }
  if (f.kind === 'local' && !f.path.trim()) { addError.value = '请选择音乐文件夹'; return; }
  if (f.kind === 'webdav' && !f.url.trim()) { addError.value = '请输入 WebDAV 地址'; return; }
  addError.value = '';
  isAdding.value = true;
  try {
    let newId: number;
    if (f.kind === 'local') {
      newId = await playerStore.addSource('local', f.name.trim(), f.path);
    } else {
      newId = await playerStore.addSource('webdav', f.name.trim(), f.url.trim(), f.username || undefined, f.password || undefined);
    }
    // 添加成功回列表并自动开始首轮扫描
    playerStore.scanSource(newId);
    form.value = { kind: 'local', name: '', path: '', url: '', username: '', password: '' };
    view.value = 'list';
  } catch (e: any) {
    addError.value = typeof e === 'string' ? e : e?.message || e?.toString() || '添加数据源失败';
  }
  isAdding.value = false;
}

async function confirmDelete() {
  if (!pendingDelete.value) return;
  isDeleting.value = true;
  try {
    await playerStore.removeSource(pendingDelete.value.id);
    pendingDelete.value = null;
  } catch { /* store 内已记录日志 */ }
  isDeleting.value = false;
}
</script>

<template>
  <AppModal width="640px" @close="emit('close')">
    <!-- Header -->
    <template #header>
      <div class="flex items-center gap-2.5 px-6 pt-5 pb-4 border-b border-border-color">
        <button
          v-if="view !== 'list'"
          class="w-7 h-7 rounded-[6px] flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-list-hover transition-colors-smooth shrink-0"
          title="返回"
          @click="goBack"
        >
          <ChevronLeft class="w-4 h-4" />
        </button>
        <div class="min-w-0">
          <h2 class="text-[16px] font-bold text-text-primary leading-tight">{{ headerTitle }}</h2>
          <p class="text-[11px] text-text-muted mt-0.5">{{ headerDesc }}</p>
        </div>
      </div>
    </template>

    <!-- ============ 列表态 ============ -->
    <div v-if="view === 'list'" class="px-4 py-3 min-h-[220px]">
      <div v-if="sources.length === 0" class="flex flex-col items-center justify-center py-14 gap-3">
        <div class="w-12 h-12 rounded-[12px] bg-bg-content border border-border-color flex items-center justify-center">
          <Database class="w-5 h-5 text-text-disabled" />
        </div>
        <p class="text-[13px] text-text-secondary">尚未添加数据源</p>
        <p class="text-[11px] text-text-muted">添加本地文件夹或 WebDAV 连接，开始构建你的音乐库</p>
      </div>

      <div v-else class="space-y-1">
        <div
          v-for="s in sources"
          :key="s.id"
          class="group flex items-center gap-3 px-3 py-2.5 rounded-[8px] hover:bg-list-hover transition-colors-smooth"
        >
          <div class="w-8 h-8 rounded-[8px] bg-bg-content border border-border-color flex items-center justify-center shrink-0">
            <Server v-if="s.kind === 'webdav'" class="w-4 h-4 text-text-muted" />
            <HardDrive v-else class="w-4 h-4 text-text-muted" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-[13px] text-text-primary truncate leading-tight">{{ s.name }}</p>
            <p class="text-[11px] text-text-muted truncate mt-0.5">{{ s.path }}</p>
          </div>
          <span v-if="!isScanning(s)" class="text-[10px] text-text-disabled shrink-0">{{ lastScannedText(s) }}</span>
          <Loader2 v-if="isScanning(s)" class="w-4 h-4 text-brand-orange animate-spin shrink-0" />
          <template v-else>
            <button
              class="w-7 h-7 rounded-[6px] flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-list-selected transition-colors-smooth shrink-0"
              title="扫描"
              @click="playerStore.scanSource(s.id)"
            >
              <Scan class="w-3.5 h-3.5" />
            </button>
            <button
              class="w-7 h-7 rounded-[6px] flex items-center justify-center text-text-muted hover:text-status-error hover:bg-list-selected transition-colors-smooth shrink-0"
              title="删除"
              @click="pendingDelete = s"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          </template>
        </div>
      </div>
    </div>

    <!-- ============ 添加态 · 第一步：选类型 ============ -->
    <div v-else-if="view === 'type'" class="px-6 py-5 grid grid-cols-2 gap-3">
      <button
        class="flex flex-col items-start gap-3 p-4 rounded-[10px] border border-border-color bg-bg-content hover:border-brand-orange/50 hover:bg-list-hover transition-colors-smooth text-left"
        @click="chooseKind('local')"
      >
        <div class="w-9 h-9 rounded-[8px] bg-bg-canvas border border-border-color flex items-center justify-center">
          <HardDrive class="w-5 h-5 text-text-secondary" />
        </div>
        <div>
          <p class="text-[13px] font-medium text-text-primary">本地文件夹</p>
          <p class="text-[11px] text-text-muted mt-0.5">扫描电脑上的音乐文件夹</p>
        </div>
      </button>
      <button
        class="flex flex-col items-start gap-3 p-4 rounded-[10px] border border-border-color bg-bg-content hover:border-brand-orange/50 hover:bg-list-hover transition-colors-smooth text-left"
        @click="chooseKind('webdav')"
      >
        <div class="w-9 h-9 rounded-[8px] bg-bg-canvas border border-border-color flex items-center justify-center">
          <Server class="w-5 h-5 text-text-secondary" />
        </div>
        <div>
          <p class="text-[13px] font-medium text-text-primary">WebDAV</p>
          <p class="text-[11px] text-text-muted mt-0.5">连接 NAS 或远程音乐库</p>
        </div>
      </button>
    </div>

    <!-- ============ 添加态 · 第二步：填表单 ============ -->
    <div v-else class="px-6 py-5 space-y-3.5">
      <div>
        <label class="block text-[11px] text-text-muted mb-1.5 font-medium">名称</label>
        <input
          v-model="form.name"
          type="text"
          placeholder="给这个数据源起个名字"
          class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
          @input="addError = ''"
        />
      </div>

      <template v-if="form.kind === 'local'">
        <div>
          <label class="block text-[11px] text-text-muted mb-1.5 font-medium">文件夹路径</label>
          <div class="flex gap-2">
            <input
              v-model="form.path"
              type="text"
              placeholder="如 D:\Music"
              class="flex-1 min-w-0 h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
              @input="addError = ''"
            />
            <button
              class="h-[34px] px-3 rounded-[6px] text-[12px] bg-list-hover text-text-primary hover:bg-list-selected transition-colors-smooth shrink-0 flex items-center gap-1.5"
              @click="selectFolder"
            >
              <FolderOpen class="w-3.5 h-3.5" />
              浏览…
            </button>
          </div>
        </div>
      </template>

      <template v-else>
        <div>
          <label class="block text-[11px] text-text-muted mb-1.5 font-medium">服务器地址</label>
          <input
            v-model="form.url"
            type="text"
            placeholder="如 https://nas.local:5006/music"
            class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
            @input="addError = ''"
          />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-[11px] text-text-muted mb-1.5 font-medium">用户名（可选）</label>
            <input
              v-model="form.username"
              type="text"
              autocomplete="off"
              class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
            />
          </div>
          <div>
            <label class="block text-[11px] text-text-muted mb-1.5 font-medium">密码（可选）</label>
            <input
              v-model="form.password"
              type="password"
              autocomplete="new-password"
              class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
            />
          </div>
        </div>

        <!-- 测试连接 -->
        <div class="flex items-center gap-3 flex-wrap">
          <button
            class="h-[30px] px-3.5 rounded-[6px] text-[12px] border border-border-color text-text-secondary hover:bg-list-hover hover:text-text-primary transition-colors-smooth disabled:opacity-40 flex items-center gap-1.5"
            :disabled="isTesting || !form.url.trim()"
            @click="testConnection"
          >
            <Loader2 v-if="isTesting" class="w-3.5 h-3.5 animate-spin" />
            {{ isTesting ? '测试中…' : '测试连接' }}
          </button>
          <span v-if="testResult?.ok" class="text-[11px] text-status-success flex items-center gap-1">
            <CheckCircle2 class="w-3.5 h-3.5" />
            连接成功（{{ testResult.latencyMs }} ms）
          </span>
          <span v-else-if="testResult && !testResult.ok" class="text-[11px] text-status-error flex items-center gap-1 min-w-0">
            <XCircle class="w-3.5 h-3.5 shrink-0" />
            <span class="truncate">{{ testResult.error || '连接失败' }}{{ testResult.statusCode ? `（HTTP ${testResult.statusCode}）` : '' }}</span>
          </span>
        </div>
      </template>

      <p v-if="addError" class="text-[11px] text-status-error">{{ addError }}</p>
    </div>

    <!-- Footer -->
    <template #footer>
      <div v-if="view === 'list'" class="px-6 py-4 border-t border-border-color flex items-center justify-end">
        <button
          class="h-[32px] px-4 rounded-full bg-text-primary text-bg-canvas text-[12px] font-medium flex items-center gap-1.5 hover:opacity-90 transition-opacity"
          @click="startAdd"
        >
          <Plus class="w-3.5 h-3.5" />
          添加数据源
        </button>
      </div>
      <div v-else-if="view === 'form'" class="px-6 py-4 border-t border-border-color flex items-center justify-end gap-2">
        <button
          class="h-[32px] px-4 text-[13px] text-text-secondary hover:text-text-primary transition-colors-smooth"
          @click="goBack"
        >取消</button>
        <button
          class="h-[32px] px-5 rounded-full bg-text-primary text-bg-canvas text-[12px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity disabled:opacity-40"
          :disabled="isAdding"
          @click="submitAdd"
        >
          <Loader2 v-if="isAdding" class="w-3.5 h-3.5 animate-spin" />
          {{ isAdding ? '添加中…' : '添加' }}
        </button>
      </div>
    </template>
  </AppModal>

  <!-- 删除二次确认 -->
  <ConfirmDialog
    v-if="pendingDelete"
    title="删除数据源"
    :message="`确定删除数据源「${pendingDelete.name}」吗？其索引的歌曲与专辑信息将一并移除，磁盘上的源文件不会被删除。`"
    confirm-text="删除"
    danger
    :busy="isDeleting"
    @confirm="confirmDelete"
    @cancel="pendingDelete = null"
  />
</template>
