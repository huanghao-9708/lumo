<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { User, Star, Loader2 } from 'lucide-vue-next';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { useScrollRestore } from '../../composables/useScrollRestore';
import { getArtworkUrl } from '../../utils';
import { libraryFetchMissingArtistCover } from '../../api/library';
import FooterStatus from '../shared/FooterStatus.vue';

const playerStore = usePlayerStore();
const uiStore = useUiStore();

const artists = computed(() => playerStore.favoriteArtists);
const isLoading = computed(() => false);

/** 滚动位置记忆：从歌手详情返回时还原 */
const scrollContainer = useScrollRestore(() => 'favorite-artists');

/**
 * 补齐缺失头像：歌手头像是按需从 iTunes 拉的（扫描阶段只存专辑封面），
 * 之前只有进过详情页的歌手才有头像 —— 收藏页因此几乎全是占位图。
 * 这里为收藏的歌手（数量通常很少）补一次后台拉取，完成后由
 * `artist-cover-fetched` 事件回填 favoriteArtists。
 */
function backfillMissingCovers() {
  if (!uiStore.fetchCoversOnline) return;
  for (const artist of artists.value) {
    if (!artist.avatar_artwork_id) {
      libraryFetchMissingArtistCover(artist.id, true).catch(console.error);
    }
  }
}

onMounted(async () => {
  // 收藏列表可能还没加载（例如冷启动直接切到本页）
  if (artists.value.length === 0) await playerStore.fetchFavoriteArtists();
  backfillMissingCovers();
});

function selectArtist(artistId: number) {
  // 走 store 导航函数：同 tick 改 tab+id（只记一条历史），并让详情页子标签回到默认分栏
  playerStore.navigateToArtist(artistId);
}

function toggleFav(artistId: number, e: Event) {
  e.stopPropagation();
  playerStore.toggleFavoriteArtist(artistId, false);
}
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div ref="scrollContainer" class="flex-1 overflow-y-auto px-8">
      <!-- 加载态 -->
      <div v-if="isLoading && artists.length === 0" class="flex items-center justify-center py-20 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <!-- 空态 -->
      <div v-else-if="artists.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
        <Star class="w-8 h-8 text-text-disabled" />
        <span class="text-[12px]">还没有收藏的歌手</span>
        <p class="text-[11px] text-text-muted/70">在歌手页面点击星标即可收藏</p>
      </div>

      <!-- 网格 -->
      <div
        v-else
        class="grid gap-6 pb-6"
        style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));"
      >
        <div
          v-for="(artist, index) in artists"
          :key="artist.id"
          class="group cursor-pointer"
          @click="selectArtist(artist.id)"
        >
          <div
            class="relative w-full aspect-square mb-3 overflow-hidden flex items-center justify-center"
            :class="[
              index % 2 === 0 ? 'rounded-[10px]' : 'rounded-full',
              !artist.avatar_artwork_id ? `bg-gradient-to-br ${artist.avatarColor}` : 'bg-bg-hover',
            ]"
          >
            <!-- 有头像就显示真实图片（与歌手网格一致）；没有才退回渐变 + 人形占位 -->
            <img
              v-if="artist.avatar_artwork_id"
              :src="getArtworkUrl(artist.avatar_artwork_id)"
              class="w-full h-full object-cover"
              :alt="artist.name"
            />
            <div v-else class="w-full h-full flex items-center justify-center bg-black/10 group-hover:bg-black/20 transition-colors-smooth">
              <User class="w-[40px] h-[40px] text-white/60" />
            </div>

            <!-- 收藏中的歌手：星标常亮，hover 才可点（点掉即取消收藏并从本页移除） -->
            <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                class="w-7 h-7 rounded-full bg-black/40 backdrop-blur-sm flex items-center justify-center hover:bg-black/60 transition-colors-smooth"
                title="取消收藏"
                @click.stop="toggleFav(artist.id, $event)"
              >
                <Star class="w-[14px] h-[14px] text-white fill-current" />
              </button>
            </div>
          </div>

          <p class="text-[15px] text-text-primary font-medium truncate leading-tight mb-1">{{ artist.name }}</p>
          <p class="text-[13px] text-text-secondary truncate">{{ artist.trackCount }} 首歌曲</p>
        </div>
      </div>
    </div>

    <FooterStatus v-if="artists.length > 0" :count="`${artists.length.toLocaleString()} 位艺术家`" />
  </div>
</template>
