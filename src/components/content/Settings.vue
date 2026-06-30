<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { FolderOpen, Server, Trash2, Sun, Moon, Info, Scan } from 'lucide-vue-next';
import { open } from '@tauri-apps/plugin-dialog';
import { usePlayerStore, type MusicSource } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { libraryGetCacheSize, libraryClearCache } from '../../api/library';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

const sources = computed(() => playerStore.sources);
const cacheSize = ref('—');
const isClearingCache = ref(false);
const isAddingSource = ref(false);
const newSource = ref({ kind: 'local' as 'local' | 'webdav', name: '', path: '', url: '', username: '', password: '' });

onMounted(async () => {
  try {
    const size = await libraryGetCacheSize();
    cacheSize.value = size > 0 ? `${(size / 1024 / 1024).toFixed(1)} MB` : '0 MB';
  } catch { cacheSize.value = '—'; }
});

async function clearCache() {
  isClearingCache.value = true;
  try {
    await libraryClearCache();
    cacheSize.value = '0 MB';
  } catch { /* ignore */ }
  isClearingCache.value = false;
}

function removeSource(sourceId: number) {
  playerStore.removeSource(sourceId);
}

async function selectFolder() {
  const selected = await open({ directory: true, multiple: false, title: '选择音乐文件夹' });
  if (selected) {
    newSource.value.path = selected;
  }
}

function scanSource(sourceId: number) {
  playerStore.scanSource(sourceId);
}

