/**
 * 桌面端统一歌曲列表列定义与解析（LDL v2 song-row 规范的单一来源）。
 *
 * 三层架构中的第 1 层「列定义与格式化」：
 * - 表头（TrackListHeader）与歌曲行（TrackRow）由同一份列配置驱动；
 * - 各页面不再复制各自的 flex/hidden 规则；
 * - 列的显隐由**实际容器宽度**决定（ResizeObserver 测量），不再依赖浏览器视口断点。
 *
 * 收纳次序（容器变窄时先隐藏）：流派 → 文件大小 → 年份 → 音频信息 → 专辑 → 艺术家 → 时长。
 * 标题、序号/播放入口、收藏、更多操作始终保留。
 */

export type TrackColumnId =
  | 'index'
  | 'favorite'
  | 'title'
  | 'artist'
  | 'album'
  | 'duration'
  | 'audioInfo'
  | 'year'
  | 'genre'
  | 'fileSize'
  | 'playedAt'
  | 'more';

export interface TrackColumnDef {
  id: TrackColumnId;
  /** 表头文案（本项目无 i18n，中文即规范） */
  label: string;
  /** flex 列伸展，fixed 列定宽 */
  kind: 'flex' | 'fixed';
  /** flex 伸缩比（flex-grow） */
  grow?: number;
  /** flex 列最小宽度（px）——判断容器能否容纳该列 */
  minWidth?: number;
  /** fixed 列宽度（px） */
  width?: number;
  align: 'left' | 'right' | 'center';
  /**
   * 自动收纳次序：容器变窄时**数值小者先被隐藏**；0 = 核心列不参与自动收纳。
   * 1 流派 → 2 文件大小 → 3 年份 → 4 音频信息 → 5 专辑 → 6 艺术家 → 7 时长
   */
  collapseOrder: number;
  /** 是否出现在表头「显示列」菜单中（用户可增减的列） */
  optional: boolean;
  /** optional 列的默认可见性（详细视图开启时全部为 true） */
  defaultVisible: boolean;
  /** 表头是否显示文字（操作列留空） */
  showLabel?: boolean;
}

/** 标题列保留的最小可读宽度（px） */
export const TITLE_MIN_WIDTH = 150;

/** 行高常量：虚拟列表的占位高度计算与行渲染共用同一来源 */
export const TRACK_ROW_HEIGHT = 40;

/** 标准列全集（渲染顺序即 DOM 顺序） */
export const TRACK_COLUMNS: TrackColumnDef[] = [
  { id: 'index', label: '#', kind: 'fixed', width: 40, align: 'center', collapseOrder: 0, optional: false, defaultVisible: true },
  { id: 'favorite', label: '', kind: 'fixed', width: 32, align: 'center', collapseOrder: 0, optional: false, defaultVisible: true },
  { id: 'title', label: '标题', kind: 'flex', grow: 2, minWidth: TITLE_MIN_WIDTH, align: 'left', collapseOrder: 0, optional: false, defaultVisible: true },
  { id: 'artist', label: '艺术家', kind: 'flex', grow: 1.5, minWidth: 96, align: 'left', collapseOrder: 6, optional: false, defaultVisible: true },
  { id: 'album', label: '专辑', kind: 'flex', grow: 1.5, minWidth: 96, align: 'left', collapseOrder: 5, optional: false, defaultVisible: true },
  { id: 'duration', label: '时长', kind: 'fixed', width: 56, align: 'right', collapseOrder: 7, optional: false, defaultVisible: true },
  { id: 'audioInfo', label: '音频', kind: 'fixed', width: 104, align: 'left', collapseOrder: 4, optional: false, defaultVisible: true, showLabel: true },
  { id: 'year', label: '年份', kind: 'fixed', width: 52, align: 'right', collapseOrder: 3, optional: true, defaultVisible: false },
  { id: 'genre', label: '流派', kind: 'fixed', width: 96, align: 'left', collapseOrder: 1, optional: true, defaultVisible: false },
  { id: 'fileSize', label: '大小', kind: 'fixed', width: 72, align: 'right', collapseOrder: 2, optional: true, defaultVisible: false },
  { id: 'playedAt', label: '播放时间', kind: 'fixed', width: 88, align: 'right', collapseOrder: 0, optional: false, defaultVisible: true },
  { id: 'more', label: '', kind: 'fixed', width: 32, align: 'center', collapseOrder: 0, optional: false, defaultVisible: true, showLabel: false },
];

const COLUMN_INDEX = new Map(TRACK_COLUMNS.map(c => [c.id, c]));

