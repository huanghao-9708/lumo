<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { 
  Search, TriangleAlert, Settings, X, 
  Play, Save, Check, Loader2, Sparkles, AlertCircle
} from 'lucide-vue-next';
import { useAiStore } from '../../stores/ai';
import { usePlayerStore, type Track } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { libraryGetTracks } from '../../api/library';
import { useScrollRestore } from '../../composables/useScrollRestore';
import MobileSongRow from './MobileSongRow.vue';
import ActionSheet, { type ActionItem } from './ActionSheet.vue';

const aiStore = useAiStore();
const playerStore = usePlayerStore();
const uiStore = useUiStore();

// 初始化时拉取设置
onMounted(() => {
  aiStore.fetchSettings();
});

// 滚动位置恢复
const scrollContainer = useScrollRestore(() => 'm-ai-radio');

// 模式切换
type Mode = 'recent' | 'seed' | 'phrase';
const currentMode = ref<Mode>('recent');
const modes = [
  { label: '最近播放', value: 'recent' as const },
  { label: '基于歌曲', value: 'seed' as const },
  { label: '一句话', value: 'phrase' as const },
];

// 种子模式状态
const seedQuery = ref('');
const searchResults = ref<Track[]>([]);
const selectedSeed = ref<Track | null>(null);
const isSearching = ref(false);
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const onSearchInput = () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  const q = seedQuery.value.trim();
  if (!q) {
    searchResults.value = [];
    return;
  }
  debounceTimer = setTimeout(async () => {
    isSearching.value = true;
    try {
      const dtos = await libraryGetTracks(20, 0, q);
      searchResults.value = dtos.map(d => playerStore.mapTrackDTO(d));
    } catch (e) {
      console.error('搜索出错:', e);
    } finally {
      isSearching.value = false;
    }
  }, 300);
};

const selectSeed = (track: Track) => {
  selectedSeed.value = track;
  seedQuery.value = '';
  searchResults.value = [];
};

const clearSeed = () => {
  selectedSeed.value = null;
};

// 一句话模式状态
const phraseText = ref('');

// 生成逻辑
const handleGenerate = () => {
  if (currentMode.value === 'seed' && !selectedSeed.value) {
    uiStore.showToast('请先选择一首种子歌曲', 'info');
    return;
  }
  if (currentMode.value === 'phrase' && !phraseText.value.trim()) {
    uiStore.showToast('请输入一句话描述', 'info');
    return;
  }

  aiStore.generate(currentMode.value, {
    phrase: currentMode.value === 'phrase' ? phraseText.value.trim() : undefined,
    seedTrackId: currentMode.value === 'seed' ? selectedSeed.value?.id : undefined,
  });
};

const cancelGenerate = () => {
  aiStore.cancelGenerate();
};

// 清除当前结果回到表单
const handleRegenerate = () => {
  aiStore.result = null;
  aiStore.generateError = null;
};

// 保存歌单
const handleSavePlaylist = async () => {
  const name = await aiStore.saveAsPlaylist();
  if (name) {
    uiStore.showToast(`已保存为歌单「${name}」`, 'info');
  }
};

// 播放全部
const playAll = () => {
  if (aiStore.result && aiStore.result.tracks.length > 0) {
    playerStore.playQueue(aiStore.result.tracks, 0);
  }
};

// 单曲播放
const onPlayTrack = (index: number) => {
  if (aiStore.result && aiStore.result.tracks.length > 0) {
    playerStore.playQueue(aiStore.result.tracks, index);
  }
};

// ActionSheet 逻辑
const showActionSheet = ref(false);
const actionTargetTrack = ref<Track | null>(null);

