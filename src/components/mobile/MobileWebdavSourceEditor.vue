<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue';
import { ChevronLeft, CheckCircle2, XCircle, Loader2, Eye, EyeOff, AlertTriangle, Scan } from 'lucide-vue-next';
import { scannerTestWebdav, type WebdavProbeResult } from '../../api/scanner';
import { usePlayerStore } from '../../stores/player';
import { registerBackHandler } from '../../composables/useMobileBack';

const emit = defineEmits<{ (e: 'close'): void }>();
const playerStore = usePlayerStore();

const name = ref('');
const url = ref('');
const username = ref('');
const password = ref('');
const showPassword = ref(false);

const testing = ref(false);
const testResult = ref<WebdavProbeResult | null>(null);

const isHttp = computed(() => url.value.trim().toLowerCase().startsWith('http://'));

const canSubmit = computed(() => {
  return name.value.trim().length > 0 && url.value.trim().length > 0 && !submitting.value;
});

const submitting = ref(false);
const submitError = ref('');

async function onTest() {
  if (!url.value.trim()) return;
  testing.value = true;
  testResult.value = null;
  submitError.value = '';
  try {
    const res = await scannerTestWebdav(
      url.value.trim(),
      username.value.trim() || undefined,
      password.value || undefined
    );
    testResult.value = res;
  } catch (e: any) {
    testResult.value = {
      ok: false,
      latencyMs: 0,
      error: e?.message || String(e),
    };
  } finally {
    testing.value = false;
  }
}

async function onSubmit() {
  if (!canSubmit.value) return;
  submitting.value = true;
  submitError.value = '';
  try {
    const cleanUrl = url.value.trim();
    const cleanName = name.value.trim();
    const cleanUser = username.value.trim() || undefined;
    const cleanPass = password.value || undefined;

    const newSourceId = await playerStore.addSource('webdav', cleanName, cleanUrl, cleanUser, cleanPass);
    if (typeof newSourceId === 'number') {
      playerStore.scanSource(newSourceId);
    }
    emit('close');
  } catch (e: any) {
    submitError.value = e?.message || '添加失败，请检查配置及网络';
  } finally {
    submitting.value = false;
  }
}

// 返回拦截：返回键关闭本页面
const unregisterBack = registerBackHandler(() => {
  emit('close');
  return true;
});
onBeforeUnmount(unregisterBack);
</script>

