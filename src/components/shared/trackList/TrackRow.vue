<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play, Heart, ListPlus, CloudOff } from 'lucide-vue-next';
import type { Track } from '../../../stores/player';
import { usePlayerStore } from '../../../stores/player';
import type { TrackColumnDef } from './columns';
import { TRACK_ROW_HEIGHT, columnCellStyle, columnAlignClass } from './columns';
import { formatAudioInfo, formatFileSize, formatTrackGenres, formatTrackYear } from './format';
import { formatPlayedAt } from '../../../utils/datetime';
import EqualizerIndicator from '../../shared/EqualizerIndicator.vue';
import PlaylistPickerModal from '../../shared/PlaylistPickerModal.vue';

/**
 * 统一歌曲行（LDL v2 song-row）：与 TrackListHeader 由同一份列配置驱动。
 *
 * 行内通用行为（收藏、艺人/专辑跳转）直接走 playerStore；
 * 页面差异通过 props 传入状态（playing/selected/greyed）、事件上抛
 * （play / toggle-select），组件内部不做页面名称判断。
 */
const props = withDefaults(defineProps<{
  track: Track;
  columns: TrackColumnDef[];
  /** 渲染下标（0 起始） */
  index: number;
  /** 展示的序号（默认 index+1；队列过滤场景传原始位置） */
  displayIndex?: number;
  /** 该行是当前播放曲目 */
  playing?: boolean;
  /** 全局播放中状态（驱动均衡器动画） */
  isPlayingNow?: boolean;
  /** 不可播（离线未缓存/文件丢失）置灰 */
  greyed?: boolean;
  /** 置灰提示文案 */
  greyTitle?: string;
  /** 多选模式 */
  batchMode?: boolean;
  /** 多选模式下该行是否选中 */
  selected?: boolean;
  rowHeight?: number;
  /** 紧凑模式（播放队列面板）：单击即播放、隐藏 more */
  compact?: boolean;
  /** 隐藏 more 操作列内容（GlobalSearch 等轻列表） */
  showMore?: boolean;
  /**
   * 行尾占位宽度（px）：补齐表头尾部控件（批量入口 40 / 显示列菜单 36），
   * 与表头传同一值（useTrackColumns 的 trailingExtraWidth），保证表头与行对齐。
   */
  trailingWidth?: number;
}>(), {
  rowHeight: TRACK_ROW_HEIGHT,
  showMore: true,
  trailingWidth: 0,
});

const emit = defineEmits<{
  play: [];
  toggleSelect: [];
}>();

const playerStore = usePlayerStore();
const playlistPickerOpen = ref(false);

const displayNo = computed(() => String((props.displayIndex ?? props.index + 1)).padStart(2, '0'));

function onRowClick() {
  if (props.compact) {
    emit('play');
    return;
  }
  if (props.batchMode) emit('toggleSelect');
}

function onRowDblclick() {
  if (props.compact || props.batchMode) return;
  emit('play');
}

function toggleFavorite(e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(props.track.id);
}

const fileSizeText = computed(() => formatFileSize(props.track.fileSize));
const yearText = computed(() => formatTrackYear(props.track.year));
const genreText = computed(() => formatTrackGenres(props.track.genres));
const audioInfoText = computed(() => formatAudioInfo(props.track));
const playedAtText = computed(() => formatPlayedAt(props.track.playedAt));
</script>