const actionItems = computed<ActionItem[]>(() => {
  const track = actionTargetTrack.value;
  if (!track) return [];
  return [
    {
      label: track.isFavorite ? '取消收藏' : '收藏',
      onClick: () => playerStore.toggleFavorite(track.id),
    },
    ...(track.albumId ? [{
      label: '查看专辑',
      onClick: () => {
        playerStore.activeAlbumId = track.albumId;
        playerStore.activeLibraryTab = '专辑';
      },
    }] : []),
    ...(track.artistId ? [{
      label: '查看艺术家',
      onClick: () => {
        playerStore.activeArtistId = track.artistId;
        playerStore.activeLibraryTab = '艺术家';
      },
    }] : []),
  ];
});

const onLongPress = (trackId: number) => {
  const found = aiStore.result?.tracks.find(t => t.id === trackId);
  if (found) {
    actionTargetTrack.value = found;
    showActionSheet.value = true;
  }
};

const closeActionSheet = () => {
  showActionSheet.value = false;
  actionTargetTrack.value = null;
};

// 导航到设置
const goToSettings = () => {
  playerStore.navigateToTab('设置');
};

// 判断是否是当前播放的歌曲
const isCurrentTrack = (id: number) => {
  return playerStore.currentTrack?.id === id;
};

const toggleFav = (trackId: number) => {
  playerStore.toggleFavorite(trackId);
};

