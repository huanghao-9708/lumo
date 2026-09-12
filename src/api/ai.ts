import { invoke } from '../utils/tauriInvoke';
import type { TrackDTO } from './types';

/** AI 设置（API key 永不出后端，前端只见 has_key） */
export interface AiSettingsDTO {
  enabled: boolean;
  base_url: string;
  model: string;
  temperature: number;
  has_key: boolean;
}

/** AI 推荐歌单条目：TrackDTO + 一句话推荐理由 */
export interface AiRankedTrackDTO extends TrackDTO {
  reason?: string;
}

/** AI 推荐结果。source='fallback' 时 degraded_reason 说明降级原因 */
export interface AiPlaylistResultDTO {
  name: string;
  description: string;
  source: 'ai' | 'fallback';
  degraded_reason?: string;
  tracks: AiRankedTrackDTO[];
}

export interface AiTestConnectionResultDTO {
  ok: boolean;
  message: string;
  latency_ms: number;
}

export type AiGenerateMode = 'recent' | 'seed' | 'phrase';

export function aiGetSettings(): Promise<AiSettingsDTO> {
  return invoke('ai_get_settings');
}

/** api_key 语义：null=保持不变，''=清除已存 Key，其他=重新存储 */
export function aiSaveSettings(payload: {
  enabled: boolean;
  baseUrl: string;
  model: string;
  temperature?: number;
  apiKey?: string | null;
}): Promise<AiSettingsDTO> {
  return invoke('ai_save_settings', payload);
}

export function aiTestConnection(): Promise<AiTestConnectionResultDTO> {
  return invoke('ai_test_connection');
}

/** requestId 由前端生成，供 aiCancelGenerate 取消 */
export function aiGeneratePlaylist(requestId: string, mode: AiGenerateMode, phrase?: string, seedTrackId?: number): Promise<AiPlaylistResultDTO> {
  return invoke('ai_generate_playlist', { requestId, mode, phrase, seedTrackId });
}

export function aiCancelGenerate(requestId: string): Promise<void> {
  return invoke('ai_cancel_generate', { requestId });
}
