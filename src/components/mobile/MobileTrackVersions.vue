<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue';
import { Check, HardDrive, Server, Loader2 } from 'lucide-vue-next';
import { libraryGetTrackVersions, librarySetPrimaryFile } from '../../api/library';
import type { TrackFileInfoDTO } from '../../api/types';
import type { Track } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { registerBackHandler } from '../../composables/useMobileBack';

const props = defineProps<{
  visible: boolean;
  track: Track | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'versionChanged', newPrimaryFileId: number): void;
}>();

const uiStore = useUiStore();
const versions = ref<TrackFileInfoDTO[]>([]);
const isLoading = ref(false);
const isSwitching = ref(false);
const isLeaving = ref(false);

watch(() => [props.visible, props.track], async ([vis, trk]) => {
  if (vis && trk) {
    isLoading.value = true;
    try {
      versions.value = await libraryGetTrackVersions((trk as Track).id);
    } catch (e) {
      console.error('获取文件版本失败:', e);
      versions.value = [];
    } finally {
      isLoading.value = false;
    }
  } else if (!vis) {
    isLeaving.value = false;
  }
}, { immediate: true });

function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || bytes <= 0) return '--';
  const units = ['B', 'KB', 'MB', 'GB'];
  let val = bytes;
  let i = 0;
  while (val >= 1024 && i < units.length - 1) {
    val /= 1024;
    i++;
  }
  return `${val >= 100 || i === 0 ? Math.round(val) : val.toFixed(1)} ${units[i]}`;
}

function qualityText(v: TrackFileInfoDTO): string {
  const parts: string[] = [];
  if (v.format || v.file_ext) {
    parts.push((v.format || v.file_ext || '').toUpperCase());
  }
  if (v.bit_depth && v.sample_rate) {
    const khz = v.sample_rate % 1000 === 0 ? (v.sample_rate / 1000).toFixed(0) : (v.sample_rate / 1000).toFixed(1);
    parts.push(`${v.bit_depth}bit / ${khz}kHz`);
  } else if (v.bitrate) {
    parts.push(`${Math.round(v.bitrate / 1000)} kbps`);
  }
  if (v.channels === 1) parts.push('单声道');
  else if (v.channels === 2) parts.push('立体声');
  return parts.join(' · ');
}

async function setPrimary(fileId: number) {
  if (!props.track || isSwitching.value) return;
  if (fileId === props.track.primary_file_id) return;
  
  isSwitching.value = true;
  try {
    await librarySetPrimaryFile(props.track.id, fileId);
    uiStore.showToast('已切换主音频版本', 'info');
    emit('versionChanged', fileId);
    close();
  } catch (e: any) {
    uiStore.showToast(e.message || '切换失败', 'error');
  } finally {
    isSwitching.value = false;
  }
}

let closeTimer: ReturnType<typeof setTimeout> | null = null;
function close() {
  isLeaving.value = true;
  if (closeTimer) clearTimeout(closeTimer);
  closeTimer = setTimeout(() => {
    isLeaving.value = false;
    closeTimer = null;
    emit('close');
  }, 250);
}

const unregisterBack = registerBackHandler(() => {
  if (props.visible) {
    close();
    return true;
  }
  return false;
});

onBeforeUnmount(() => {
  unregisterBack();
  if (closeTimer) clearTimeout(closeTimer);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[220] flex flex-col justify-end">
      <!-- 遮罩 -->
      <Transition name="sheet-fade">
        <div
          v-if="visible && !isLeaving"
          class="absolute inset-0 bg-black/40"
          @click="close"
        ></div>
      </Transition>

      <!-- 抽屉面板 -->
      <Transition name="sheet-slide">
        <div
          v-if="visible && !isLeaving"
          class="relative bg-bg-canvas rounded-t-[16px] overflow-hidden flex flex-col max-h-[75vh]"
          style="padding-bottom: env(safe-area-inset-bottom);"
        >
          <!-- 顶部把手 -->
          <div class="pt-3 pb-1 flex justify-center">
            <div class="w-10 h-1 rounded-full bg-border-solid"></div>
          </div>

          <!-- 标题信息 -->
          <div class="px-5 pt-2 pb-3 border-b border-border-color">
            <h3 class="text-[16px] font-bold text-text-primary truncate">音频版本与规格</h3>
            <p v-if="track" class="text-[13px] text-text-muted truncate mt-0.5">
              {{ track.title }} — {{ track.artist }}
            </p>
          </div>

          <!-- 列表区 -->
          <div class="flex-1 overflow-y-auto p-4 space-y-2.5">
            <div v-if="isLoading" class="flex flex-col items-center justify-center py-10 gap-2 text-text-muted">
              <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
              <span class="text-[12px]">读取文件版本…</span>
            </div>

            <div v-else-if="versions.length === 0" class="flex flex-col items-center justify-center py-10 text-text-muted">
              <span class="text-[13px]">暂无可用版本详情</span>
            </div>

            <template v-else>
              <div
                v-for="v in versions"
                :key="v.id"
                class="rounded-[10px] p-3 border transition-colors-smooth cursor-pointer active:bg-list-hover relative flex items-start gap-3"
                :class="v.id === track?.primary_file_id
                  ? 'border-brand-orange bg-brand-orange/10'
                  : 'border-border-color bg-bg-content'"
                @click="setPrimary(v.id)"
              >
                <!-- 图标 -->
                <div class="w-9 h-9 rounded-[8px] flex items-center justify-center flex-shrink-0 mt-0.5"
                  :class="v.id === track?.primary_file_id ? 'bg-brand-orange text-white' : 'bg-bg-hover text-text-muted'"
                >
                  <component :is="v.source_kind === 'webdav' ? Server : HardDrive" class="w-5 h-5" />
                </div>

                <!-- 详情 -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="text-[14px] font-semibold text-text-primary">
                      {{ (v.format || v.file_ext || '音频').toUpperCase() }}
                    </span>
                    <span
                      v-if="v.id === track?.primary_file_id"
                      class="text-[10px] font-medium text-brand-orange bg-brand-orange/20 px-1.5 py-0.5 rounded-full"
                    >
                      当前主版本
                    </span>
                  </div>

                  <p class="text-[12px] font-mono text-text-secondary mt-1">
                    {{ qualityText(v) }} · {{ formatBytes(v.file_size) }}
                  </p>

                  <p class="text-[11px] font-mono text-text-muted truncate mt-1">
                    {{ v.relative_path || v.path }}
                  </p>
                </div>

                <!-- 选定钩 -->
                <Check
                  v-if="v.id === track?.primary_file_id"
                  class="w-5 h-5 text-brand-orange flex-shrink-0 mt-1"
                />
              </div>
            </template>
          </div>

          <!-- 关闭按钮 -->
          <div class="p-3 border-t border-border-color bg-bg-canvas">
            <button
              class="w-full h-11 rounded-[10px] bg-bg-hover text-text-primary text-[15px] font-medium active:opacity-80 transition-opacity"
              @click="close"
            >
              关闭
            </button>
          </div>
        </div>
      </Transition>
    </div>
  </Teleport>
</template>
