<script setup lang="ts">
import { ref } from 'vue';
import { X, Heart, ListMusic, Plus, Loader2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';

/**
 * 批量操作条（迭代记录 2.4）：已选计数、全选/取消全选、
 * 添加到喜欢的音乐、带下拉的歌单选择（支持内联新建歌单）、退出多选。
 * 动作直接走 playerStore（batchSetFavorite / batchAddToPlaylist），
 * 视图只负责传选择结果与处理 exit / toggleSelectAll。
 */
const props = defineProps<{
  /** 已选曲目 id（保持选择顺序） */
  selectedIds: number[];
  /** 当前视图列表是否已全选 */
  allSelected: boolean;
  /** 隐藏「添加到喜欢的音乐」（如喜欢的音乐页，避免自我包含） */
  hideFavoriteAction?: boolean;
  /** 操作进行中（由视图传入可选；组件内部动作自带 loading 态） */
  busy?: boolean;
}>();

const emit = defineEmits<{
  (e: 'exit'): void;
  (e: 'toggle-select-all'): void;
}>();

const playerStore = usePlayerStore();
const uiStore = useUiStore();

/* ============ 歌单下拉 ============ */
const playlistMenuOpen = ref(false);
const creatingPlaylist = ref(false);
const newPlaylistName = ref('');
const acting = ref(false);

function togglePlaylistMenu() {
  playlistMenuOpen.value = !playlistMenuOpen.value;
  if (!playlistMenuOpen.value) creatingPlaylist.value = false;
}

async function addFavorite() {
  if (acting.value || props.selectedIds.length === 0) return;
  acting.value = true;
  try {
    await playerStore.batchSetFavorite(props.selectedIds, true);
    uiStore.showToast(`已添加 ${props.selectedIds.length} 首歌曲到喜欢的音乐`);
    emit('exit');
  } catch {
    uiStore.showToast('批量收藏失败，请重试');
  } finally {
    acting.value = false;
  }
}

async function addToPlaylist(playlistId: number, playlistName: string) {
  if (acting.value || props.selectedIds.length === 0) return;
  acting.value = true;
  try {
    const { added, skipped } = await playerStore.batchAddToPlaylist(playlistId, props.selectedIds);
    uiStore.showToast(
      skipped > 0
        ? `已添加 ${added} 首，跳过 ${skipped} 首重复`
        : `已添加 ${added} 首歌曲到「${playlistName}」`
    );
    playlistMenuOpen.value = false;
    emit('exit');
  } catch {
    uiStore.showToast('添加失败，请重试');
  } finally {
    acting.value = false;
  }
}

/** 内联「新建歌单…」：一键创建并把所选歌曲加进去 */
async function createPlaylistAndAdd() {
  const name = newPlaylistName.value.trim();
  if (!name || acting.value) return;
  acting.value = true;
  try {
    const playlistId = await playerStore.createPlaylist(name, '');
    const { added } = await playerStore.batchAddToPlaylist(playlistId, props.selectedIds);
    uiStore.showToast(`已创建「${name}」并添加 ${added} 首`);
    newPlaylistName.value = '';
    creatingPlaylist.value = false;
    playlistMenuOpen.value = false;
    emit('exit');
  } catch {
    uiStore.showToast('创建歌单失败，请重试');
  } finally {
    acting.value = false;
  }
}
</script>

<template>
  <div class="flex-shrink-0 px-8 py-2.5 border-t border-border-color bg-bg-content flex items-center gap-2 select-none">
    <!-- 退出多选 -->
    <button
      class="h-7 w-7 flex items-center justify-center rounded-[6px] text-text-muted hover:text-text-primary hover:bg-list-hover transition-colors-smooth"
      title="退出多选"
      @click="emit('exit')"
    >
      <X class="w-4 h-4" />
    </button>

    <!-- 已选计数 -->
    <span class="text-[12px] text-text-secondary">
      已选 <span class="font-mono tabular-nums text-text-primary">{{ selectedIds.length }}</span> 首
    </span>

    <div class="flex-1"></div>

    <!-- 全选 / 取消全选 -->
    <button
      class="h-7 px-3 rounded-[6px] text-[12px] text-text-secondary hover:bg-list-hover transition-colors-smooth"
      @click="emit('toggle-select-all')"
    >{{ allSelected ? '取消全选' : '全选' }}</button>

    <!-- 添加到喜欢的音乐 -->
    <button
      v-if="!hideFavoriteAction"
      class="h-7 px-3 rounded-[6px] text-[12px] flex items-center gap-1.5 text-text-secondary hover:text-text-primary hover:bg-list-hover transition-colors-smooth disabled:opacity-40"
      :disabled="acting || selectedIds.length === 0"
      @click="addFavorite"
    >
      <Loader2 v-if="acting" class="w-3.5 h-3.5 animate-spin" />
      <Heart v-else class="w-3.5 h-3.5" />
      添加到喜欢的音乐
    </button>

    <!-- 添加到歌单（下拉） -->
    <div class="relative">
      <button
        class="h-7 px-3 rounded-[6px] text-[12px] flex items-center gap-1.5 text-text-secondary hover:text-text-primary hover:bg-list-hover transition-colors-smooth"
        @click="togglePlaylistMenu"
      >
        <ListMusic class="w-3.5 h-3.5" />
        添加到歌单
      </button>

      <div
        v-if="playlistMenuOpen"
        class="absolute bottom-full right-0 mb-2 w-[240px] max-h-[300px] overflow-y-auto bg-bg-content border border-border-color rounded-[8px] shadow-lg z-50 py-1"
      >
        <p class="px-3 py-1.5 text-[10px] font-semibold text-text-muted uppercase tracking-widest">添加到</p>

        <button
          v-for="pl in playerStore.playlists"
          :key="pl.id"
          class="w-full text-left px-3 py-2 text-[13px] text-text-primary hover:bg-list-hover transition-colors-smooth flex items-center justify-between gap-2"
          @click="addToPlaylist(pl.id, pl.name)"
        >
          <span class="truncate">{{ pl.name }}</span>
          <span class="text-[11px] font-mono text-text-muted flex-shrink-0">{{ pl.count }}</span>
        </button>

        <p v-if="playerStore.playlists.length === 0" class="px-3 py-2 text-[12px] text-text-muted/70">暂无歌单</p>

        <div class="border-t border-border-color mt-1 pt-1">
          <!-- 内联新建：输入歌单名 → 创建并添加 -->
          <div v-if="creatingPlaylist" class="px-3 py-2">
            <input
              v-model="newPlaylistName"
              type="text"
              placeholder="歌单名称"
              autofocus
              class="w-full h-[30px] px-2.5 text-[12px] bg-bg-canvas border border-border-color rounded-[6px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50"
              @keydown.enter.prevent="createPlaylistAndAdd"
              @keydown.esc.prevent="creatingPlaylist = false"
            />
            <div class="flex justify-end gap-2 mt-2">
              <button
                class="h-[26px] px-2.5 rounded-[6px] text-[11px] text-text-muted hover:bg-list-hover transition-colors-smooth"
                @click="creatingPlaylist = false"
              >取消</button>
              <button
                class="h-[26px] px-2.5 rounded-[6px] text-[11px] bg-text-primary text-bg-canvas hover:opacity-90 transition-opacity flex items-center gap-1 disabled:opacity-40"
                :disabled="!newPlaylistName.trim() || acting"
                @click="createPlaylistAndAdd"
              >
                <Loader2 v-if="acting" class="w-3 h-3 animate-spin" />
                创建并添加
              </button>
            </div>
          </div>
          <button
            v-else
            class="w-full text-left px-3 py-2 text-[13px] text-text-secondary hover:bg-list-hover transition-colors-smooth flex items-center gap-1.5"
            @click="creatingPlaylist = true"
          >
            <Plus class="w-3.5 h-3.5" />
            新建歌单…
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