<template>
  <div class="fixed inset-0 z-50 bg-bg-content flex flex-col">
    <!-- 顶栏 -->
    <header class="flex items-center gap-2 px-3 h-12 border-b border-border-color bg-bg-canvas flex-shrink-0">
      <button
        class="p-2 -ml-1 text-text-secondary active:text-text-primary active:bg-list-hover rounded-full transition-colors-smooth"
        aria-label="返回"
        @click="emit('close')"
      >
        <ChevronLeft class="w-6 h-6" />
      </button>
      <h1 class="text-[17px] font-semibold text-text-primary flex-1 truncate">
        添加 WebDAV 音乐库
      </h1>
    </header>

    <!-- 表单主体 -->
    <main class="flex-1 overflow-y-auto p-4 space-y-4">
      <div class="rounded-[12px] bg-bg-canvas border border-border-color p-4 space-y-4">
        <!-- 来源名称 -->
        <div>
          <label class="block text-[13px] font-medium text-text-muted mb-1.5">来源名称</label>
          <input
            v-model="name"
            type="text"
            placeholder="例如：家庭 NAS / 坚果云"
            class="w-full h-11 px-3 rounded-[8px] bg-bg-content border border-border-solid text-[14px] text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-brand-orange"
          />
        </div>

        <!-- 服务器地址 -->
        <div>
          <label class="block text-[13px] font-medium text-text-muted mb-1.5">服务器地址 (URL)</label>
          <input
            v-model="url"
            type="url"
            placeholder="https://dav.example.com/music"
            class="w-full h-11 px-3 rounded-[8px] bg-bg-content border border-border-solid text-[14px] text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-brand-orange font-mono"
            @input="testResult = null"
          />
          <!-- http 风险提示 -->
          <div v-if="isHttp" class="flex items-center gap-1.5 mt-2 text-[12px] text-amber-500">
            <AlertTriangle class="w-3.5 h-3.5 flex-shrink-0" />
            <span>使用未加密的 HTTP 连接，在公共网络下传输可能有凭据泄露风险</span>
          </div>
        </div>

        <!-- 用户名 -->
        <div>
          <label class="block text-[13px] font-medium text-text-muted mb-1.5">用户名（可选）</label>
          <input
            v-model="username"
            type="text"
            autocomplete="username"
            placeholder="WebDAV 用户名"
            class="w-full h-11 px-3 rounded-[8px] bg-bg-content border border-border-solid text-[14px] text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-brand-orange"
          />
        </div>

        <!-- 密码 -->
        <div>
          <label class="block text-[13px] font-medium text-text-muted mb-1.5">密码（可选）</label>
          <div class="relative">
            <input
              v-model="password"
              :type="showPassword ? 'text' : 'password'"
              autocomplete="current-password"
              placeholder="WebDAV 访问密码"
              class="w-full h-11 pl-3 pr-10 rounded-[8px] bg-bg-content border border-border-solid text-[14px] text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-brand-orange"
            />
            <button
              type="button"
              class="absolute right-2.5 top-2.5 p-1 text-text-muted active:text-text-primary"
              @click="showPassword = !showPassword"
            >
              <component :is="showPassword ? EyeOff : Eye" class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- 测试连接反馈卡片 -->
        <div v-if="testResult" class="p-3 rounded-[8px] text-[13px]" :class="testResult.ok ? 'bg-emerald-500/10 text-emerald-600 border border-emerald-500/20' : 'bg-red-500/10 text-red-600 border border-red-500/20'">
          <div class="flex items-start gap-2">
            <component :is="testResult.ok ? CheckCircle2 : XCircle" class="w-4 h-4 mt-0.5 flex-shrink-0" />
            <div class="flex-1 min-w-0">
              <p class="font-medium">
                {{ testResult.ok ? `连接成功 (${testResult.latencyMs}ms)` : '连接失败' }}
              </p>
              <p v-if="testResult.serverHeader" class="text-[12px] opacity-80 mt-0.5 font-mono">
                服务端: {{ testResult.serverHeader }}
              </p>
              <p v-if="testResult.error" class="text-[12px] mt-1 opacity-90 break-all">
                {{ testResult.error }}
              </p>
            </div>
          </div>
        </div>

        <!-- 提交错误 -->
        <div v-if="submitError" class="p-3 rounded-[8px] text-[13px] bg-red-500/10 text-red-600 border border-red-500/20 flex items-center gap-2">
          <XCircle class="w-4 h-4 flex-shrink-0" />
          <span>{{ submitError }}</span>
        </div>

        <!-- 操作按钮组 -->
        <div class="pt-2 flex items-center gap-3">
          <button
            type="button"
            class="flex-1 h-11 rounded-[8px] border border-border-solid text-[14px] font-medium text-text-secondary active:bg-list-hover transition-colors-smooth flex items-center justify-center gap-2 disabled:opacity-50"
            :disabled="!url.trim() || testing"
            @click="onTest"
          >
            <Loader2 v-if="testing" class="w-4 h-4 animate-spin text-brand-orange" />
            <span>{{ testing ? '测试中…' : '测试连接' }}</span>
          </button>

          <button
            type="button"
            class="flex-1 h-11 rounded-[8px] bg-brand-orange text-white text-[14px] font-medium active:opacity-90 transition-opacity flex items-center justify-center gap-2 disabled:opacity-50"
            :disabled="!canSubmit"
            @click="onSubmit"
          >
            <Loader2 v-if="submitting" class="w-4 h-4 animate-spin" />
            <Scan v-else class="w-4 h-4" />
            <span>{{ submitting ? '保存中…' : '保存并扫描' }}</span>
          </button>
        </div>
      </div>
    </main>
  </div>
</template>
