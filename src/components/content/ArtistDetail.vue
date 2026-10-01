<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Play, Shuffle, User, Loader2, Disc3, Star, Search } from 'lucide-vue-next';
import { usePlayerStore, type Album, type Track } from '../../stores/player';
import { useDesktopModeStore } from '../../stores/desktopMode';
import { getArtworkUrl } from '../../utils';
import { useScrollRestore } from '../../composables/useScrollRestore';
import { useBatchSelect } from '../../composables/useBatchSelect';
import BatchActionBar from '../shared/BatchActionBar.vue';
import TrackListHeader from '../shared/trackList/TrackListHeader.vue';
import TrackRow from '../shared/trackList/TrackRow.vue';
import { useTrackColumns } from '../shared/trackList/useTrackColumns';
import type { TrackListContext } from '../shared/trackList/columns';

const props = defineProps<{
  artistId: number | null;
}>();

const playerStore = usePlayerStore();
// 极简模式：头像不挂载（DM-04）
const desktopMode = useDesktopModeStore();
const visualAllowed = computed(() => desktopMode.visualAllowed);

const detail = computed(() => playerStore.currentArtistDetails);

/**
 * 子标签读写都落在 store 的详情对象上（而不是组件局部 ref）：
 * 进入专辑详情再返回时组件会重建，局部状态会丢，store 上的状态才能保持原分栏。
 */
const activeSubTab = computed<'tracks' | 'albums'>({
  get: () => detail.value?.subTab ?? 'tracks',
  set: (tab) => playerStore.setArtistDetailSubTab(tab),
});

/** 列表滚动位置记忆：切子标签 / 进详情返回都各记一份，key 里带艺人 id 避免串位 */
const listScrollEl = useScrollRestore(() => `artist-detail:${props.artistId ?? 0}:${activeSubTab.value}`);

/** 是否已收藏该歌手 */
const isArtistFavorited = computed(() =>
  props.artistId !== null && playerStore.favoriteArtists.some(a => a.id === props.artistId)
);

/** 切换歌手收藏 */
function toggleArtistFav() {
  if (props.artistId !== null) {
    playerStore.toggleFavoriteArtist(props.artistId, !isArtistFavorited.value);
  }
}

const albumGrid = computed<Album[]>(() => {
  const d = detail.value;
  if (!d || !d.albums) return [];
  return d.albums.map((a: Album) => ({
    id: a.id,
    title: a.title,
    artist: a.artist,
    year: a.year || 0,
    coverColor: a.coverColor || 'from-warm-500 to-warm-700',
    cover_artwork_id: a.cover_artwork_id ?? null,
    cover_thumb: a.cover_thumb || null,
    artist_name: a.artist_name,
    track_count: a.track_count,
  }));
});

// 详情页搜索只属于当前艺术家，不继承列表页筛选词。
const filterQuery = ref('');
watch(() => props.artistId, () => { filterQuery.value = ''; });
const filterActive = computed(() => !!filterQuery.value.trim());
const filterKeyword = computed(() => filterQuery.value.trim().toLowerCase());
const batch = useBatchSelect();
watch([() => props.artistId, activeSubTab, filterQuery], () => batch.exit());

const visibleTracks = computed<Track[]>(() => {
  const list = detail.value?.tracks ?? [];
  const q = filterKeyword.value;
  if (!q) return list;
  return list.filter(t =>
    t.title.toLowerCase().includes(q) ||
    (t.album || '').toLowerCase().includes(q) ||
    (t.artist || '').toLowerCase().includes(q)
  );
});

const isAllSelected = computed(() => visibleTracks.value.length > 0 && visibleTracks.value.every(track => batch.isSelected(track.id)));
function onToggleSelectAll() {
  if (isAllSelected.value) batch.selectNone();
  else batch.selectAll(visibleTracks.value);
}

