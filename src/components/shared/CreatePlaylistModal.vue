<script setup lang="ts">
import { ref } from 'vue';
import { usePlayerStore } from '../../stores/player';

const playerStore = usePlayerStore();

const name = ref('');
const description = ref('');
const isCreating = ref(false);
const error = ref('');

async function confirm() {
  if (!name.value.trim()) {
    error.value = '请输入歌单名称';
    return;
  }
  isCreating.value = true;
  error.value = '';
  try {
    const newId = await playerStore.createPlaylist(name.value.trim(), description.value.trim());
    if (newId) {
      playerStore.isCreatePlaylistModalOpen = false;
      // 自动选中新创建的歌单
      playerStore.activePlaylistId = newId;
      playerStore.activeLibraryTab = '播放列表';
    }
  } catch (e) {
    error.value = '创建失败，请重试';
  } finally {
    isCreating.value = false;
  }
}

function cancel() {
  playerStore.isCreatePlaylistModalOpen = false;
}

function onOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget) cancel();
}
</script>

<template>
  <div
    class="fixed inset-0 z-[100] bg-black/30 flex items-center justify-center"
    @click="onOverlayClick"
  >
    <div class="bg-bg-canvas rounded-[12px] w-[380px] shadow-lg overflow-hidden">
      <!-- Header -->
      <div class="px-6 pt-6 pb-4">
        <h2 class="text-[18px] font-bold text-text-primary mb-1">新建歌单</h2>
        <p class="text-[12px] text-text-muted">创建一个新的播放列表</p>
      </div>

      <!-- Form -->
      <div class="px-6 pb-4 space-y-3">
        <div>
          <label class="block text-[11px] text-text-muted mb-1.5 font-medium">名称</label>
          <input
            v-model="name"
            type="text"
            placeholder="输入歌单名称"
            class="w-full h-[36px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50 outline-none"
            @keyup.enter="confirm"
          />
        </div>
        <div>
          <label class="block text-[11px] text-text-muted mb-1.5 font-medium">描述（可选）</label>
          <textarea
            v-model="description"
            rows="2"
            placeholder="添加描述…"
            class="w-full px-3 py-2 text-[13px] bg-bg-content border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50 outline-none resize-none"
          />
        </div>
        <p v-if="error" class="text-[11px] text-red-500">{{ error }}</p>
      </div>

      <!-- Actions -->
      <div class="px-6 pb-6 flex items-center justify-end gap-2">
        <button
          class="h-[34px] px-4 text-[13px] text-text-secondary hover:text-text-primary transition-colors-smooth"
          @click="cancel"
        >取消</button>
        <button
          class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
          :disabled="isCreating"
          @click="confirm"
        >
          {{ isCreating ? '创建中…' : '创建' }}
        </button>
      </div>
    </div>
  </div>
</template>