<template>
  <div
    class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
    :style="{ height: rowHeight + 'px' }"
    :class="{
      'playing-row bg-list-selected': playing,
      'bg-list-selected ring-1 ring-inset ring-brand-orange/40': batchMode && selected,
    }"
    :data-track-id="track.id"
    :data-selected="batchMode ? String(selected === true) : undefined"
    @click="onRowClick"
    @dblclick="onRowDblclick"
  >
    <template v-for="col in columns" :key="col.id">
      <!-- 序号 / 复选框 / 播放图标 -->
      <div v-if="col.id === 'index'" :style="columnCellStyle(col)" class="text-center text-[12px] font-mono shrink-0">
        <input
          v-if="batchMode"
          type="checkbox"
          :checked="selected"
          :aria-label="`选择歌曲：${track.title}`"
          class="w-3.5 h-3.5 accent-brand-orange cursor-pointer"
          @click.stop
          @change="emit('toggleSelect')"
        />
        <template v-else>
          <span v-if="playing" class="inline-flex items-center justify-center">
            <EqualizerIndicator :playing="isPlayingNow" />
          </span>
          <template v-else>
            <span class="text-text-muted group-hover:hidden tabular-nums">{{ displayNo }}</span>
            <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
          </template>
        </template>
      </div>

      <!-- 收藏（多选态保留列宽，选择统一放在序号列） -->
      <div v-else-if="col.id === 'favorite'" :style="columnCellStyle(col)" class="flex items-center justify-center shrink-0">
        <template v-if="!batchMode">
          <Heart
            v-if="track.isFavorite"
            class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
            @click="toggleFavorite"
          />
          <Heart
            v-else
            class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
            @click="toggleFavorite"
          />
        </template>
      </div>

      <!-- 标题（悬停/聚焦可看全名） -->
      <div v-else-if="col.id === 'title'" :style="columnCellStyle(col)" class="min-w-0 pl-1">
        <span
          class="text-[13px] truncate block"
          :title="track.title"
          :class="playing ? 'text-brand-orange font-semibold' : greyed ? 'text-text-disabled' : 'text-text-primary font-medium'"
        >{{ track.title }}</span>
      </div>

      <!-- 艺术家 -->
      <div v-else-if="col.id === 'artist'" :style="columnCellStyle(col)" class="min-w-0 text-[13px] truncate shrink-0" :class="[columnAlignClass(col), greyed ? 'text-text-disabled' : 'text-text-secondary']">
        <span class="hover:underline cursor-pointer" :title="track.artist" @click.stop="batchMode ? emit('toggleSelect') : playerStore.navigateToArtist(track.artistId)">{{ track.artist }}</span>
      </div>

      <!-- 专辑 -->
      <div v-else-if="col.id === 'album'" :style="columnCellStyle(col)" class="min-w-0 text-[13px] truncate shrink-0" :class="[columnAlignClass(col), greyed ? 'text-text-disabled' : 'text-text-secondary']">
        <span class="hover:underline cursor-pointer" :title="track.album" @click.stop="batchMode ? emit('toggleSelect') : playerStore.navigateToAlbum(track.albumId)">{{ track.album }}</span>
      </div>

      <!-- 时长 -->
      <div v-else-if="col.id === 'duration'" :style="columnCellStyle(col)" class="text-[12px] font-mono text-text-muted tabular-nums shrink-0" :class="columnAlignClass(col)">
        {{ track.duration }}
      </div>

      <!-- 音频信息（只描述可验证的事实） -->
      <div v-else-if="col.id === 'audioInfo'" :style="columnCellStyle(col)" class="min-w-0 shrink-0" :class="columnAlignClass(col)">
        <span class="text-[10px] font-mono uppercase truncate block" :title="audioInfoText" :class="greyed ? 'text-text-disabled' : 'text-text-muted'">{{ audioInfoText }}</span>
      </div>

      <!-- 年份 -->
      <div v-else-if="col.id === 'year'" :style="columnCellStyle(col)" class="text-[12px] font-mono text-text-muted tabular-nums shrink-0" :class="columnAlignClass(col)">
        {{ yearText }}
      </div>

      <!-- 流派 -->
      <div v-else-if="col.id === 'genre'" :style="columnCellStyle(col)" class="min-w-0 text-[12px] truncate shrink-0" :class="[columnAlignClass(col), greyed ? 'text-text-disabled' : 'text-text-secondary']">
        <span class="truncate block" :title="genreText">{{ genreText }}</span>
      </div>

      <!-- 文件大小（当前首选音源） -->
      <div v-else-if="col.id === 'fileSize'" :style="columnCellStyle(col)" class="text-[12px] font-mono text-text-muted tabular-nums shrink-0" :class="columnAlignClass(col)">
        {{ fileSizeText }}
      </div>

      <!-- 播放时间（页面专属列，最近播放） -->
      <div v-else-if="col.id === 'playedAt'" :style="columnCellStyle(col)" class="text-[12px] font-mono text-text-muted tabular-nums shrink-0" :class="columnAlignClass(col)">
        {{ playedAtText }}
      </div>

      <!-- 添加到歌单 / 不可播状态 -->
      <div v-else-if="col.id === 'more' && showMore" :style="columnCellStyle(col)" class="flex items-center justify-center shrink-0">
        <CloudOff
          v-if="greyed && !batchMode"
          class="w-3.5 h-3.5 text-text-disabled"
          :title="greyTitle"
        />
        <button
          v-else-if="!batchMode"
          class="w-8 h-8 flex items-center justify-center rounded-[6px] text-text-muted opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-bg-hover transition-opacity"
          title="添加到歌单"
          aria-label="添加到歌单"
          @click.stop="playlistPickerOpen = true"
        ><ListPlus class="w-4 h-4" /></button>
      </div>

      <!-- 页面自定义列插槽 -->
      <div v-else :style="columnCellStyle(col)" class="min-w-0 shrink-0" :class="columnAlignClass(col)">
        <slot :name="`cell-${col.id}`" :track="track" :index="index" />
      </div>
    </template>

    <!-- 行尾占位：宽度与表头尾部控件（批量入口/显示列菜单）配对，保证列对齐 -->
    <div v-if="trailingWidth > 0" class="shrink-0" :style="{ width: trailingWidth + 'px' }" />
  </div>
  <PlaylistPickerModal
    v-if="playlistPickerOpen"
    :track-ids="[track.id]"
    :track-title="track.title"
    @close="playlistPickerOpen = false"
  />
</template>