// 过滤激活时自动把分页剩余拉完，保证过滤覆盖该艺术家的全部歌曲。
// 需等详情切到当前艺术家且首屏加载完成（组件创建时 detail 可能还是旧艺术家/加载中）。
const isLoadingAllForFilter = ref(false);
async function loadRemainingForFilter() {
  const id = props.artistId;
  if (!id || !filterActive.value || isLoadingAllForFilter.value) return;
  isLoadingAllForFilter.value = true;
  try {
    // 等待：详情已切到当前艺术家 && 首屏加载完成
    let guard = 0;
    while (guard++ < 300) {
      const d = detail.value;
      if (d && d.id === id && !d.isLoadingTracks) break;
      await new Promise(r => setTimeout(r, 100));
    }
    // 逐页追加直到拉全
    guard = 0;
    while (detail.value?.id === id && detail.value?.hasMoreTracks && guard++ < 300) {
      await playerStore.fetchArtistTracks(id, true);
    }
  } finally {
    isLoadingAllForFilter.value = false;
  }
}
watch(
  [filterActive, () => detail.value?.hasMoreTracks, () => detail.value?.isLoadingTracks],
  () => {
    if (filterActive.value) loadRemainingForFilter();
  },
  { immediate: true },
);

/** 播放用列表：过滤激活时播过滤结果，否则播全部 */
function currentList(): Track[] {
  return filterActive.value ? visibleTracks.value : (detail.value?.tracks ?? []);
}

/* ============ 统一列解析：艺人详情隐藏艺术家列（本页即上下文） ============ */
const listContext = computed<TrackListContext>(() => ({ hidden: ['artist'], batchEntry: true }));
const { resolvedColumns, menuColumns, trailingExtraWidth } = useTrackColumns({
  containerRef: listScrollEl,
  context: listContext,
});

function playAll() {
  const list = currentList();
  if (list.length > 0) playerStore.playAll(list, 0);
}

function shufflePlay() {
  const list = currentList();
  if (list.length > 0) {
    const idx = Math.floor(Math.random() * list.length);
    playerStore.playAll(list, idx);
  }
}

function isPlayingTrack(trackId: number): boolean {
  const t = playerStore.currentTrack;
  return !!t && t.id === trackId;
}

function playTrack(index: number) {
  const list = currentList();
  playerStore.playAll(list, index);
}

function selectAlbum(albumId: number) {
  playerStore.activeLibraryTab = '专辑';
  playerStore.activeAlbumId = albumId;
}

function getColorClass(color: string): string {
  return color || 'from-warm-500 to-warm-700';
}

