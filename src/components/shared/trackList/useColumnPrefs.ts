import { ref, watch } from 'vue';
import type { TrackColumnDef, TrackColumnId, TrackListContext } from './columns';
import { resolveTrackColumns } from './columns';

/**
 * 歌曲列表「显示列」偏好（LDL v2 song-row 规范）。
 *
 * - 模块级单例：所有列表视图共享同一份偏好，任意一处的表头菜单修改即时同步全局。
 * - 持久化在 localStorage（`lumo_track_columns`），与本项目的 `lumo_*` 键约定一致。
 * - `columns` 只记录**显式**开关；未记录的列跟随「详细视图」与默认值。
 */

const STORAGE_KEY = 'lumo_track_columns';

export interface TrackColumnPrefs {
  /** 详细视图：一次性打开全部可选列 */
  detailedView: boolean;
  /** 显式的单列开关（true = 钉住显示；false = 停用） */
  columns: Partial<Record<TrackColumnId, boolean>>;
}

function loadPrefs(): TrackColumnPrefs {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<TrackColumnPrefs>;
      if (typeof parsed === 'object' && parsed !== null) {
        return {
          detailedView: parsed.detailedView === true,
          columns: (parsed.columns && typeof parsed.columns === 'object') ? parsed.columns : {},
        };
      }
    }
  } catch {
    // 损坏数据按默认处理
  }
  return { detailedView: false, columns: {} };
}

// 模块级共享状态（所有视图同一份）
const prefs = ref<TrackColumnPrefs>(loadPrefs());

watch(prefs, (value) => {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // localStorage 不可用时放弃持久化，仅内存生效
  }
}, { deep: true });

export function useColumnPrefs() {
  /** 某列的显式偏好：true/false = 用户钉住/停用；undefined = 跟随默认与宽度收纳 */
  function explicitPref(id: TrackColumnId): boolean | undefined {
    return prefs.value.columns[id];
  }

  function setColumnPref(id: TrackColumnId, visible: boolean | undefined) {
    const next = { ...prefs.value.columns };
    if (visible === undefined) delete next[id];
    else next[id] = visible;
    prefs.value = { ...prefs.value, columns: next };
  }

  /** 详细视图开关：只翻转默认值，不清除用户的单列显式偏好（钉住/停用始终优先） */
  function setDetailedView(on: boolean) {
    prefs.value = { detailedView: on, columns: prefs.value.columns };
  }

  function resetToDefault() {
    prefs.value = { detailedView: false, columns: {} };
  }

  return {
    prefs,
    explicitPref,
    setColumnPref,
    setDetailedView,
    resetToDefault,
  };
}

/**
 * 偏好感知的列解析（纯函数，便于单测）：
 * 显式启用 → pinned（优先于宽度收纳）；显式停用 → hidden；
 * 详细视图只影响「无显式偏好」的可选列默认值。
 */
export function resolveTrackColumnsWithPrefs(
  containerWidth: number,
  context: TrackListContext,
  prefs: TrackColumnPrefs,
  trailingWidth = 0,
): TrackColumnDef[] {
  const pinned: TrackColumnId[] = [...(context.pinned ?? [])];
  const hidden: TrackColumnId[] = [...(context.hidden ?? [])];
  for (const key of Object.keys(prefs.columns) as TrackColumnId[]) {
    const on = prefs.columns[key];
    if (on === true && !pinned.includes(key)) pinned.push(key);
    if (on === false && !hidden.includes(key)) hidden.push(key);
  }
  return resolveTrackColumns(containerWidth, {
    ...context,
    pinned,
    hidden,
    detailedView: prefs.detailedView || context.detailedView,
  }, trailingWidth);
}
