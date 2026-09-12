<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  Sparkles, Settings, Loader2, Play, Save, Search, RefreshCw, Wand2, TriangleAlert,
} from 'lucide-vue-next';
import { useAiStore } from '../../stores/ai';
import { usePlayerStore } from '../../stores/player';
import { useUiStore } from '../../stores/ui';
import { libraryGetTracks } from '../../api/library';
import type { Track } from '../../stores/player';

const aiStore = useAiStore();
const playerStore = usePlayerStore();
const uiStore = useUiStore();

onMounted(() => {
  aiStore.fetchSettings();
});

/* ============ 模式 Tab ============ */
const mode = ref<'recent' | 'seed' | 'phrase'>('recent');
const tabs: { key: 'recent' | 'seed' | 'phrase'; label: string }[] = [
  { key: 'recent', label: '基于最近播放' },
  { key: 'seed', label: '基于这首歌' },
  { key: 'phrase', label: '一句话' },
];

/* ============ 一句话模式 ============ */
const phrase = ref('');

/* ============ 种子歌模式：搜索选歌 ============ */
const seedQuery = ref('');
const seedResults = ref<Track[]>([]);
const isSearching = ref(false);
const seedTrack = ref<Track | null>(null);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

function onSeedInput() {
  if (searchTimer) clearTimeout(searchTimer);
  const q = seedQuery.value.trim();
  if (!q) {
    seedResults.value = [];
    return;
  }
  searchTimer = setTimeout(async () => {
    isSearching.value = true;
    try {
      const dtos = await libraryGetTracks(20, 0, q);
      seedResults.value = dtos.map(d => ({
        id: d.id,
        title: d.title,
        artistId: d.artist_id || null,
        artist: d.artist_name || '未知艺人',
        albumId: d.album_id || null,
        album: d.album_title || '',
        duration: '',
        durationSec: Math.floor((d.duration_ms ?? 0) / 1000),
        format: '',
        coverColor: '',
        cover_artwork_id: d.cover_artwork_id,
        isFavorite: d.is_favorite,
        primary_file_id: d.media_file_id,
        fileSize: d.file_size ?? null,
        sourceKind: (d.source_kind === 'webdav' ? 'webdav' : 'local') as 'local' | 'webdav',
      }));
    } catch (e) {
      console.error('[ai] seed search failed:', e);
    } finally {
      isSearching.value = false;
    }
  }, 250);
}

function selectSeed(track: Track) {
  seedTrack.value = track;
  seedResults.value = [];
  seedQuery.value = track.title;
}

/* ============ 生成 / 取消 / 保存 ============ */
const canGenerate = computed(() => {
  if (aiStore.isGenerating) return false;
  if (mode.value === 'phrase') return phrase.value.trim().length > 0;
  if (mode.value === 'seed') return seedTrack.value !== null;
  return true;
});

function generate() {
  if (!canGenerate.value) return;
  aiStore.generate(mode.value, {
    phrase: mode.value === 'phrase' ? phrase.value.trim() : undefined,
    seedTrackId: mode.value === 'seed' ? seedTrack.value?.id : undefined,
  });
}

async function saveAsPlaylist() {
  const name = await aiStore.saveAsPlaylist();
  if (name) uiStore.showToast(`已保存为歌单「${name}」`);
}

function playAll() {
  const tracks = aiStore.result?.tracks;
  if (tracks && tracks.length > 0) playerStore.playQueue(tracks, 0);
}
</script>

