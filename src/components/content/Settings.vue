<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import type { Component } from 'vue';
import { Palette, Database, RefreshCw, HardDrive, Info, ShieldCheck, Server, FolderOpen, Upload, Download, AlertTriangle } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useSyncStore } from '../../stores/sync';
import { libraryGetCacheSize, libraryClearCache } from '../../api/library';
import { playbackGetAudioCacheSize, playbackClearAudioCache } from '../../api/playback';
import ToggleSwitch from '../shared/ToggleSwitch.vue';
import SourceManagerModal from './settings/SourceManagerModal.vue';
import WebdavFolderPicker from '../shared/WebdavFolderPicker.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();
const syncStore = useSyncStore();

// ===== 左栏分类导航 =====
type SectionId = 'appearance' | 'privacy' | 'sources' | 'sync' | 'storage' | 'about';
const activeSection = ref<SectionId>('appearance');
const navItems: { id: SectionId; label: string; icon: Component }[] = [
  { id: 'appearance', label: '外观', icon: Palette },
  { id: 'privacy', label: '隐私', icon: ShieldCheck },
  { id: 'sources', label: '数据源', icon: Database },
  { id: 'sync', label: '数据同步', icon: RefreshCw },
  { id: 'storage', label: '存储', icon: HardDrive },
  { id: 'about', label: '关于', icon: Info },
];

const sources = computed(() => playerStore.sources);
const showSourceManager = ref(false);

// ===== 缓存 =====
const cacheSize = ref('—');
const isClearingCache = ref(false);

// ===== 数据同步 =====
const showFolderPicker = ref(false);
const showRemotePrompt = ref(false);

onMounted(async () => {
  try {
    const [artworkSize, audioSize] = await Promise.all([
      libraryGetCacheSize(),
      playbackGetAudioCacheSize(),
    ]);
    const size = artworkSize + audioSize;
    cacheSize.value = size > 0 ? `${(size / 1024 / 1024).toFixed(1)} MB` : '0 MB';
  } catch { cacheSize.value = '—'; }

  // 加载同步配置
  await syncStore.fetchConfig();

  // 首次加载：如果同步已启用且有完整配置，自动检查云端
  if (syncStore.config.enabled && syncStore.config.webdav_url && syncStore.config.remote_path) {
    const result = await syncStore.checkRemote();
    if (result?.has_data) {
      showRemotePrompt.value = true;
    }
  }
});

async function clearCache() {
  isClearingCache.value = true;
  try {
    await Promise.all([libraryClearCache(), playbackClearAudioCache()]);
    cacheSize.value = '0 MB';
  } catch { /* ignore */ }
  isClearingCache.value = false;
}
</script>

