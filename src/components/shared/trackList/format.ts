/**
 * 歌曲列表字段格式化（LDL v2 song-row 规范）。
 *
 * 原则：只描述能够验证的事实——
 * - 音质信息只展示真实提取到的比特率/位深/采样率，缺失则省略，不推断"无损/Hi-Res"；
 * - 年份/流派/文件大小缺失统一显示 `—`；
 * - 不在渲染路径里发 IPC 或读文件。
 */

/** 格式化所需的最小字段集（结构化类型，便于单测与解耦） */
export interface TrackFormatFields {
  format?: string | null;
  bitrate?: number | null;
  sampleRate?: number | null;
  bitDepth?: number | null;
  fileSize?: number | null;
  year?: number | null;
  genres?: string | null;
}

/** 缺失值占位符 */
export const FIELD_PLACEHOLDER = '—';

/**
 * 音频信息列：`FLAC` / `MP3 · 320 kbps` / `FLAC · 24bit/96kHz`。
 * bitrate 单位为 bps（与 media_files.bitrate 一致）。
 */
export function formatAudioInfo(t: Pick<TrackFormatFields, 'format' | 'bitrate' | 'sampleRate' | 'bitDepth'>): string {
  const base = t.format ? t.format.toUpperCase() : '';
  if (!base) return FIELD_PLACEHOLDER;
  // 位深+采样率成对出现才展示，避免半截信息造成误导
  if (t.bitDepth && t.bitDepth > 0 && t.sampleRate && t.sampleRate > 0) {
    return `${base} · ${t.bitDepth}bit/${formatSampleRate(t.sampleRate)}`;
  }
  if (t.bitrate && t.bitrate > 0) {
    return `${base} · ${Math.round(t.bitrate / 1000)} kbps`;
  }
  return base;
}

/** 采样率 → `44.1kHz` / `48kHz`（去掉多余的 0） */
export function formatSampleRate(hz: number): string {
  const khz = hz / 1000;
  const rounded = Math.round(khz * 10) / 10;
  return `${rounded}kHz`;
}

/** 年份列：缺失显示 `—`；异常值（<1000 或 >3000）不展示 */
export function formatTrackYear(year: number | null | undefined): string {
  if (!year || year < 1000 || year > 3000) return FIELD_PLACEHOLDER;
  return String(year);
}

/** 文件大小列：`8.4 MB` / `968 KB`；缺失显示 `—` */
export function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return FIELD_PLACEHOLDER;
  if (bytes < 1024 * 1024) {
    const kb = Math.max(1, Math.round(bytes / 1024));
    return `${kb} KB`;
  }
  const mb = bytes / (1024 * 1024);
  return `${mb >= 10 ? Math.round(mb) : Math.round(mb * 10) / 10} MB`;
}

/** 流派列：多流派以 ` / ` 连接展示；缺失显示 `—` */
export function formatTrackGenres(genres: string | null | undefined): string {
  const raw = (genres ?? '').trim();
  if (!raw) return FIELD_PLACEHOLDER;
  return raw;
}