onUnmounted(() => {
  if (debounceTimer) clearTimeout(debounceTimer);
});
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden">
    
    <!-- 未开启 AI 设置引导 -->
    <div v-if="aiStore.settings && !aiStore.settings.enabled" class="flex-1 flex flex-col items-center justify-center p-6 text-center">
      <Sparkles class="w-16 h-16 text-text-secondary mb-4 opacity-50" stroke-width="1.5" />
      <h2 class="text-[17px] font-bold text-text-primary mb-2">未开启 AI 功能</h2>
      <p class="text-[15px] text-text-secondary mb-8">
        AI 电台可以根据你的喜好生成智能歌单。请先前往设置页面配置 AI 大模型 API。
      </p>
      <button 
        class="h-11 px-8 rounded-full bg-brand-orange text-white text-[15px] font-medium active:opacity-80 transition-opacity flex items-center justify-center space-x-2"
        @click="goToSettings"
      >
        <Settings class="w-5 h-5" />
        <span>去设置</span>
      </button>
    </div>

    <!-- 已开启配置 -->
    <div 
      v-else
      ref="scrollContainer" 
      class="flex-1 overflow-y-auto px-4 py-4"
    >
      <!-- 生成错误提示 -->
      <div v-if="aiStore.generateError" class="mb-6 p-4 rounded-xl bg-red-500/10 border border-red-500/20">
        <div class="flex items-start space-x-3">
          <TriangleAlert class="w-5 h-5 text-red-500 mt-0.5 shrink-0" />
          <div class="flex-1">
            <h3 class="text-[15px] font-semibold text-red-500 mb-1">生成失败</h3>
            <p class="text-[13px] text-text-secondary break-all mb-3">{{ aiStore.generateError }}</p>
            <button 
              class="h-9 px-4 rounded-full bg-red-500 text-white text-[13px] font-medium active:opacity-80"
              @click="handleGenerate"
            >
              重试
            </button>
          </div>
        </div>
      </div>

      <!-- 生成中骨架屏 -->
      <div v-if="aiStore.isGenerating" class="flex flex-col space-y-6">
        <div class="flex flex-col items-center justify-center py-8">
          <Loader2 class="w-10 h-10 text-brand-orange animate-spin mb-4" />
          <p class="text-[15px] text-text-primary font-medium">正在生成你的专属电台...</p>
        </div>
        
        <div class="space-y-4">
          <div v-for="i in 8" :key="i" class="flex items-center space-x-3 px-1">
            <div class="w-11 h-11 rounded-md bg-white/5 skeleton-bg animate-pulse"></div>
            <div class="flex-1 flex flex-col space-y-2">
              <div class="h-4 w-3/4 rounded bg-white/5 skeleton-bg animate-pulse"></div>
              <div class="h-3 w-1/2 rounded bg-white/5 skeleton-bg animate-pulse"></div>
            </div>
          </div>
        </div>
        
        <div class="mt-6 flex justify-center">
          <button 
            class="h-11 px-8 rounded-full border border-border-color text-text-primary text-[15px] font-medium active:bg-list-hover"
            @click="cancelGenerate"
          >
            取消生成
          </button>
        </div>
      </div>

      <!-- 生成结果展示 -->
      <div v-else-if="aiStore.result" class="flex flex-col space-y-6">
        <!-- 歌单信息 -->
        <div class="text-center mb-2 px-2">
          <h1 class="text-[20px] font-bold text-text-primary mb-2">
            {{ aiStore.result.name || 'AI 电台' }}
          </h1>
          <p v-if="aiStore.result.description" class="text-[14px] text-text-secondary">
            {{ aiStore.result.description }}
          </p>
        </div>
        
        <!-- 降级提示 -->
        <div v-if="aiStore.result.source === 'fallback'" class="mx-2 p-3 rounded-lg bg-orange-500/10 border border-orange-500/20 flex items-start space-x-2">
          <AlertCircle class="w-4 h-4 text-orange-500 mt-0.5 shrink-0" />
          <p class="text-[13px] text-orange-500/90 leading-tight">
            {{ aiStore.result.degradedReason || '由于 AI 响应超时或异常，已为你随机推荐本地曲目' }}
          </p>
        </div>

        <!-- 歌曲列表 -->
        <div class="flex flex-col">
          <MobileSongRow
            v-for="(track, idx) in aiStore.result.tracks"
            :key="track.id + '-' + idx"
            :track="track"
            :index="idx"
            :is-playing="playerStore.isPlaying"
            :is-current="isCurrentTrack(track.id)"
            @play="onPlayTrack(idx)"
            @toggle-fav="toggleFav"
            @long-press="onLongPress"
          />
        </div>
        
        <!-- 底部操作区 -->
        <div class="flex flex-col space-y-3 mt-4 pt-4 pb-8">
          <button 
            class="w-full h-12 rounded-[24px] bg-brand-orange text-white text-[15px] font-medium active:opacity-80 flex items-center justify-center space-x-2"
            @click="playAll"
          >
            <Play class="w-5 h-5 fill-current" />
            <span>播放全部</span>
          </button>
          
          <button 
            class="w-full h-12 rounded-[24px] border border-border-color text-text-primary text-[15px] font-medium active:bg-list-hover flex items-center justify-center space-x-2 disabled:opacity-50"
            :disabled="aiStore.isSaving || !!aiStore.savedPlaylistName"
            @click="handleSavePlaylist"
          >
            <Check v-if="aiStore.savedPlaylistName" class="w-5 h-5 text-green-500" />
            <Loader2 v-else-if="aiStore.isSaving" class="w-5 h-5 animate-spin" />
            <Save v-else class="w-5 h-5" />
            <span>{{ aiStore.savedPlaylistName ? '已保存' : '保存为歌单' }}</span>
          </button>
          
          <button 
            class="w-full h-12 text-[14px] text-text-secondary active:text-text-primary"
            @click="handleRegenerate"
          >
            重新生成
          </button>
        </div>
      </div>

      <!-- 表单模式 -->
      <div v-else class="flex flex-col space-y-6">
        <!-- 模式切换 Tab -->
        <div class="flex bg-white/5 rounded-full p-1 h-11 items-center">
          <button 
            v-for="mode in modes" 
            :key="mode.value"
            class="flex-1 h-9 rounded-full text-[14px] font-medium transition-colors flex items-center justify-center"
            :class="currentMode === mode.value ? 'bg-white/15 text-text-primary shadow-sm' : 'text-text-secondary active:text-text-primary active:bg-white/5'"
            @click="currentMode = mode.value"
          >
            {{ mode.label }}
          </button>
        </div>

        <!-- 最近播放说明 -->
        <div v-if="currentMode === 'recent'" class="px-2 py-4">
          <p class="text-[15px] text-text-secondary text-center leading-relaxed">
            将基于你最近播放的 50 首曲目，由 AI 智能分析你的口味偏好，生成符合你当前心境的专属电台。
          </p>
        </div>

        <!-- 基于歌曲 (种子) -->
        <div v-else-if="currentMode === 'seed'" class="flex flex-col space-y-4">
          <div v-if="!selectedSeed" class="relative">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <Search class="w-5 h-5 text-text-muted" />
            </div>
            <input 
              v-model="seedQuery"
              type="text" 
              enterkeyhint="search"
              placeholder="搜索歌曲..." 
              class="w-full h-11 pl-10 pr-4 bg-white/5 border-none rounded-xl text-[15px] text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-brand-orange"
              @input="onSearchInput"
            >
            <div v-if="isSearching" class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
              <Loader2 class="w-4 h-4 text-brand-orange animate-spin" />
            </div>
            
            <!-- 搜索结果列表 -->
            <div v-if="searchResults.length > 0" class="mt-2 max-h-[300px] overflow-y-auto bg-bg-content rounded-xl border border-border-color shadow-lg divide-y divide-border-color">
              <div 
                v-for="track in searchResults" 
                :key="track.id"
                class="flex flex-col px-4 py-3 active:bg-list-hover"
                @click="selectSeed(track)"
              >
                <div class="text-[15px] text-text-primary truncate font-medium">{{ track.title }}</div>
                <div class="text-[13px] text-text-secondary truncate mt-0.5">{{ track.artist }}</div>
              </div>
            </div>
          </div>
          
          <div v-else class="p-4 bg-white/5 rounded-xl border border-border-color flex items-center space-x-3">
            <div class="flex-1 min-w-0">
              <div class="text-[13px] text-brand-orange mb-1">选中的种子歌曲</div>
              <div class="text-[15px] text-text-primary truncate font-medium">{{ selectedSeed.title }}</div>
              <div class="text-[13px] text-text-secondary truncate mt-0.5">{{ selectedSeed.artist }}</div>
            </div>
            <button 
              class="w-10 h-10 flex items-center justify-center rounded-full active:bg-white/10"
              @click="clearSeed"
            >
              <X class="w-5 h-5 text-text-secondary" />
            </button>
          </div>
        </div>

        <!-- 一句话描述 -->
        <div v-else-if="currentMode === 'phrase'" class="flex flex-col">
          <textarea 
            v-model="phraseText"
            rows="2"
            placeholder="用一句话描述你想听的歌单，例如：适合下雨天听的华语慢歌"
            class="w-full min-h-[44px] p-4 bg-white/5 border-none rounded-xl text-[15px] text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-brand-orange resize-none"
          ></textarea>
        </div>

        <!-- 生成按钮 -->
        <div class="pt-6">
          <button 
            class="w-full h-12 rounded-[24px] bg-brand-orange text-white text-[15px] font-bold active:opacity-80 transition-opacity flex items-center justify-center space-x-2 disabled:opacity-40"
            :disabled="(currentMode === 'seed' && !selectedSeed) || (currentMode === 'phrase' && !phraseText.trim())"
            @click="handleGenerate"
          >
            <Sparkles class="w-5 h-5" />
            <span>生成 AI 电台</span>
          </button>
        </div>
      </div>
      
    </div>

    <!-- ActionSheet 菜单 -->
    <ActionSheet 
      :visible="showActionSheet"
      :actions="actionItems"
      @close="closeActionSheet"
    />
  </div>
</template>

<style scoped>
/* 项目复用 skeleton-bg 样式 */
.skeleton-bg {
  background-color: var(--color-bg-skeleton, rgba(255, 255, 255, 0.05));
}
</style>
