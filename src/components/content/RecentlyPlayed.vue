<script setup lang="ts">
import { computed } from 'vue';
import {
  Play, Loader2, Heart, MoreHorizontal, Music,
} from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();

const tracks = computed(() => playerStore.tracks);

function formatPlayedAt(playedAt: string | undefined): string {
  if (!playedAt) return '--';
  const d = new Date(playedAt);
  if (isNaN(d.getTime())) return '--';
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);
  const hhmm = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  if (d >= today) return `今日 ${hhmm}`;
  if (d >= yesterday) return `昨日 ${hhmm}`;
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

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
  playerStore.playTrack(index);
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

      <!-- 列表 -->
      <div v-else>
        <div
          v-for="(track, index) in tracks"
          :key="track.id"
          class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer"
          style="height: 40px;"
          :class="{ 'playing-row bg-list-selected': isPlayingTrack(track.id) }"
          @dblclick="playSong(index)"
        >
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

          <div class="w-8 shrink-0 flex items-center justify-center">
            <Heart
              v-if="track.isFavorite" class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
            <Heart
              v-else class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
              @click="toggleFav(track.id, $event)"
            />
          </div>

          <div class="flex-[2] min-w-0 pl-1">
            <span class="text-[13px] truncate block" :class="isPlayingTrack(track.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
              {{ track.title }}
            </span>
          </div>

          <div class="flex-[1.5] min-w-0 hidden sm:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(track.artistId) { playerStore.activeLibraryTab = '艺术家'; playerStore.activeArtistId = track.artistId; }">{{ track.artist }}</span></div>

          <div class="flex-[1.5] min-w-0 hidden md:block text-[13px] text-text-secondary truncate"><span class="hover:underline cursor-pointer" @click.stop="if(track.albumId) { playerStore.activeLibraryTab = '专辑'; playerStore.activeAlbumId = track.albumId; }">{{ track.album }}</span></div>

          <!-- 播放时间列 -->
          <div class="w-[80px] text-right shrink-0 text-[12px] font-mono text-text-muted tabular-nums">
            {{ formatPlayedAt(track.playedAt) }}
          </div>

          <div class="w-8 shrink-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <MoreHorizontal class="w-4 h-4 text-text-muted" />
          </div>
        </div>
      </div>

      <!-- Footer -->
    </div>

    <!-- Footer Status（固定在底部） -->
    <FooterStatus v-if="tracks.length > 0" :count="`${tracks.length.toLocaleString()} 首歌曲`" :hint="totalDurationText" />
  </div>
</template>
