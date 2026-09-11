/**
 * DB 时间戳解析与格式化。
 *
 * 后端 tracks.last_played_at / play_history.played_at 由 SQLite datetime('now')
 * 写出，是 UTC 朴素串（如 "2026-09-11 01:19:51"，无时区标记）。
 * 浏览器 `new Date("2026-09-11 01:19:51")` 会按本地时区解析——GMT+8 下整整偏 8 小时，
 * 「今日/昨日」判断会错。
 *
 * 约定：任何地方都不要直接 `new Date(dbTime)`，一律走这里。
 */

/** 解析 DB 时间戳：无时区标记的串补 `Z`（按 UTC）再解析；无法解析返回 null */
export function parseDbTime(value: string | null | undefined): Date | null {
  if (!value) return null;
  const raw = value.trim();
  if (!raw) return null;
  // SQLite 串的空格分隔换成 T，跨浏览器解析行为一致
  const normalized = raw.includes('T') ? raw : raw.replace(' ', 'T');
  const hasZone = /(?:Z|[+-]\d{2}:?\d{2})$/.test(normalized);
  const d = new Date(hasZone ? normalized : `${normalized}Z`);
  return isNaN(d.getTime()) ? null : d;
}

function formatHm(d: Date): string {
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** 今日 / 昨日 / 日期 三态格式（最近播放列表的播放时间列） */
export function formatPlayedAt(value: string | null | undefined): string {
  const d = parseDbTime(value);
  if (!d) return '--';
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);
  if (d >= today) return `今日 ${formatHm(d)}`;
  if (d >= yesterday) return `昨日 ${formatHm(d)}`;
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** 相对时间：刚刚 / N 分钟前 / N 小时前 / 昨天 HH:mm / N 天前 / MM-DD（首页「上次听歌」卡） */
export function formatRelativeTime(value: string | null | undefined): string {
  const d = parseDbTime(value);
  if (!d) return '--';
  const diffMs = Date.now() - d.getTime();
  if (diffMs < 0) return '刚刚';
  const min = Math.floor(diffMs / 60000);
  if (min < 1) return '刚刚';
  if (min < 60) return `${min} 分钟前`;
  const hour = Math.floor(min / 60);
  if (hour < 24) return `${hour} 小时前`;
  const day = Math.floor(hour / 24);
  if (day === 1) return `昨天 ${formatHm(d)}`;
  if (day < 7) return `${day} 天前`;
  return `${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
