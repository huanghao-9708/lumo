<script setup lang="ts">
import { computed } from 'vue';
import {
  Play, Loader2, Heart, MoreHorizontal, Music, ListChecks, CheckSquare,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useBatchSelect } from '../../composables/useBatchSelect';
import BatchActionBar from '../shared/BatchActionBar.vue';
import { formatPlayedAt } from '../../utils/datetime';
import FooterStatus from '../shared/FooterStatus.vue';

const props = defineProps<{
  /** 内容区搜索框传入的过滤词：内存过滤当前列表 */
  filterQuery?: string;
}>();

const playerStore = usePlayerStore();

/* ============ 批量选择（本视图一份实例） ============ */
const batch = useBatchSelect();
const isAllSelected = computed(() => batch.count > 0 && batch.count === visibleTracks.value.length);
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(visibleTracks.value);
}

const tracks = computed(() => playerStore.tracks);

/** 内存过滤（最近播放一次性全量加载，过滤覆盖完整） */
const visibleTracks = computed(() => {
  const q = (props.filterQuery ?? '').trim().toLowerCase();
  if (!q) return tracks.value;
  return tracks.value.filter(t =>
    t.title.toLowerCase().includes(q) ||
    t.artist.toLowerCase().includes(q) ||
    t.album.toLowerCase().includes(q)
  );
});

const totalDurationText = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const days = Math.floor(totalSec / 86400);
  const hours = Math.floor((totalSec % 86400) / 3600);
  const mins = Math.floor((totalSec % 3600) / 60);
  const parts: string[] = [];
  if (days > 0) parts.push(`${days} 天`);
  if (hours > 0) parts.push(`${hours} 小时`);
  parts.push(`${mins} 分钟`);
  return `总计 ${parts.join(' ')}`;
});

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playSong(index: number) {
  // 播放过滤后的列表（与所见一致）
  playerStore.playQueue(visibleTracks.value, index);
}

function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="flex-1 overflow-y-auto px-8">
      <!-- 表头 -->
      <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
        <div class="w-10 text-center shrink-0">#</div>
        <div class="w-8 shrink-0"></div>
        <div class="flex-[2] min-w-0 pl-1">标题</div>
        <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
        <div class="flex-[1.5] min-w-0 hidden md:block">专辑</div>
        <div class="w-[80px] text-right shrink-0">播放时间</div>
        <div class="w-8 shrink-0"></div>
        <!-- 批量选择入口 -->
        <button
          class="ml-2 w-8 shrink-0 flex items-center justify-center text-text-muted hover:text-text-primary transition-colors-smooth"
          :class="batch.isActive ? 'text-brand-orange' : ''"
          :title="batch.isActive ? '退出多选' : '多选歌曲'"
          @click="batch.isActive ? batch.exit() : batch.enter()"
        >
          <ListChecks class="w-[14px] h-[14px]" />
        </button>
      </div>

      <!-- 加载态 -->
      <div v-if="playerStore.isLoadingTracks && tracks.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="tracks.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Music class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">还没有播放记录</span>
      </div>

      <!-- 过滤无结果 -->
      <div v-else-if="visibleTracks.length === 0" class="flex flex-col items-center justify-center py-20 text-text-muted">
        <span class="text-[12px]">没有匹配的歌曲</span>
      </div>

      <!-- 列表 -->
      <div v-else>
        <div
          v-for="(track, index) in visibleTracks"
          :key="track.id"
          class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer"
          style="height: 40px;"
          :class="{ 'playing-row bg-list-selected': isPlayingTrack(track.id), 'bg-list-selected/60': batch.isActive && batch.isSelected(track.id) }"
          @click="batch.isActive && batch.toggle(track)"
          @dblclick="!batch.isActive && playSong(index)"
        >
          <div class="w-10 text-center shrink-0 text-[12px] font-mono">
            <!-- 多选态：序号列换成复选框 -->
            <span
              v-if="batch.isActive"
              class="inline-flex items-center justify-center"
              @click.stop="batch.toggle(track)"
            >
              <span
                class="w-[14px] h-[14px] rounded-[3px] border flex items-center justify-center transition-colors-smooth"
                :class="batch.isSelected(track.id) ? 'bg-brand-orange border-brand-orange' : 'border-border-solid'"
              >
                <CheckSquare v-if="batch.isSelected(track.id)" class="w-[10px] h-[10px] text-white" />
              </span>
            </span>
            <template v-else>
              <span v-if="isPlayingTrack(track.id)" class="text-brand-orange inline-flex items-center justify-center">
                <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
                <Play v-else class="w-[12px] h-[12px] fill-current" />
              </span>
              <template v-else>
                <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
                <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
              </template>
            </template>
          </div>

          <div class="w-8 shrink-0 flex items-center justify-center">
            <!-- 多选态：心形改为切换选择 -->
            <template v-if="batch.isActive">
              <span
                class="w-[14px] h-[14px] rounded-[3px] border flex items-center justify-center transition-colors-smooth"
                :class="batch.isSelected(track.id) ? 'bg-brand-orange border-brand-orange' : 'border-border-solid opacity-0 group-hover:opacity-100'"
                @click.stop="batch.toggle(track)"
              >
                <CheckSquare v-if="batch.isSelected(track.id)" class="w-[10px] h-[10px] text-white" />
              </span>
            </template>
            <template v-else>
              <Heart
                v-if="track.isFavorite" class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
                @click="toggleFav(track.id, $event)"
              />
              <Heart
                v-else class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
                @click="toggleFav(track.id, $event)"
              />
            </template>
          </div>

          <div class="flex-[2] min-w-0 pl-1">
            <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
              {{ track.title }}
            </span>
          </div>

          <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="batch.isActive ? batch.toggle(track) : playerStore.navigateToArtist(track.artistId)">{{ track.artist }}</span></div>

          <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="batch.isActive ? batch.toggle(track) : playerStore.navigateToAlbum(track.albumId)">{{ track.album }}</span></div>

          <!-- 播放时间列 -->
          <div class="w-[80px] text-right shrink-0 text-[12px] font-mono text-text-muted tabular-nums">
            {{ formatPlayedAt(track.playedAt) }}
          </div>

          <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <MoreHorizontal class="w-4 h-4 text-text-muted" />
          </div>
          <div class="w-8 shrink-0"></div>
        </div>
      </div>

      <!-- 批量操作条（多选态） -->
      <BatchActionBar
        v-if="batch.isActive"
        :selected-ids="[...batch.selectedIds]"
        :all-selected="isAllSelected"
        @exit="batch.exit()"
        @toggle-select-all="onToggleSelectAll"
      />

      <!-- Footer -->
    </div>

    <!-- Footer Status（固定在底部） -->
    <FooterStatus v-if="tracks.length > 0" :count="`${tracks.length.toLocaleString()} 首歌曲`" :hint="filterQuery?.trim() ? `过滤后 ${visibleTracks.length} 首` : totalDurationText" />
  </div>
</template>
