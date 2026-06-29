<script setup lang="ts">
import { computed } from 'vue';
import {
  Play, Shuffle, Loader2, Disc3, Heart, MoreHorizontal, Clock,
} from 'lucide-vue-next';
import { usePlayerStore, type Track } from '../../stores/player';
import { useArtworkSrc } from '../../composables/useArtworkSrc';

const props = defineProps<{
  albumId: number | null;
}>();

const playerStore = usePlayerStore();

const album = computed(() => playerStore.currentAlbumDetails);

/** 是否已收藏该专辑 */
const isAlbumFavorited = computed(() =>
  props.albumId !== null && playerStore.favoriteAlbums.some(a => a.id === props.albumId)
);

/** 切换专辑收藏 */
function toggleAlbumFav() {
  if (props.albumId !== null) {
    playerStore.toggleFavoriteAlbum(props.albumId, !isAlbumFavorited.value);
  }
}

/** 封面 */
const coverSrc = useArtworkSrc(() => album.value?.cover_artwork_id ?? null);

/** 轨道列表 */
const tracks = computed<Track[]>(() => album.value?.tracks ?? []);
const isLoadingTracks = computed(() => {
  // 刚设了 activeAlbumId 但 watcher 还没拉取完
  return props.albumId !== null && !album.value;
});

/** 元数据行文案 */
const trackCount = computed(() => tracks.value.length);
const totalDuration = computed(() => {
  const totalSec = tracks.value.reduce((sum, t) => sum + (t.durationSec || 0), 0);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
});
const metaText = computed(() => {
  if (!album.value) return '';
  const parts: string[] = [];
  if (album.value.year) parts.push(String(album.value.year));
  parts.push(`${trackCount.value} TRACKS`);
  parts.push(totalDuration.value);
  return parts.join(' · ');
});

/** 当前播放判定 */
function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

/** 双击播放单曲 */
function playTrack(index: number) {
  playerStore.playAll(tracks.value, index);
}

/** 播放全部 */
function playAll() {
  if (tracks.value.length > 0) playerStore.playAll(tracks.value, 0);
}

/** 随机播放 */
function shufflePlay() {
  if (tracks.value.length === 0) return;
  const idx = Math.floor(Math.random() * tracks.value.length);
  playerStore.playAll(tracks.value, idx);
}

/** 收藏 */
function toggleFav(trackId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavorite(trackId);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <!-- 无选中时占位 -->
    <div v-if="!albumId" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Disc3 class="w-10 h-10 text-text-disabled" />
      <p class="text-[13px]">选择一张专辑查看详情</p>
    </div>

    <!-- 加载中 -->
    <div v-else-if="isLoadingTracks" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载专辑…</span>
    </div>

    <template v-else-if="album">
      <!-- 专辑头部 -->
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <!-- 封面（~180px） -->
          <div class="w-[180px] h-[180px] rounded-[10px] overflow-hidden flex-shrink-0 bg-bg-hover flex items-center justify-center">
            <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
            <Disc3 v-else class="w-10 h-10 text-text-disabled" />
          </div>

          <!-- 标题 + 元数据 + 按钮 -->
          <div class="flex-1 min-w-0 pt-2">
            <div class="flex items-center gap-3 mb-1">
              <h1 class="text-[28px] font-bold text-text-primary tracking-tight leading-tight">{{ album.title }}</h1>
              <button
                class="flex-shrink-0 transition-colors-smooth"
                :class="isAlbumFavorited ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
                @click="toggleAlbumFav"
                :title="isAlbumFavorited ? '取消收藏' : '收藏专辑'"
              >
                <Heart class="w-[22px] h-[22px]" :class="isAlbumFavorited ? 'fill-current' : ''" />
              </button>
            </div>
            <p class="text-[14px] text-text-secondary mb-2">{{ album.artist }}</p>

            <!-- 元数据行（等宽 mono） -->
            <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mb-5">{{ metaText }}</p>

            <!-- 操作按钮 -->
            <div class="flex items-center gap-3">
              <button
                class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
                @click="playAll"
              >
                <Play class="w-[14px] h-[14px] fill-current" />
                播放全部
              </button>
              <button
                class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-primary flex items-center gap-2 hover:bg-list-hover transition-colors-smooth"
                @click="shufflePlay"
              >
                <Shuffle class="w-[14px] h-[14px]" />
                随机播放
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 分割线 -->
      <div class="h-px bg-border-color mx-8"></div>

      <!-- 轨道列表 -->
      <div class="flex-1 overflow-y-auto px-8">
        <!-- 表头 -->
        <div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
          <div class="w-10 text-center shrink-0">#</div>
          <div class="w-8 shrink-0"></div>
          <div class="flex-[2] min-w-0 pl-1">标题</div>
          <div class="flex-[1.5] min-w-0 hidden sm:block">艺术家</div>
          <div class="w-[56px] text-right shrink-0">
            <Clock class="w-[12px] h-[12px] inline-block" />
          </div>
          <div class="w-8 shrink-0"></div>
        </div>

        <!-- 空列表 -->
        <div v-if="tracks.length === 0" class="flex flex-col items-center justify-center py-16 gap-3 text-text-muted">
          <span class="text-[12px]">该专辑暂无曲目</span>
        </div>

        <!-- 轨道行 -->
        <div
          v-for="(track, index) in tracks"
          :key="track.id"
          class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative"
          style="height: 40px;"
          :class="{
            'playing-row bg-list-selected': isPlayingTrack(track.id),
          }"
          @dblclick="playTrack(index)"
        >
          <!-- 序号 / 播放图标 -->
          <div class="w-10 text-center shrink-0 text-[12px] font-mono">
            <span v-if="isPlayingTrack(track.id)" class="text-brand-orange inline-flex items-center justify-center">
              <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
              <Play v-else class="w-[12px] h-[12px] fill-current" />
            </span>
            <template v-else>
              <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
              <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
            </template>
          </div>

          <!-- 收藏 -->
          <div class="w-8 shrink-0 flex items-center justify-center">
            <Heart
              v-if="track.isFavorite"
              class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
            <Heart
              v-else
              class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
          </div>

          <!-- 标题 -->
          <div class="flex-[2] min-w-0 pl-1">
            <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
              {{ track.title }}
            </span>
          </div>

          <!-- 艺术家 -->
          <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate">{{ track.artist }}</div>

          <!-- 时长 -->
          <div class="w-[56px] text-right shrink-0 text-[12px] font-mono text-text-muted tabular-nums">{{ track.duration }}</div>

          <!-- more -->
          <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <MoreHorizontal class="w-4 h-4 text-text-muted" />
          </div>
        </div>

      </div>

      <!-- 底部统计（固定在底部） -->
      <div v-if="tracks.length > 0" class="flex-shrink-0 flex items-center justify-between px-8 py-4 border-t border-border-color text-[11px] text-text-muted font-mono bg-bg-content">
        <span>{{ trackCount }} 首曲目 · {{ totalDuration }}</span>
        <span>双击播放</span>
      </div>
    </template>
  </div>
</template>
