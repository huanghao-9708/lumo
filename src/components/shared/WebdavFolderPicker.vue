<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { FolderOpen, ChevronRight, ArrowLeft, Plus, Check, X, Loader } from 'lucide-vue-next';
import { useSyncStore } from '../../stores/sync';
import { syncCreateFolder } from '../../api/sync';

const emit = defineEmits<{
  select: [path: string];
  close: [];
}>();

const syncStore = useSyncStore();

/** 当前浏览的路径（相对 URL 路径） */
const currentPath = ref('/');
const pathHistory = ref<string[]>(['/']);

/** 新建文件夹状态 */
const showNewFolderInput = ref(false);
const newFolderName = ref('');

/** 进入子目录 */
function enterDir(name: string) {
  const dir = name.replace(/\/$/, '');
  const newPath = currentPath.value === '/'
    ? `/${dir}`
    : `${currentPath.value.replace(/\/$/, '')}/${dir}`;
  pathHistory.value.push(newPath);
  currentPath.value = newPath;
  loadEntries();
}

/** 返回上级目录 */
function goUp() {
  if (pathHistory.value.length > 1) {
    pathHistory.value.pop();
    currentPath.value = pathHistory.value[pathHistory.value.length - 1];
    loadEntries();
  }
}

/** 加载当前路径的目录列表 */
async function loadEntries() {
  await syncStore.browseWebdav(currentPath.value);
}

/** 确认选择当前路径 */
function confirmSelect() {
  emit('select', currentPath.value);
}

/** 取消 */
function cancel() {
  emit('close');
}

/** 创建文件夹 */
async function handleCreateFolder() {
  if (!newFolderName.value.trim()) return;
  const targetPath = currentPath.value === '/'
    ? `/${newFolderName.value.trim()}`
    : `${currentPath.value.replace(/\/$/, '')}/${newFolderName.value.trim()}`;
  try {
    if (!syncStore.config.webdav_url) return;
    await syncCreateFolder(
      syncStore.config.webdav_url,
      syncStore.config.username || null,
      syncStore.config.password || null,
      targetPath,
    );
    newFolderName.value = '';
    showNewFolderInput.value = false;
    loadEntries();
  } catch (e: any) {
    syncStore.browseError = `新建文件夹失败: ${e}`;
  }
}

// 路径面包屑（分段）
const pathSegments = ref<{ name: string; path: string }[]>([]);
watch(currentPath, (p) => {
  const parts = p.split('/').filter(Boolean);
  const segments: { name: string; path: string }[] = [];
  let accumulated = '';
  for (const part of parts) {
    accumulated = accumulated ? `${accumulated}/${part}` : `/${part}`;
    segments.push({ name: part, path: accumulated });
  }
  pathSegments.value = segments;
}, { immediate: true });

onMounted(() => {
  loadEntries();
});

function onOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget) cancel();
}
</script>

<template>
  <div class="fixed inset-0 z-[100] bg-black/30 flex items-center justify-center" @click="onOverlayClick">
    <div class="bg-bg-canvas rounded-[12px] w-[480px] max-h-[70vh] shadow-lg flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="px-6 pt-6 pb-3 flex items-center justify-between">
        <h2 class="text-[16px] font-bold text-text-primary">选择同步文件夹</h2>
        <button class="text-text-muted hover:text-text-primary transition-colors-smooth" @click="cancel">
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- 面包屑导航 -->
      <div class="px-6 pb-3 flex items-center gap-1 text-[12px] min-h-[28px]">
        <button
          class="text-text-muted hover:text-text-primary transition-colors-smooth"
          @click="currentPath = '/'; pathHistory = ['/']; loadEntries()"
        >根目录</button>
        <template v-for="(seg, i) in pathSegments" :key="i">
          <ChevronRight class="w-3 h-3 text-text-disabled" />
          <button
            :class="i === pathSegments.length - 1 ? 'font-medium text-text-primary' : 'text-text-muted hover:text-text-primary'"
            @click="currentPath = seg.path; pathHistory.push(seg.path); loadEntries()"
          >{{ seg.name }}</button>
        </template>
      </div>

      <!-- 目录列表 -->
      <div class="flex-1 overflow-y-auto px-6 pb-3 min-h-0">
        <!-- 返回上级 -->
        <button
          v-if="currentPath !== '/'"
          class="flex items-center gap-2 w-full px-3 py-2 rounded-[8px] text-text-muted hover:text-text-primary hover:bg-list-hover transition-colors-smooth text-[13px]"
          @click="goUp"
        >
          <ArrowLeft class="w-4 h-4" />
          ..
        </button>

        <!-- 加载中 -->
        <div v-if="syncStore.isBrowsing" class="flex items-center justify-center py-8 text-text-muted">
          <Loader class="w-5 h-5 animate-spin" />
        </div>

        <!-- 空状态 -->
        <div v-else-if="!syncStore.isBrowsing && syncStore.browseEntries.length === 0 && !syncStore.browseError" class="text-center py-8 text-[12px] text-text-muted/70">
          该文件夹为空
        </div>

        <!-- 从 path 中提取最后一段作为文件夹名 -->
        <button
          v-for="entry in syncStore.browseEntries"
          :key="entry.path"
          class="flex items-center gap-2 w-full px-3 py-2 rounded-[8px] text-text-primary hover:bg-list-hover transition-colors-smooth text-[13px]"
          @click="enterDir(entry.path)"
        >
          <FolderOpen class="w-4 h-4 text-brand-orange shrink-0" />
          <span class="truncate">{{ entry.path.replace(/\/$/, '').split('/').pop() || entry.path }}</span>
        </button>

        <!-- 错误 -->
        <p v-if="syncStore.browseError" class="text-[11px] text-red-500 mt-2">{{ syncStore.browseError }}</p>

        <!-- 新建文件夹输入 -->
        <div v-if="showNewFolderInput" class="mt-2 flex items-center gap-2">
          <input
            v-model="newFolderName"
            type="text"
            placeholder="文件夹名称"
            class="flex-1 h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
            @keyup.enter="handleCreateFolder"
          />
          <button class="h-[34px] px-3 rounded-[6px] bg-brand-orange text-white text-[12px] font-medium hover:opacity-90 transition-opacity" @click="handleCreateFolder">创建</button>
          <button class="h-[34px] px-3 rounded-[6px] text-text-muted hover:text-text-primary transition-colors-smooth text-[12px]" @click="showNewFolderInput = false; newFolderName = ''">取消</button>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-border-color flex items-center justify-between">
        <button
          class="flex items-center gap-1.5 text-[12px] text-text-muted hover:text-text-primary transition-colors-smooth"
          @click="showNewFolderInput = !showNewFolderInput"
        >
          <Plus class="w-4 h-4" />
          新建文件夹
        </button>
        <div class="flex items-center gap-2">
          <button class="h-[34px] px-4 text-[13px] text-text-secondary hover:text-text-primary transition-colors-smooth" @click="cancel">取消</button>
          <button class="h-[34px] px-4 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity" @click="confirmSelect">
            <Check class="w-4 h-4" />
            选择此文件夹
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