/** 标准列表渲染的列（optional 列仍受用户偏好与宽度收纳约束） */
export const STANDARD_COLUMN_IDS: TrackColumnId[] = [
  'index', 'favorite', 'title', 'artist', 'album', 'duration', 'audioInfo', 'year', 'genre', 'fileSize', 'more',
];

/** 页面上下文：声明本页可用的列集合（默认标准列） */
export interface TrackListContext {
  /** 本页强制不显示的列（如专辑详情无专辑列、搜索页无收藏列） */
  hidden?: TrackColumnId[];
  /** 页面专属列（如最近播放的 playedAt），插入在 more 列之前 */
  extra?: TrackColumnId[];
  /** 用户显式启用的可选列（优先级高于宽度收纳，空间不足时压缩标题而不丢列） */
  pinned?: TrackColumnId[];
  /** 详细视图：开启全部可选列 */
  detailedView?: boolean;
}

/** 按 id 取列定义（含页面专属的防御式拷贝） */
export function getColumnDef(id: TrackColumnId): TrackColumnDef {
  const def = COLUMN_INDEX.get(id);
  if (!def) throw new Error(`未知列: ${id}`);
  return def;
}

function columnWidthPx(col: TrackColumnDef): number {
  return col.kind === 'fixed' ? (col.width ?? 0) : (col.minWidth ?? 0);
}

/** 核心固定宽度：序号 + 收藏 + 更多 */
const CORE_FIXED_WIDTH = 40 + 32 + 32;

/**
 * 解析当前容器宽度下应渲染的列。
 *
 * 规则（与计划 §3.4 一致）：
 * - 用户显式启用的列（pinned / 偏好 true）永远渲染；
 * - 显式停用的列（偏好 false）永不渲染；
 * - 其余列按收纳次序从最重要到次要依次装入，放不下的隐藏；
 * - 标题列始终保留（收缩到最小可读宽度）。
 */
export function resolveTrackColumns(
  containerWidth: number,
  context: TrackListContext = {},
): TrackColumnDef[] {
  const hidden = new Set(context.hidden ?? []);
  const pinned = new Set(context.pinned ?? []);
  const detailed = context.detailedView ?? false;

  // 标准列 + 页面专属列（按定义顺序插到 more 列之前）
  const standardIds = STANDARD_COLUMN_IDS.filter(id => !hidden.has(id));
  const extraIds = (context.extra ?? []).filter(id => !hidden.has(id) && !standardIds.includes(id));
  const mergedIds = [...standardIds];
  const morePos = mergedIds.indexOf('more');
  const insertAt = morePos >= 0 ? morePos : mergedIds.length;
  extraIds.forEach((id, i) => mergedIds.splice(insertAt + i, 0, id));
  const candidates = mergedIds.map(getColumnDef);

  // 每列的目标可见性
  const wanted: TrackColumnDef[] = [];
  const auto: { col: TrackColumnDef; priority: number }[] = [];
  for (const col of candidates) {
    if (col.collapseOrder === 0 || pinned.has(col.id)) {
      wanted.push(col); // 核心列 / 用户钉住的列
      continue;
    }
    // optional 列：显式偏好 > 详细视图 > 默认值；非 optional 信息列默认想要
    const desired = col.optional ? (detailed || col.defaultVisible) : true;
    if (desired) auto.push({ col, priority: col.collapseOrder });
  }

  // 从最重要（collapseOrder 大）到次要依次装入可用空间
  auto.sort((a, b) => b.priority - a.priority);
  let budget = containerWidth - CORE_FIXED_WIDTH - TITLE_MIN_WIDTH;
  for (const { col } of auto) {
    if (budget - columnWidthPx(col) >= 0) {
      wanted.push(col);
      budget -= columnWidthPx(col);
    }
  }

  // 恢复标准 DOM 顺序
  return candidates.filter(c => wanted.includes(c));
}

/** 列单元格的共享样式：表头与行用同一函数，保证对齐与宽度一致 */
export function columnCellStyle(col: TrackColumnDef): Record<string, string> {
  if (col.kind === 'flex') {
    return {
      flex: `${col.grow ?? 1} ${(col.grow ?? 1) * 100} 0%`,
      minWidth: col.minWidth ? `${col.minWidth}px` : '0px',
    };
  }
  return { width: `${col.width ?? 0}px`, flexShrink: '0' };
}

/** 列对齐 class（表头与行共用） */
export function columnAlignClass(col: TrackColumnDef): string {
  switch (col.align) {
    case 'right': return 'text-right';
    case 'center': return 'text-center';
    default: return 'text-left';
  }
}
