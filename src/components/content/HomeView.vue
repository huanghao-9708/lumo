<script setup lang="ts">
import { computed, onMounted } from 'vue';
import {
  Clock, Play, Disc, Heart, Flame, User, History, CalendarPlus,
  ChevronRight, Loader2, Music, Sparkles,
} from 'lucide-vue-next';
import { usePlayerStore, type RankedTrack } from '../../stores/player';
import { formatRelativeTime } from '../../utils/datetime';

const playerStore = usePlayerStore();

const stats = computed(() => playerStore.stats);
const insights = computed(() => playerStore.insights);
const isLoading = computed(() => playerStore.isLoadingStats || playerStore.isLoadingInsights);

onMounted(() => {
  playerStore.fetchStats();
  playerStore.fetchInsights();
});

/* ============ 数值格式化 ============ */

/** 毫秒 → 「X 小时 Y 分」（0 分钟时显示「<1 分钟」） */
function formatListenMs(ms: number | undefined): string {
  if (ms === undefined) return '--';
  const totalMin = Math.floor(ms / 60000);
  if (totalMin < 1) return '0 分钟';
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  return h > 0 ? `${h} 小时 ${m} 分` : `${m} 分钟`;
}

/** 今日听歌副信息：「今日 X 小时 Y 分」 */
function formatTodayListen(ms: number | undefined): string {
  if (ms === undefined) return '--';
  return `今日 ${formatListenMs(ms)}`;
}

function formatCount(n: number | undefined): string {
  return n === undefined ? '--' : n.toLocaleString();
}

/* ============ 统计卡（两排 8 张） ============ */

interface StatCard {
  icon: typeof Clock;
  label: string;
  value: string;
  sub: string;
}

const statCards = computed<StatCard[]>(() => {
  const s = stats.value;
  const ins = insights.value;
  const topTrack = ins?.topPlayedTracks[0];
  const topArtist = ins?.topPlayedArtists[0];
  const topAlbum = ins?.topPlayedAlbums[0];
  const lastPlayed = ins?.lastPlayed;
  return [
    {
      icon: Clock,
      label: '累计听歌时长',
      value: formatListenMs(s?.total_listen_ms),
      sub: formatTodayListen(s?.today_listen_ms),
    },
    {
      icon: Play,
      label: '累计听歌次数',
      value: formatCount(s?.total_play_count),
      sub: `今日 ${formatCount(s?.today_play_count)} 次`,
    },
    {
      icon: Disc,
      label: '曲库歌曲',
      value: formatCount(s?.track_count),
      sub: `${formatCount(s?.album_count)} 张专辑 · ${formatCount(s?.artist_count)} 位艺人`,
    },
    {
      icon: Heart,
      label: '我喜欢的音乐',
      value: formatCount(s?.favorite_track_count),
      sub: `${formatCount(s?.playlist_count)} 个歌单 · 收藏 ${formatCount(s?.favorite_album_count)} 张专辑`,
    },
    {
      icon: Flame,
      label: '播放最多的歌曲',
      value: topTrack?.title || '暂无播放记录',
      sub: topTrack ? `${topTrack.artist} · 播放 ${topTrack.playCount} 次` : '--',
    },
    {
      icon: User,
      label: '听得最多的歌手',
      value: topArtist?.name || '暂无播放记录',
      sub: topArtist ? `累计 ${topArtist.playCount} 次 · ${topArtist.trackCount} 首歌` : '--',
    },
    {
      icon: Disc,
      label: '播放最多的专辑',
      value: topAlbum?.title || '暂无播放记录',
      sub: topAlbum ? `${topAlbum.artist} · ${topAlbum.playCount} 次` : '--',
    },
    {
      icon: History,
      label: '上次听歌',
      value: lastPlayed ? formatRelativeTime(lastPlayed.playedAt) : '暂无播放记录',
      sub: lastPlayed ? `${lastPlayed.title} · ${lastPlayed.artist}` : '--',
    },
  ];
});

/* ============ 排行榜（6 张 Top5 卡，两列） ============ */

function playFromList(list: RankedTrack[], idx: number) {
  if (!list[idx]) return;
  playerStore.playQueue(list, idx);
}

