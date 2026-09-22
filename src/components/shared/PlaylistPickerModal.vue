<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Loader2, Plus, ListMusic } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import AppModal from './AppModal.vue';

/**
 * 通用「添加到歌单」弹窗。
 *
 * 走 AppModal（Teleport 到 body、z-100），因此不受调用方所在层级影响：
 * 沉浸式播放页那一层是 z-200 的独立层，内联下拉会被压住或裁掉。
 * 支持内联新建歌单并直接把这批曲目加进去。
 */
const props = withDefaults(defineProps<{
  /** 要加入歌单的曲目 id 列表（单曲传一个） */
  trackIds: number[];
  /** 右上角展示的曲目标题（可选） */
  trackTitle?: string;
}>(), { trackTitle: '' });

const emit = defineEmits<{ close: []; added: [name: string] }>();

const playerStore = usePlayerStore();
const uiStore = useUiStore();

const acting = ref(false);
const creating = ref(false);
const newName = ref('');

const hasTracks = computed(() => props.trackIds.length > 0);

onMounted(() => {
  if (playerStore.playlists.length === 0) playerStore.fetchPlaylists();
});

async function addTo(playlistId: number, name: string) {
  if (acting.value || !hasTracks.value) return;
  acting.value = true;
  try {
    const { added, skipped } = await playerStore.batchAddToPlaylist(playlistId, props.trackIds);
    let msg = `已添加 ${added} 首到「${name}」`;
    if (skipped > 0) msg += `，${skipped} 首已在其中`;
    uiStore.showToast(msg);
    emit('added', name);
    emit('close');
  } catch (e) {
    console.error('Failed to add to playlist:', e);
    uiStore.showToast('添加失败，请重试');
  } finally {
    acting.value = false;
  }
}

async function createAndAdd() {
  const name = newName.value.trim();
  if (!name || acting.value) return;
  acting.value = true;
  try {
    const id = await playerStore.createPlaylist(name, '');
    const { added } = await playerStore.batchAddToPlaylist(id, props.trackIds);
    uiStore.showToast(`已新建「${name}」并添加 ${added} 首`);
    newName.value = '';
    creating.value = false;
    emit('added', name);
    emit('close');
  } catch (e) {
    console.error('Failed to create playlist:', e);
    uiStore.showToast('创建歌单失败，请重试');
  } finally {
    acting.value = false;
  }
}
</script>

<template>
  <AppModal width="380px" :close-on-overlay="!acting" :close-on-esc="!acting" @close="emit('close')">
    <div class="px-6 pt-6 pb-3">
      <h2 class="text-[18px] font-bold text-text-primary mb-1">添加到歌单</h2>
      <p class="text-[12px] text-text-muted truncate">
        {{ trackTitle ? `${trackTitle} · ${trackIds.length} 首` : `${trackIds.length} 首歌曲` }}
      </p>
    </div>

    <div class="px-3 pb-2">
      <div
        v-if="playerStore.playlists.length === 0"
        class="px-3 py-6 text-center text-[12px] text-text-muted"
      >
        还没有歌单，先新建一个吧
      </div>

      <button
        v-for="pl in playerStore.playlists"
        :key="pl.id"
        class="w-full flex items-center gap-3 px-3 py-2 rounded-[8px] hover:bg-list-hover transition-colors-smooth text-left disabled:opacity-50"
        :disabled="acting || !hasTracks"
        @click="addTo(pl.id, pl.name)"
      >
        <span class="w-9 h-9 rounded-[6px] overflow-hidden bg-bg-hover flex items-center justify-center flex-shrink-0">
          <img v-if="pl.cover_thumb" :src="pl.cover_thumb" class="w-full h-full object-cover" alt="" />
          <ListMusic v-else class="w-4 h-4 text-text-disabled" />
        </span>
        <span class="flex-1 min-w-0">
          <span class="block text-[13px] text-text-primary truncate">{{ pl.name }}</span>
          <span class="block text-[11px] text-text-muted font-mono tabular-nums">{{ pl.count }} 首</span>
        </span>
        <Loader2 v-if="acting" class="w-3.5 h-3.5 animate-spin text-text-muted flex-shrink-0" />
      </button>
    </div>

    <div class="px-6 pb-6 pt-2">
      <div v-if="creating" class="flex items-center gap-2">
        <input
          v-model="newName"
          type="text"
          placeholder="输入歌单名称"
          class="flex-1 h-[36px] px-3 text-[13px] bg-bg-content border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50 outline-none"
          @keydown.enter.prevent="createAndAdd"
          @keydown.esc.prevent="creating = false"
        />
        <button
          class="h-[36px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity disabled:opacity-40"
          :disabled="!newName.trim() || acting"
          @click="createAndAdd"
        >
          <Loader2 v-if="acting" class="w-3.5 h-3.5 animate-spin" />
          添加
        </button>
      </div>
      <button
        v-else
        class="w-full h-[34px] flex items-center justify-center gap-2 text-[13px] text-text-secondary hover:text-text-primary transition-colors-smooth"
        @click="creating = true"
      >
        <Plus class="w-[14px] h-[14px]" />
        新建歌单…
      </button>
    </div>
  </AppModal>
</template>