async function addSource() {
  if (!newSource.value.name.trim()) return;
  isAddingSource.value = true;
  try {
    if (newSource.value.kind === 'local') {
      await playerStore.addSource('local', newSource.value.name, newSource.value.path);
    } else {
      await playerStore.addSource('webdav', newSource.value.name, newSource.value.url, newSource.value.username, newSource.value.password);
    }
    newSource.value = { kind: 'local', name: '', path: '', url: '', username: '', password: '' };
  } catch { /* ignore */ }
  isAddingSource.value = false;
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-y-auto select-none min-w-0 px-8 pt-8 pb-8">
    <h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none mb-8">设置</h1>

    <!-- 数据源管理 -->
    <section class="mb-10">
      <h2 class="text-[18px] font-bold text-text-primary mb-1">数据源</h2>
      <p class="text-[12px] text-text-muted mb-4">管理你的音乐库文件夹和 WebDAV 连接</p>

      <div class="space-y-2 mb-4">
        <div v-for="s in sources" :key="s.id" class="flex items-center justify-between px-4 py-3 bg-bg-canvas border border-border-color rounded-[8px]">
          <div class="flex items-center gap-3 min-w-0">
            <Server v-if="s.kind === 'webdav'" class="w-4 h-4 text-text-muted shrink-0" />
            <FolderOpen v-else class="w-4 h-4 text-text-muted shrink-0" />
            <div class="min-w-0">
              <p class="text-[13px] text-text-primary truncate">{{ s.name }}</p>
              <p class="text-[11px] text-text-muted truncate">{{ s.kind === 'webdav' ? s.path : s.path }}</p>
              <p v-if="s.lastScanned" class="text-[10px] text-text-disabled mt-0.5">{{ s.lastScanned }}</p>
            </div>
          </div>
          <div class="flex items-center gap-2 shrink-0 ml-2">
            <button class="flex items-center gap-1.5 text-[11px] px-2.5 py-1.5 rounded-[6px] bg-list-hover text-text-secondary hover:bg-list-selected hover:text-text-primary transition-colors-smooth" @click="scanSource(s.id)">
              <Scan class="w-3.5 h-3.5" />
              扫描
            </button>
            <button class="text-text-muted hover:text-red-500 transition-colors-smooth" @click="removeSource(s.id)">
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
        <div v-if="sources.length === 0" class="text-[12px] text-text-muted/70 px-1">暂无数据源</div>
      </div>

      <!-- 添加数据源 -->
      <div class="border border-border-color rounded-[8px] p-4 bg-bg-canvas">
        <div class="flex items-center gap-2 mb-3">
          <button class="text-[12px] px-3 py-1 rounded-[6px]" :class="newSource.kind === 'local' ? 'bg-list-selected font-medium' : 'text-text-muted'" @click="newSource.kind = 'local'">本地文件夹</button>
          <button class="text-[12px] px-3 py-1 rounded-[6px]" :class="newSource.kind === 'webdav' ? 'bg-list-selected font-medium' : 'text-text-muted'" @click="newSource.kind = 'webdav'">WebDAV</button>
        </div>
        <div class="space-y-2">
          <input v-model="newSource.name" placeholder="名称" class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50" />
          <div v-if="newSource.kind === 'local'" class="flex gap-2">
            <input v-model="newSource.path" placeholder="路径（如 D:\Music）" class="flex-1 h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50" />
            <button class="h-[34px] px-3 rounded-[6px] text-[12px] bg-list-hover text-text-primary hover:bg-list-selected transition-colors-smooth shrink-0" @click="selectFolder">浏览…</button>
          </div>
          <template v-else>
            <input v-model="newSource.url" placeholder="URL" class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50" />
            <input v-model="newSource.username" placeholder="用户名" class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50" />
            <input v-model="newSource.password" type="password" placeholder="密码" class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50" />
          </template>
          <button
            class="h-[34px] px-4 rounded-full bg-text-primary text-bg-canvas text-[12px] font-medium hover:opacity-90 transition-opacity"
            :disabled="isAddingSource"
            @click="addSource"
          >{{ isAddingSource ? '添加中…' : '添加数据源' }}</button>
        </div>
      </div>
    </section>

    <!-- 缓存管理 -->
    <section class="mb-10">
      <h2 class="text-[18px] font-bold text-text-primary mb-1">缓存</h2>
      <p class="text-[12px] text-text-muted mb-4">管理艺术封面和其他临时数据</p>
      <div class="flex items-center justify-between px-4 py-3 bg-bg-canvas border border-border-color rounded-[8px] max-w-md">
        <span class="text-[13px] text-text-primary">缓存大小：{{ cacheSize }}</span>
        <button
          class="text-[12px] px-3 py-1.5 rounded-[6px] bg-list-hover text-text-primary hover:bg-list-selected transition-colors-smooth"
          :disabled="isClearingCache"
          @click="clearCache"
        >{{ isClearingCache ? '清理中…' : '清理缓存' }}</button>
      </div>
    </section>

    <!-- 主题 -->
    <section class="mb-10">
      <h2 class="text-[18px] font-bold text-text-primary mb-1">主题</h2>
      <p class="text-[12px] text-text-muted mb-4">切换亮色 / 暗色模式</p>
      <button
        class="flex items-center gap-2 px-4 py-2.5 bg-bg-canvas border border-border-color rounded-[8px] text-[13px] text-text-primary hover:bg-list-hover transition-colors-smooth"
        @click="uiStore.toggleDarkMode()"
      >
        <Sun v-if="uiStore.isDarkMode" class="w-4 h-4" />
        <Moon v-else class="w-4 h-4" />
        {{ uiStore.isDarkMode ? '切换到亮色模式' : '切换到暗色模式' }}
      </button>
    </section>

    <!-- 关于 -->
    <section>
      <h2 class="text-[18px] font-bold text-text-primary mb-1">关于</h2>
      <p class="text-[12px] text-text-muted mb-4">版本和项目信息</p>
      <div class="px-4 py-3 bg-bg-canvas border border-border-color rounded-[8px] max-w-md">
        <div class="flex items-center gap-3">
          <Info class="w-5 h-5 text-text-muted shrink-0" />
          <div>
            <p class="text-[13px] text-text-primary font-medium">Lumo Player</p>
            <p class="text-[11px] text-text-muted font-mono">v1.0.0</p>
            <p class="text-[11px] text-text-muted">一个轻量、温暖的本地音乐播放器</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