// ===== 滚动加载更多（过滤未激活时按 30 条/页追加）=====
function onScroll(e: Event) {
  const el = e.target as HTMLElement;
  if (el.scrollTop + el.clientHeight < el.scrollHeight - 200) return;
  if (activeSubTab.value === 'tracks') {
    const d = detail.value;
    if (filterActive.value || !d?.hasMoreTracks || d.isLoadingTracks) return;
    if (props.artistId !== null) {
      playerStore.fetchArtistTracks(props.artistId, true);
    }
  }
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">

    <div v-if="!artistId" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <User class="w-10 h-10 text-text-disabled" />
      <p class="text-[13px]">选择一位艺术家查看详情</p>
    </div>

    <div v-else-if="!detail" class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
      <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
      <span class="text-[12px]">加载艺术家…</span>
    </div>

    <template v-else-if="detail">
      <div class="px-8 pt-8 pb-4 flex-shrink-0">
        <div class="flex items-start gap-8">
          <div
            v-if="visualAllowed"
            class="w-[152px] h-[152px] rounded-[10px] overflow-hidden flex-shrink-0 flex items-center justify-center relative"
            :class="!detail.avatar_artwork_id ? `bg-gradient-to-br ${getColorClass(detail.avatarColor)}` : ''"
          >
            <img 
              v-if="detail.avatar_artwork_id"
              :src="getArtworkUrl(detail.avatar_artwork_id)"
              class="w-full h-full object-cover"
            />
            <User v-else class="w-[48px] h-[48px] text-white/60" />
          </div>

          <div class="flex-1 min-w-0 pt-2">
            <div class="flex items-center gap-3 mb-1">
              <h1 class="text-(--text-page-title) font-bold text-text-primary tracking-tight leading-tight">{{ detail.name }}</h1>
              <button
                class="flex-shrink-0 transition-colors-smooth"
                :class="isArtistFavorited ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'"
                @click="toggleArtistFav"
                :title="isArtistFavorited ? '取消收藏' : '收藏歌手'"
              >
                <Star class="w-[22px] h-[22px]" :class="isArtistFavorited ? 'fill-current' : ''" />
              </button>
            </div>

            <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mb-5">
              {{ detail.stats?.track_count ?? 0 }} TRACKS · {{ detail.stats?.album_count ?? 0 }} ALBUMS
            </p>

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

      <div class="h-px bg-border-color mx-8"></div>

      <div class="flex items-center gap-8 px-8 pt-4 pb-0 flex-shrink-0">
        <button
          class="text-[13px] pb-2 border-b-2 transition-colors-smooth"
          :class="activeSubTab === 'tracks' ? 'text-text-primary border-brand-orange font-medium' : 'text-text-muted border-transparent hover:text-text-primary'"
          @click="activeSubTab = 'tracks'"
        >全部歌曲</button>
        <button
          class="text-[13px] pb-2 border-b-2 transition-colors-smooth"
          :class="activeSubTab === 'albums' ? 'text-text-primary border-brand-orange font-medium' : 'text-text-muted border-transparent hover:text-text-primary'"
          @click="activeSubTab = 'albums'"
        >全部专辑</button>
        <div v-if="activeSubTab === 'tracks'" class="relative ml-auto mb-2 w-[240px] max-w-[45%]">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted pointer-events-none" />
          <input v-model="filterQuery" aria-label="搜索艺术家歌曲" placeholder="搜索该艺术家的歌曲…" class="w-full h-8 pl-8 pr-3 rounded-[8px] bg-bg-canvas border border-border-color text-[12px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50" />
        </div>
      </div>

      <div class="h-px bg-border-color mx-8"></div>

      <div ref="listScrollEl" class="flex-1 overflow-y-auto px-8" @scroll="onScroll">

        <template v-if="activeSubTab === 'tracks'">
          <!-- 统一表头（右端含显示列菜单） -->
          <TrackListHeader :columns="resolvedColumns" :menu-columns="menuColumns" show-batch-entry :batch-active="batch.isActive" @toggle-batch="batch.isActive ? batch.exit() : batch.enter()" />

          <div v-if="detail.isLoadingTracks && (!detail.tracks || detail.tracks.length === 0)" class="flex items-center justify-center py-16">
            <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
          </div>

          <div v-else-if="!detail.tracks || detail.tracks.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <span class="text-[12px]">暂无歌曲</span>
          </div>

          <template v-else-if="visibleTracks.length === 0">
            <div class="flex flex-col items-center justify-center py-16 text-text-muted">
              <span class="text-[12px]">没有匹配的歌曲</span>
              <span v-if="detail.hasMoreTracks" class="text-[11px] text-text-disabled mt-1">正在加载更多以匹配…</span>
            </div>
          </template>

          <div v-else>
            <TrackRow
              v-for="(track, index) in visibleTracks"
              :key="track.id"
              :track="track"
              :columns="resolvedColumns"
              :index="index"
              :playing="isPlayingTrack(track.id)"
              :is-playing-now="playerStore.isPlaying"
              :batch-mode="batch.isActive"
              :selected="batch.isSelected(track.id)"
              :trailing-width="trailingExtraWidth"
              @play="playTrack(index)"
              @toggle-select="batch.toggle(track)"
            />

            <!-- 分页追加加载指示 -->
            <div v-if="detail.isLoadingTracks || isLoadingAllForFilter" class="flex items-center justify-center py-4 text-text-muted">
              <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" />
              <span class="text-[11px]">加载更多…</span>
            </div>
          </div>
        </template>

        <template v-if="activeSubTab === 'albums'">
          <div
            v-if="visualAllowed"
            class="grid gap-6 pt-4 pb-4"
            style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))"
          >
            <div
              v-for="album in albumGrid"
              :key="album.id"
              class="group cursor-pointer"
              @click="selectAlbum(album.id)"
            >
              <div
                class="relative w-full aspect-square rounded-[10px] mb-3 overflow-hidden flex-shrink-0 bg-gradient-to-br flex items-center justify-center"
                :class="getColorClass(album.coverColor)"
              >
                <img v-if="album.cover_thumb" :src="album.cover_thumb" class="w-full h-full object-cover" :alt="album.title" />
                <Disc3 v-else class="w-10 h-10 text-white/60" />
                <div class="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors-smooth rounded-[10px] pointer-events-none"></div>
              </div>

              <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ album.title }}</p>
              <p class="text-[13px] text-text-secondary truncate">{{ album.track_count ?? 0 }} 首歌曲</p>
            </div>
          </div>

          <div v-else data-artist-minimal-albums class="py-4 space-y-1">
            <button
              v-for="album in albumGrid"
              :key="album.id"
              class="w-full min-h-11 px-3 py-2 flex items-center gap-4 rounded-[8px] text-left hover:bg-list-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-orange/40"
              @click="selectAlbum(album.id)"
            >
              <span class="flex-1 min-w-0 text-[14px] text-text-primary truncate">{{ album.title }}</span>
              <span v-if="album.year" class="text-[12px] text-text-muted font-mono">{{ album.year }}</span>
              <span class="text-[12px] text-text-secondary whitespace-nowrap">{{ album.track_count ?? 0 }} 首歌曲</span>
            </button>
          </div>

          <div v-if="detail.isLoadingAlbums" role="status" class="py-4 text-center text-[12px] text-text-muted">加载专辑…</div>
          <div v-if="!detail.isLoadingAlbums && albumGrid.length === 0" class="flex flex-col items-center justify-center py-16 text-text-muted">
            <span class="text-[12px]">暂无专辑</span>
          </div>

          <!-- 专辑分页（页式替换分页，store 已有翻页函数） -->
          <div
            v-if="(detail.albumsTotalPages ?? 1) > 1"
            class="flex items-center justify-center gap-4 pb-4 text-[12px] text-text-secondary"
          >
            <button
              class="px-3 py-1.5 rounded-[6px] transition-colors-smooth disabled:opacity-40"
              :class="(detail.albumsCurrentPage ?? 1) <= 1 ? '' : 'hover:bg-list-hover hover:text-text-primary'"
              :disabled="detail.isLoadingAlbums || (detail.albumsCurrentPage ?? 1) <= 1"
              @click="playerStore.prevArtistAlbumsPage()"
            >上一页</button>
            <span class="font-mono tabular-nums">{{ detail.albumsCurrentPage ?? 1 }} / {{ detail.albumsTotalPages ?? 1 }}</span>
            <button
              class="px-3 py-1.5 rounded-[6px] transition-colors-smooth disabled:opacity-40"
              :class="(detail.albumsCurrentPage ?? 1) >= (detail.albumsTotalPages ?? 1) ? '' : 'hover:bg-list-hover hover:text-text-primary'"
              :disabled="detail.isLoadingAlbums || (detail.albumsCurrentPage ?? 1) >= (detail.albumsTotalPages ?? 1)"
              @click="playerStore.nextArtistAlbumsPage()"
            >下一页</button>
          </div>
        </template>
      </div>
      <BatchActionBar v-if="batch.isActive && activeSubTab === 'tracks'" :selected-ids="[...batch.selectedIds]" :all-selected="isAllSelected" @exit="batch.exit()" @toggle-select-all="onToggleSelectAll" />
    </template>
  </div>
</template>
