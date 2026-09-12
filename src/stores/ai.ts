import { defineStore } from "pinia";
import { ref } from "vue";

import {
  aiGetSettings, aiSaveSettings, aiTestConnection, aiGeneratePlaylist, aiCancelGenerate,
  type AiSettingsDTO, type AiPlaylistResultDTO, type AiTestConnectionResultDTO, type AiGenerateMode,
} from '../api/ai';
import { usePlayerStore, type Track } from './player';
import { useUiStore } from './ui';

/** AI 结果的前端模型（DTO 经 mapTrackDto 映射，可直接喂 playQueue） */
export interface AiPlaylistResult {
  name: string;
  description: string;
  source: 'ai' | 'fallback';
  degradedReason?: string;
  tracks: Array<Track & { reason?: string }>;
}

/**
 * AI 推荐歌单（PRD-AI推荐歌单 2026-09-12）。
 * 设置读写 / 连接测试 / 生成编排（含取消）/ 保存为歌单。
 */
export const useAiStore = defineStore("ai", () => {
  const settings = ref<AiSettingsDTO | null>(null);
  const isLoadingSettings = ref(false);

  const isGenerating = ref(false);
  const result = ref<AiPlaylistResult | null>(null);
  const generateError = ref<string | null>(null);
  const isSaving = ref(false);
  const savedPlaylistName = ref<string | null>(null);

  // 进行中请求的取消句柄（前端侧）
  let activeRequestId: string | null = null;

  async function fetchSettings() {
    isLoadingSettings.value = true;
    try {
      settings.value = await aiGetSettings();
    } catch (e) {
      console.error('[ai] failed to load settings:', e);
    } finally {
      isLoadingSettings.value = false;
    }
  }

  async function saveSettings(payload: {
    enabled?: boolean;
    baseUrl?: string;
    model?: string;
    temperature?: number;
    apiKey?: string | null;
  }): Promise<boolean> {
    const current = settings.value;
    try {
      settings.value = await aiSaveSettings({
        enabled: payload.enabled ?? current?.enabled ?? false,
        baseUrl: payload.baseUrl ?? current?.base_url ?? '',
        model: payload.model ?? current?.model ?? '',
        temperature: payload.temperature ?? current?.temperature ?? 0.8,
        apiKey: payload.apiKey ?? null,
      });
      return true;
    } catch (e) {
      console.error('[ai] failed to save settings:', e);
      return false;
    }
  }

  async function testConnection(): Promise<AiTestConnectionResultDTO> {
    return aiTestConnection();
  }

  /** 生成推荐歌单。取消=放弃结果回到空闲态（后端请求中止）。 */
  async function generate(mode: AiGenerateMode, opts?: { phrase?: string; seedTrackId?: number }) {
    if (isGenerating.value) return;
    isGenerating.value = true;
    generateError.value = null;
    result.value = null;
    savedPlaylistName.value = null;
    activeRequestId = crypto.randomUUID();
    try {
      const dto: AiPlaylistResultDTO = await aiGeneratePlaylist(activeRequestId, mode, opts?.phrase, opts?.seedTrackId);
      const playerStore = usePlayerStore();
      result.value = {
        name: dto.name,
        description: dto.description,
        source: dto.source,
        degradedReason: dto.degraded_reason,
        tracks: dto.tracks.map(t => ({ ...playerStore.mapTrackDTO(t), reason: t.reason })),
      };
    } catch (e: unknown) {
      const msg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      generateError.value = msg;
    } finally {
      isGenerating.value = false;
      activeRequestId = null;
    }
  }

  function cancelGenerate() {
    if (activeRequestId) {
      aiCancelGenerate(activeRequestId).catch(() => {});
    }
    // 不等后端返回：立即回到空闲态，结果被丢弃
    isGenerating.value = false;
    activeRequestId = null;
  }

  /** 把当前结果保存为真实歌单（含 AI 起的名字与简介），返回歌单名 */
  async function saveAsPlaylist(): Promise<string | null> {
    const r = result.value;
    if (!r || r.tracks.length === 0 || isSaving.value) return null;
    isSaving.value = true;
    try {
      const playerStore = usePlayerStore();
      const playlistId = await playerStore.createPlaylist(r.name, r.description);
      await playerStore.batchAddToPlaylist(playlistId, r.tracks.map(t => t.id));
      savedPlaylistName.value = r.name;
      return r.name;
    } catch (e) {
      console.error('[ai] failed to save playlist:', e);
      const uiStore = useUiStore();
      uiStore.showToast('保存歌单失败，请重试');
      return null;
    } finally {
      isSaving.value = false;
    }
  }

  return {
    settings,
    isLoadingSettings,
    isGenerating,
    result,
    generateError,
    isSaving,
    savedPlaylistName,
    fetchSettings,
    saveSettings,
    testConnection,
    generate,
    cancelGenerate,
    saveAsPlaylist,
  };
});
