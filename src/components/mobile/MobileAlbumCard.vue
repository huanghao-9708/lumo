<script setup lang="ts">
import { Play, Disc3 } from 'lucide-vue-next';
import type { Album } from '../../stores/player';
import { getArtworkUrl } from '../../utils';
import { libraryGetAlbumTracks } from '../../api/library';
import { usePlayerStore } from '../../stores/player';

/**
 * 移动端专辑卡片（2 列网格）。
 *
 * 与桌面 AlbumCard（5 列 + Hover 浮层）的关键差异：
 *   - 2 列网格（手机），gap-4
 *   - 播放按钮常驻可见（右上角小圆形），不依赖 Hover
 *   - 单击封面 → 进入详情
 *   - 单击播放按钮 → 直接播放整张专辑
 */

const props = defineProps<{
  album: Album;
}>();

const emit = defineEmits<{
  (e: 'select', album: Album): void;
}>();

const playerStore = usePlayerStore();

function getCoverSrc(album: Album): string {
  if (album.cover_thumb) return album.cover_thumb;
  if (album.cover_artwork_id) return getArtworkUrl(album.cover_artwork_id);
  return '';
}

function onSelect() {
  emit('select', props.album);
}

async function onPlayAlbum(e: Event) {
  e.stopPropagation();
  try {
    const result = await libraryGetAlbumTracks(props.album.id);
    const tracks = result.map(t => {
      const durationMs = t.duration_ms ?? 0;
      const sec = Math.floor(durationMs / 1000);
      return {
        id: t.id,
        title: t.title,
        artist: t.artist_name || '未知艺人',
        album: t.album_title || props.album.title,
        duration: `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`,
        durationSec: sec,
        format: t.format ? t.format.toUpperCase() : 'UNKNOWN',
        artistId: t.artist_id ?? null,
        albumId: t.album_id ?? null,
        coverColor: '',
        isFavorite: false,
        primary_file_id: t.media_file_id,
        cover_artwork_id: t.cover_artwork_id,
        fileSize: t.file_size ?? null,
        sourceKind: (t.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
      };
    });
    if (tracks.length > 0) {
      await playerStore.playAll(tracks, 0);
    }
  } catch (err) {
    console.error('Failed to play album:', err);
  }
}
</script>

<template>
  <div class="min-w-0 cursor-pointer" @click="onSelect">
    <!-- 封面 -->
    <div class="relative w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-2">
      <img
        v-if="getCoverSrc(album)"
        :src="getCoverSrc(album)"
        :alt="album.title"
        class="w-full h-full object-cover"
        loading="lazy"
      />
      <div v-else class="w-full h-full flex items-center justify-center">
        <Disc3 class="w-10 h-10 text-text-disabled" aria-hidden="true" />
      </div>

      <!-- 常驻播放按钮（右上角） -->
      <button
        class="absolute right-2 top-2 w-8 h-8 rounded-full bg-text-primary/70 backdrop-blur-sm text-bg-canvas flex items-center justify-center active:opacity-80 transition-opacity"
        :aria-label="`播放 ${album.title}`"
        @click="onPlayAlbum"
      >
        <Play class="w-[14px] h-[14px] fill-current ml-0.5" aria-hidden="true" />
      </button>
    </div>

    <!-- 标题 + 艺术家 -->
    <p
      class="font-medium text-text-primary truncate leading-tight"
      style="font-size: var(--text-15);"
      :class="playerStore.activeAlbumId === album.id ? 'text-brand-orange' : ''"
    >{{ album.title }}</p>
    <p
      class="text-text-muted truncate"
      style="font-size: var(--text-13);"
    >{{ album.artist }}<span v-if="album.year"> · {{ album.year }}</span></p>
  </div>
</template>