<template>
  <div class="flex-1 flex min-w-0 overflow-hidden select-none">
    <!-- ============ 左栏：分类导航 ============ -->
    <nav class="w-[200px] shrink-0 border-r border-border-color bg-bg-canvas flex flex-col px-3 pt-6 pb-4">
      <h1 class="px-3 mb-5 text-[20px] font-bold text-text-primary tracking-tight leading-none">设置</h1>
      <button
        v-for="item in navItems"
        :key="item.id"
        class="flex items-center gap-2.5 h-[34px] px-3 rounded-[6px] text-[13px] transition-colors-smooth"
        :class="activeSection === item.id ? 'bg-list-selected text-text-primary font-medium' : 'text-text-secondary hover:bg-list-hover hover:text-text-primary'"
        @click="activeSection = item.id"
      >
        <component
          :is="item.icon"
          class="w-4 h-4"
          :class="activeSection === item.id ? 'text-brand-orange' : 'text-text-muted'"
        />
        {{ item.label }}
      </button>
    </nav>

    <!-- ============ 右栏：当前分区内容 ============ -->
    <div class="flex-1 min-w-0 overflow-y-auto bg-bg-content px-8 py-8">
      <div class="max-w-[560px]">

        <!-- ---- 外观 ---- -->
        <section v-if="activeSection === 'appearance'">
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">外观</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">自定义界面主题与显示模式</p>

          <div class="space-y-2">
            <div
              class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px] transition-opacity"
              :class="uiStore.followSystem ? 'opacity-50' : ''"
            >
              <div>
                <p class="text-[13px] text-text-primary">暗色模式</p>
                <p class="text-[11px] text-text-muted mt-0.5">使用深色主题界面</p>
              </div>
              <ToggleSwitch
                :model-value="uiStore.isDarkMode"
                :disabled="uiStore.followSystem"
                @update:model-value="uiStore.setDarkMode($event)"
              />
            </div>
            <div class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
              <div>
                <p class="text-[13px] text-text-primary">跟随系统</p>
                <p class="text-[11px] text-text-muted mt-0.5">自动跟随操作系统的亮暗设置</p>
              </div>
              <ToggleSwitch
                :model-value="uiStore.followSystem"
                @update:model-value="uiStore.setFollowSystem($event)"
              />
            </div>
          </div>
        </section>

        <!-- ---- 隐私 ---- -->
        <section v-else-if="activeSection === 'privacy'">
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">隐私</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">Lumo 默认完全离线。以下开关决定元数据缺失时是否联网匹配</p>

          <div class="space-y-2">
            <div class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
              <div class="min-w-0 pr-3">
                <p class="text-[13px] text-text-primary">在线歌词匹配</p>
                <p class="text-[11px] text-text-muted mt-0.5">未命中本地歌词时，向 LRCLIB 发送歌名、艺人、专辑与时长以匹配歌词</p>
              </div>
              <ToggleSwitch
                :model-value="uiStore.fetchLyricsOnline"
                @update:model-value="uiStore.setFetchLyricsOnline($event)"
              />
            </div>
            <div class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
              <div class="min-w-0 pr-3">
                <p class="text-[13px] text-text-primary">在线封面匹配</p>
                <p class="text-[11px] text-text-muted mt-0.5">专辑或艺人缺少封面时，向 iTunes 发送对应名称以搜索封面</p>
              </div>
              <ToggleSwitch
                :model-value="uiStore.fetchCoversOnline"
                @update:model-value="uiStore.setFetchCoversOnline($event)"
              />
            </div>
          </div>
        </section>

        <!-- ---- 数据源 ---- -->
        <section v-else-if="activeSection === 'sources'">
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">数据源</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">管理本地文件夹与 WebDAV 连接</p>

          <div class="bg-bg-canvas border border-border-color rounded-[8px] overflow-hidden">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <span class="text-[13px] text-text-primary">已连接 {{ sources.length }} 个数据源</span>
              <button
                class="h-[30px] px-4 rounded-full bg-text-primary text-bg-canvas text-[12px] font-medium hover:opacity-90 transition-opacity"
                @click="showSourceManager = true"
              >管理数据源</button>
            </div>
            <div
              v-for="s in sources"
              :key="s.id"
              class="flex items-center gap-3 px-4 py-2.5 border-b border-border-color last:border-b-0"
            >
              <Server v-if="s.kind === 'webdav'" class="w-4 h-4 text-text-muted shrink-0" />
              <FolderOpen v-else class="w-4 h-4 text-text-muted shrink-0" />
              <div class="min-w-0">
                <p class="text-[13px] text-text-primary truncate leading-tight">{{ s.name }}</p>
                <p class="text-[11px] text-text-muted truncate mt-0.5">{{ s.path }}</p>
              </div>
            </div>
            <div v-if="sources.length === 0" class="px-4 py-6 text-center">
              <p class="text-[12px] text-text-muted">尚未添加数据源</p>
              <p class="text-[11px] text-text-muted/70 mt-1">点击「管理数据源」添加本地文件夹或 WebDAV 连接</p>
            </div>
          </div>
        </section>

        <!-- ---- 数据同步 ---- -->
        <section v-else-if="activeSection === 'sync'">
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">数据同步</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">通过 WebDAV 跨设备同步歌单、收藏和播放历史</p>

          <!-- 开关 -->
          <div class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
            <div>
              <p class="text-[13px] text-text-primary font-medium">启用 WebDAV 数据同步</p>
              <p class="text-[11px] text-text-muted mt-0.5">连接后可上传 / 恢复歌单与播放数据</p>
            </div>
            <ToggleSwitch
              :model-value="syncStore.config.enabled"
              @update:model-value="(v: boolean) => { syncStore.config.enabled = v; syncStore.saveConfig(); }"
            />
          </div>

          <!-- 配置表单（启用后才显示） -->
          <div v-if="syncStore.config.enabled" class="mt-2 border border-border-color rounded-[8px] p-4 bg-bg-canvas">
            <div class="space-y-2">
              <input
                v-model="syncStore.config.webdav_url"
                placeholder="WebDAV 地址（如 https://nas.local/music）"
                class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
                @blur="syncStore.saveConfig()"
              />
              <input
                v-model="syncStore.config.username"
                placeholder="用户名"
                class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
                @blur="syncStore.saveConfig()"
              />
              <input
                v-model="syncStore.config.password"
                type="password"
                placeholder="密码"
                class="w-full h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted outline-none focus:border-brand-orange/50"
                @blur="syncStore.saveConfig()"
              />

              <!-- 同步文件夹选择 -->
              <div class="flex items-center gap-2">
                <input
                  :value="syncStore.config.remote_path || '/'"
                  readonly
                  class="flex-1 min-w-0 h-[34px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[6px] text-text-primary outline-none cursor-default"
                  placeholder="/ (根目录)"
                />
                <button
                  class="h-[34px] px-3 rounded-[6px] text-[12px] bg-list-hover text-text-primary hover:bg-list-selected transition-colors-smooth shrink-0"
                  @click="showFolderPicker = true"
                >
                  <FolderOpen class="w-4 h-4 inline-block mr-1" />
                  选择文件夹
                </button>
              </div>

              <!-- 上次同步时间 -->
              <p v-if="syncStore.config.last_sync_at" class="text-[10px] text-text-muted/60 mt-1">
                上次同步：{{ syncStore.config.last_sync_at }}
                ({{ syncStore.config.last_sync_direction === 'upload' ? '上传' : '下载' }})
              </p>

              <!-- 操作按钮 -->
              <div class="flex items-center gap-2 mt-3">
                <button
                  class="h-[34px] px-4 rounded-full bg-text-primary text-bg-canvas text-[12px] font-medium flex items-center gap-1.5 hover:opacity-90 transition-opacity disabled:opacity-40"
                  :disabled="syncStore.isSyncing || !syncStore.config.webdav_url"
                  @click="syncStore.uploadNow()"
                >
                  <Upload v-if="!syncStore.isSyncing" class="w-3.5 h-3.5" />
                  <RefreshCw v-else class="w-3.5 h-3.5 animate-spin" />
                  {{ syncStore.isSyncing ? '同步中…' : '立即同步' }}
                </button>
                <button
                  class="h-[34px] px-4 rounded-full border border-border-color text-text-secondary text-[12px] font-medium flex items-center gap-1.5 hover:bg-list-hover transition-colors-smooth disabled:opacity-40"
                  :disabled="syncStore.isRestoring || !syncStore.config.webdav_url"
                  @click="syncStore.restoreNow()"
                >
                  <Download v-if="!syncStore.isRestoring" class="w-3.5 h-3.5" />
                  <RefreshCw v-else class="w-3.5 h-3.5 animate-spin" />
                  {{ syncStore.isRestoring ? '恢复中…' : '从云端恢复' }}
                </button>
              </div>

              <!-- 首次检测到云端数据的提示 -->
              <div
                v-if="showRemotePrompt"
                class="mt-3 flex items-start gap-2 px-3 py-2.5 rounded-[8px] bg-bg-active text-[12px] text-text-primary"
              >
                <AlertTriangle class="w-4 h-4 text-brand-orange shrink-0 mt-0.5" />
                <div>
                  检测到云端已有同步数据，是否拉取覆盖本地？
                  <button
                    class="ml-2 font-medium text-brand-orange hover:underline"
                    @click="syncStore.restoreNow(); showRemotePrompt = false"
                  >拉取</button>
                  <button
                    class="ml-2 text-text-muted hover:text-text-primary"
                    @click="showRemotePrompt = false"
                  >忽略</button>
                </div>
              </div>

              <!-- 结果/错误提示 -->
              <p v-if="syncStore.lastResult" class="text-[11px] text-status-success mt-1">{{ syncStore.lastResult }}</p>
              <p v-if="syncStore.lastError" class="text-[11px] text-status-error mt-1">{{ syncStore.lastError }}</p>
            </div>
          </div>
        </section>

        <!-- ---- 存储 ---- -->
        <section v-else-if="activeSection === 'storage'">
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">存储</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">管理封面缩略图与音频缓存</p>

          <div class="flex items-center justify-between px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
            <div>
              <p class="text-[13px] text-text-primary">缓存占用</p>
              <p class="text-[11px] text-text-muted mt-0.5">封面缩略图与音频缓存，共 {{ cacheSize }}</p>
            </div>
            <button
              class="h-[30px] px-3.5 rounded-[6px] text-[12px] bg-list-hover text-text-primary hover:bg-list-selected transition-colors-smooth disabled:opacity-40"
              :disabled="isClearingCache"
              @click="clearCache"
            >{{ isClearingCache ? '清理中…' : '清理缓存' }}</button>
          </div>
        </section>

        <!-- ---- 关于 ---- -->
        <section v-else>
          <h2 class="text-[24px] font-bold text-text-primary tracking-tight leading-none">关于</h2>
          <p class="text-[12px] text-text-muted mt-1.5 mb-6">版本与项目信息</p>

          <div class="flex items-center gap-3 px-4 py-3.5 bg-bg-canvas border border-border-color rounded-[8px]">
            <div class="w-10 h-10 rounded-[10px] bg-bg-content border border-border-color flex items-center justify-center shrink-0">
              <Info class="w-5 h-5 text-text-muted" />
            </div>
            <div>
              <p class="text-[13px] text-text-primary font-medium">Lumo Player</p>
              <p class="text-[11px] text-text-muted font-mono">v1.0.0</p>
              <p class="text-[11px] text-text-muted">一个轻量、温暖的本地音乐播放器</p>
            </div>
          </div>
        </section>

      </div>
    </div>
  </div>

  <!-- 数据源管理弹窗 -->
  <SourceManagerModal v-if="showSourceManager" @close="showSourceManager = false" />

  <!-- WebDAV 文件夹选择器（数据同步用） -->
  <WebdavFolderPicker
    v-if="showFolderPicker"
    @select="(path: string) => { syncStore.config.remote_path = path; syncStore.saveConfig(); showFolderPicker = false; }"
    @close="showFolderPicker = false"
  />
</template>