<template>
  <div class="flex-1 flex flex-col bg-bg-content overflow-hidden select-none min-w-0">
    <!-- Header -->
    <div class="px-8 pt-6 pb-4 flex-shrink-0">
      <div class="flex items-end justify-between mb-1">
        <div class="flex items-center gap-3">
          <Sparkles class="w-6 h-6 text-brand-orange" />
          <h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none">AI 电台</h1>
        </div>
        <button
          class="h-8 px-3 rounded-[8px] text-[12px] text-text-muted hover:text-text-primary hover:bg-list-hover transition-colors-smooth flex items-center gap-1.5"
          @click="playerStore.navigateToTab('设置')"
        >
          <Settings class="w-3.5 h-3.5" />
          AI 设置
        </button>
      </div>
      <p class="text-[12px] text-text-muted font-mono">从你的曲库中生成一份应景歌单 · 请求只包含标题与艺人</p>
    </div>

    <div class="flex-1 overflow-y-auto px-8 pb-8">

      <!-- ===== 未开启引导 ===== -->
      <div
        v-if="aiStore.settings && !aiStore.settings.enabled"
        class="mt-6 max-w-[560px] mx-auto bg-bg-canvas border border-border-color rounded-[10px] px-6 py-8 text-center"
      >
        <Wand2 class="w-8 h-8 text-text-disabled mx-auto mb-4" />
        <p class="text-[14px] font-medium text-text-primary mb-2">AI 推荐尚未开启</p>
        <p class="text-[12px] text-text-muted leading-relaxed mb-5">
          在设置中开启后，填入 OpenAI 兼容的模型服务（本地 Ollama 填
          <span class="font-mono">http://localhost:11434/v1</span> 即可全程离线）。
          请求只会发送歌曲标题与艺人名。
        </p>
        <button
          class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium hover:opacity-90 transition-opacity"
          @click="playerStore.navigateToTab('设置')"
        >去设置</button>
      </div>

      <template v-else>
        <!-- ===== 输入区 ===== -->
        <div class="max-w-[640px] mx-auto mt-2">
          <!-- 模式 Tab -->
          <div class="flex items-center gap-0 bg-bg-canvas border border-border-color rounded-[8px] p-[2px] w-fit mb-4">
            <button
              v-for="t in tabs"
              :key="t.key"
              class="h-7 px-3.5 rounded-[6px] text-[12px] transition-colors-smooth"
              :class="mode === t.key ? 'bg-list-selected text-text-primary font-medium' : 'text-text-muted hover:text-text-primary'"
              @click="mode = t.key"
            >{{ t.label }}</button>
          </div>

          <!-- recent -->
          <div v-if="mode === 'recent'" class="text-[13px] text-text-secondary leading-relaxed mb-4">
            根据你最近的收听记录、收藏与高频曲目，生成一份既有熟悉感又有新意的歌单。
          </div>

          <!-- seed -->
          <div v-else-if="mode === 'seed'" class="mb-4">
            <div class="relative">
              <Search class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                v-model="seedQuery"
                @input="onSeedInput"
                type="text"
                placeholder="搜索一首种子歌曲…"
                class="w-full h-[36px] pl-8 pr-3 text-[13px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50"
              />
              <Loader2 v-if="isSearching" class="w-3.5 h-3.5 animate-spin text-text-muted absolute right-3 top-1/2 -translate-y-1/2" />
            </div>
            <div v-if="seedTrack" class="mt-2 px-3 py-2 bg-bg-canvas border border-border-color rounded-[8px] text-[12px] text-text-secondary">
              种子：<span class="text-text-primary font-medium">{{ seedTrack.title }}</span> — {{ seedTrack.artist }}
            </div>
            <div v-if="seedResults.length > 0" class="mt-2 border border-border-color rounded-[8px] overflow-hidden">
              <button
                v-for="t in seedResults"
                :key="t.id"
                class="w-full text-left px-3 py-2 hover:bg-list-hover transition-colors-smooth flex items-center justify-between gap-3"
                @click="selectSeed(t)"
              >
                <span class="text-[13px] text-text-primary truncate">{{ t.title }}</span>
                <span class="text-[11px] text-text-muted truncate flex-shrink-0">{{ t.artist }}</span>
              </button>
            </div>
          </div>

          <!-- phrase -->
          <div v-else class="mb-4">
            <textarea
              v-model="phrase"
              rows="2"
              placeholder="用一句话描述你想听的歌单，例如：下雨天写代码时的安静钢琴曲"
              class="w-full px-3 py-2.5 text-[13px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted focus:border-brand-orange/50 resize-none leading-relaxed"
              @keydown.enter.exact.prevent="generate"
            ></textarea>
          </div>

          <!-- 生成按钮 -->
          <button
            class="h-[36px] px-5 rounded-full bg-brand-orange text-white text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity disabled:opacity-40 disabled:cursor-default"
            :disabled="!canGenerate"
            @click="generate"
          >
            <Sparkles class="w-4 h-4" />
            生成歌单
          </button>
        </div>

        <!-- ===== 生成中 ===== -->
        <div v-if="aiStore.isGenerating" class="max-w-[640px] mx-auto mt-8">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2 text-[12px] text-text-muted">
              <Loader2 class="w-3.5 h-3.5 animate-spin text-brand-orange" />
              正在从曲库中挑选…
            </div>
            <button
              class="h-7 px-3 rounded-[6px] text-[12px] text-text-muted hover:text-text-primary hover:bg-list-hover transition-colors-smooth"
              @click="aiStore.cancelGenerate()"
            >取消</button>
          </div>
          <div v-for="i in 8" :key="i" class="flex items-center py-2.5" :style="{ height: '44px' }">
            <div class="w-8 h-3 rounded-[3px] skeleton-bg mr-4"></div>
            <div class="flex-1">
              <div class="w-[45%] h-3 rounded-[3px] skeleton-bg mb-1.5"></div>
              <div class="w-[25%] h-2.5 rounded-[3px] skeleton-bg"></div>
            </div>
          </div>
        </div>

        <!-- ===== 错误 ===== -->
        <div v-else-if="aiStore.generateError" class="max-w-[640px] mx-auto mt-8">
          <div class="flex items-start gap-3 px-4 py-3.5 bg-bg-canvas border border-status-error/30 rounded-[8px]">
            <TriangleAlert class="w-4 h-4 text-status-error flex-shrink-0 mt-0.5" />
            <div class="flex-1 min-w-0">
              <p class="text-[13px] text-text-primary">{{ aiStore.generateError }}</p>
              <button class="text-[12px] text-text-muted hover:text-brand-orange transition-colors-smooth mt-1" @click="generate">
                重试
              </button>
            </div>
          </div>
        </div>

        <!-- ===== 结果 ===== -->
        <div v-else-if="aiStore.result" class="max-w-[640px] mx-auto mt-8">
          <!-- 降级提示 -->
          <div
            v-if="aiStore.result.source === 'fallback'"
            class="flex items-center gap-2 px-3 py-2 mb-3 bg-bg-canvas border border-border-color rounded-[8px] text-[12px] text-text-muted"
          >
            <TriangleAlert class="w-3.5 h-3.5 flex-shrink-0" />
            {{ aiStore.result.degradedReason || 'AI 暂不可用，已按规则生成' }}
          </div>

          <div class="flex items-end justify-between mb-1">
            <h2 class="text-[22px] font-bold text-text-primary tracking-tight">{{ aiStore.result.name }}</h2>
            <button
              class="text-[12px] text-text-muted hover:text-text-primary transition-colors-smooth flex items-center gap-1"
              :disabled="aiStore.isGenerating"
              @click="generate"
            >
              <RefreshCw class="w-3 h-3" />
              重新生成
            </button>
          </div>
          <p class="text-[12px] text-text-muted mb-4">{{ aiStore.result.description }}</p>

          <!-- 曲目列表 -->
          <div class="border-t border-border-color">
            <div
              v-for="(track, index) in aiStore.result.tracks"
              :key="track.id"
              class="flex items-center py-2 border-b border-border-color/60 group cursor-pointer"
              style="min-height: 48px;"
              @dblclick="playerStore.playQueue(aiStore.result!.tracks, index)"
            >
              <span class="w-7 text-[12px] font-mono tabular-nums flex-shrink-0" :class="index < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ String(index + 1).padStart(2, '0') }}</span>
              <div class="flex-1 min-w-0">
                <p class="text-[13px] text-text-primary truncate leading-tight">{{ track.title }}</p>
                <p class="text-[11px] text-text-muted truncate mt-0.5">
                  {{ track.artist }}<span v-if="track.reason" class="text-text-muted/80"> · {{ track.reason }}</span>
                </p>
              </div>
              <Play
                class="w-4 h-4 text-text-disabled opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0 ml-3"
                @click.stop="playerStore.playQueue(aiStore.result!.tracks, index)"
              />
            </div>
          </div>

          <!-- 操作 -->
          <div class="flex items-center gap-3 mt-5">
            <button
              class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
              @click="playAll"
            >
              <Play class="w-[14px] h-[14px] fill-current" />
              播放全部
            </button>
            <button
              class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-primary flex items-center gap-2 hover:bg-list-hover transition-colors-smooth disabled:opacity-40"
              :disabled="aiStore.isSaving"
              @click="saveAsPlaylist"
            >
              <Loader2 v-if="aiStore.isSaving" class="w-[14px] h-[14px] animate-spin" />
            <Save v-else class="w-[14px] h-[14px]" />
            保存为歌单
            </button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