const topPlayed = computed(() => insights.value?.topPlayedTracks ?? []);
const topArtists = computed(() => insights.value?.topPlayedArtists ?? []);
const topAlbums = computed(() => insights.value?.topPlayedAlbums ?? []);
const recentPlayed = computed(() => insights.value?.recentPlayedTracks ?? []);
const recentAdded = computed(() => insights.value?.recentAddedTracks ?? []);
const favoriteList = computed(() => insights.value?.favoriteTracks ?? []);
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="flex-1 overflow-y-auto px-4 md:px-8 py-4 md:py-6">

      <!-- Header -->
      <div class="mb-4 md:mb-6">
        <h1 class="text-[24px] md:text-[32px] font-bold text-text-primary tracking-tight leading-none mb-2">首页</h1>
        <p class="text-[12px] text-text-muted leading-relaxed font-mono">你的曲库与收听行为概览</p>
      </div>

      <!-- 加载指示（首次进入，卡片显示占位符的同时给出轻量反馈） -->
      <div v-if="isLoading && !stats && !insights" class="flex items-center justify-center py-24 text-text-muted">
        <Loader2 class="w-4 h-4 animate-spin text-brand-orange" />
      </div>

      <template v-else>
        <!-- ===== AI 电台入口（PRD-AI推荐歌单） ===== -->
        <button
          class="w-full mb-6 md:mb-8 bg-gradient-to-r from-brand-orange/12 to-transparent border border-brand-orange/30 rounded-[10px] px-4 py-3.5 flex items-center gap-3 hover:border-brand-orange/60 transition-colors-smooth text-left"
          @click="playerStore.navigateToTab('AI 电台')"
        >
          <Sparkles class="w-[18px] h-[18px] text-brand-orange flex-shrink-0" />
          <div class="flex-1 min-w-0">
            <p class="text-[13px] font-semibold text-text-primary">AI 电台</p>
            <p class="text-[11px] text-text-muted truncate">基于你的曲库与收听口味，生成一份应景歌单</p>
          </div>
          <ChevronRight class="w-4 h-4 text-text-muted flex-shrink-0" />
        </button>

        <!-- ===== 统计卡：两排 8 张（移动端 2 列） ===== -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4 mb-6 md:mb-8">
          <div
            v-for="card in statCards"
            :key="card.label"
            class="bg-bg-content border border-border-color rounded-[10px] px-3.5 md:px-4 py-3.5 md:py-4 min-w-0"
          >
            <div class="flex items-center gap-2 mb-3">
              <component :is="card.icon" class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
              <span class="text-[11px] text-text-muted truncate">{{ card.label }}</span>
            </div>
            <p class="text-[17px] font-bold text-text-primary leading-tight truncate" :title="card.value">{{ card.value }}</p>
            <p class="text-[11px] text-text-muted mt-1.5 truncate" :title="card.sub">{{ card.sub }}</p>
          </div>
        </div>

        <!-- ===== 快速访问：6 张 Top5 排行榜卡（移动端单列） ===== -->
        <h2 class="px-1 text-[10px] font-semibold text-text-muted mb-3 uppercase tracking-widest">快速访问</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3 md:gap-4">

          <!-- 播放最多的歌曲：行点击播放，整榜作为队列 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <Flame class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">播放最多的歌曲</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.loadSmartPlaylist('most_played')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in topPlayed"
                :key="item.id"
                class="flex items-center px-4 py-[6px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playFromList(topPlayed, idx)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-[13px] text-text-primary truncate leading-tight">{{ item.title }}</p>
                  <p class="text-[11px] text-text-muted truncate">{{ item.artist }}</p>
                </div>
                <span class="text-[11px] text-text-muted font-mono tabular-nums ml-3 flex-shrink-0">{{ item.playCount }} 次</span>
              </div>
              <div v-if="topPlayed.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <Music class="w-3.5 h-3.5" /> 暂无播放记录
              </div>
            </div>
          </div>

          <!-- 听得最多的歌手：行点击进艺人详情 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <User class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">听得最多的歌手</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.navigateToTab('艺术家')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in topArtists"
                :key="item.id"
                class="flex items-center px-4 py-[9px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playerStore.navigateToArtist(item.id)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <p class="flex-1 min-w-0 text-[13px] text-text-primary truncate">{{ item.name }}</p>
                <span class="text-[11px] text-text-muted font-mono tabular-nums ml-3 flex-shrink-0">{{ item.playCount }} 次</span>
              </div>
              <div v-if="topArtists.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <User class="w-3.5 h-3.5" /> 暂无播放记录
              </div>
            </div>
          </div>

          <!-- 播放最多的专辑：行点击进专辑详情 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <Disc class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">播放最多的专辑</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.navigateToTab('专辑')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in topAlbums"
                :key="item.id"
                class="flex items-center px-4 py-[6px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playerStore.navigateToAlbum(item.id)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-[13px] text-text-primary truncate leading-tight">{{ item.title }}</p>
                  <p class="text-[11px] text-text-muted truncate">{{ item.artist }}</p>
                </div>
                <span class="text-[11px] text-text-muted font-mono tabular-nums ml-3 flex-shrink-0">{{ item.playCount }} 次</span>
              </div>
              <div v-if="topAlbums.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <Disc class="w-3.5 h-3.5" /> 暂无播放记录
              </div>
            </div>
          </div>

          <!-- 最近播放：行点击播放 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <History class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">最近播放</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.navigateToTab('最近播放')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in recentPlayed"
                :key="item.id"
                class="flex items-center px-4 py-[6px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playFromList(recentPlayed, idx)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-[13px] text-text-primary truncate leading-tight">{{ item.title }}</p>
                  <p class="text-[11px] text-text-muted truncate">{{ item.artist }}</p>
                </div>
              </div>
              <div v-if="recentPlayed.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <History class="w-3.5 h-3.5" /> 还没有播放记录
              </div>
            </div>
          </div>

          <!-- 最近添加：行点击播放 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <CalendarPlus class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">最近添加</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.loadSmartPlaylist('recently_added')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in recentAdded"
                :key="item.id"
                class="flex items-center px-4 py-[6px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playFromList(recentAdded, idx)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-[13px] text-text-primary truncate leading-tight">{{ item.title }}</p>
                  <p class="text-[11px] text-text-muted truncate">{{ item.artist }}</p>
                </div>
              </div>
              <div v-if="recentAdded.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <Music class="w-3.5 h-3.5" /> 曲库还没有歌曲
              </div>
            </div>
          </div>

          <!-- 我喜欢的音乐：行点击播放 -->
          <div class="bg-bg-content border border-border-color rounded-[10px] flex flex-col min-w-0">
            <div class="flex items-center justify-between px-4 py-3 border-b border-border-color">
              <div class="flex items-center gap-2 min-w-0">
                <Heart class="w-[14px] h-[14px] text-brand-orange flex-shrink-0" />
                <span class="text-[13px] font-semibold text-text-primary truncate">我喜欢的音乐</span>
              </div>
              <button
                class="text-[11px] text-text-muted hover:text-brand-orange transition-colors-smooth flex items-center gap-0.5 flex-shrink-0"
                @click="playerStore.navigateToTab('喜欢的音乐')"
              >更多 <ChevronRight class="w-3 h-3" /></button>
            </div>
            <div class="py-1">
              <div
                v-for="(item, idx) in favoriteList"
                :key="item.id"
                class="flex items-center px-4 py-[6px] hover:bg-list-hover cursor-pointer transition-colors-smooth"
                @click="playFromList(favoriteList, idx)"
              >
                <span class="w-5 text-[11px] font-mono tabular-nums flex-shrink-0" :class="idx < 3 ? 'text-brand-orange' : 'text-text-muted'">{{ idx + 1 }}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-[13px] text-text-primary truncate leading-tight">{{ item.title }}</p>
                  <p class="text-[11px] text-text-muted truncate">{{ item.artist }}</p>
                </div>
              </div>
              <div v-if="favoriteList.length === 0" class="px-4 py-6 flex items-center justify-center gap-2 text-[12px] text-text-muted">
                <Heart class="w-3.5 h-3.5" /> 还没有收藏的歌曲
              </div>
            </div>
          </div>

        </div>
      </template>
    </div>
  </div>
</template>
