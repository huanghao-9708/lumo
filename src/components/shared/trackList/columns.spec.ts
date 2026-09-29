import { describe, expect, it } from 'vitest';
import {
  TRACK_COLUMNS,
  TRACK_ROW_HEIGHT,
  columnCellStyle,
  getColumnDef,
  resolveTrackColumns,
} from './columns';

const ids = (cols: { id: string }[]) => cols.map(c => c.id);

describe('resolveTrackColumns', () => {
  it('默认 1200 宽度下标准视图包含标题/艺术家/专辑/时长/音频，不含可选列', () => {
    const cols = resolveTrackColumns(979); // 1200 窗口的 Content 几何宽
    expect(ids(cols)).toEqual([
      'index', 'favorite', 'title', 'artist', 'album', 'duration', 'audioInfo', 'more',
    ]);
  });

  it('详细视图在 1200 宽度下加入年份/流派/大小', () => {
    const cols = resolveTrackColumns(979, { detailedView: true });
    expect(ids(cols)).toContain('year');
    expect(ids(cols)).toContain('genre');
    expect(ids(cols)).toContain('fileSize');
  });

  it('容器变窄按 流派→大小→年份→音频→专辑→艺术家 收纳', () => {
    // 自宽向窄扫描，记录各列「首次消失」的宽度（值越大 = 越早被收纳）
    const goneAt: Record<string, number> = {};
    for (let w = 979; w >= 360; w -= 1) {
      const visible = new Set(resolveTrackColumns(w, { detailedView: true }).map(c => c.id));
      for (const col of TRACK_COLUMNS) {
        if (!visible.has(col.id) && !(col.id in goneAt)) goneAt[col.id] = w;
      }
    }
    expect(goneAt['genre']).toBeGreaterThan(goneAt['fileSize']);
    expect(goneAt['fileSize']).toBeGreaterThan(goneAt['year']);
    expect(goneAt['year']).toBeGreaterThan(goneAt['audioInfo']);
    expect(goneAt['audioInfo']).toBeGreaterThan(goneAt['album']);
    expect(goneAt['album']).toBeGreaterThan(goneAt['artist']);
    // 核心列到最后（360px）仍在
    expect(goneAt['title']).toBeUndefined();
    expect(goneAt['index']).toBeUndefined();
  });

  it('页面上下文隐藏列（专辑详情无专辑、艺人详情无艺术家）', () => {
    const album = resolveTrackColumns(979, { hidden: ['album'] });
    expect(ids(album)).not.toContain('album');

    const artist = resolveTrackColumns(979, { hidden: ['artist'] });
    expect(ids(artist)).not.toContain('artist');
  });

  it('页面专属列（最近播放的 playedAt）插入在 more 之前', () => {
    const cols = resolveTrackColumns(979, { extra: ['playedAt'] });
    expect(ids(cols)).toContain('playedAt');
    expect(ids(cols).indexOf('playedAt')).toBe(ids(cols).indexOf('more') - 1);
  });

  it('用户钉住的列优先于宽度收纳（空间不足时仍渲染）', () => {
    // 极窄容器 + 钉住流派：流派保留，让标题/其他列让位
    const cols = resolveTrackColumns(500, { pinned: ['genre'] });
    expect(ids(cols)).toContain('genre');
  });

  it('playedAt 不受 detailedView 影响（核心上下文列）', () => {
    const cols = resolveTrackColumns(979, { extra: ['playedAt'], detailedView: false });
    expect(ids(cols)).toContain('playedAt');
  });

  it('列顺序与标准定义顺序一致（不因收纳打乱 DOM 顺序）', () => {
    const cols = resolveTrackColumns(979, { detailedView: true });
    const expectedOrder = TRACK_COLUMNS.filter(c => ids(cols).includes(c.id)).map(c => c.id);
    expect(ids(cols)).toEqual(expectedOrder);
  });
});

describe('列定义共享工具', () => {
  it('getColumnDef 返回定义且未知列报错', () => {
    expect(getColumnDef('title').label).toBe('标题');
    expect(() => getColumnDef('nope' as never)).toThrow();
  });

  it('flex 列样式带最小宽，fixed 列样式定宽', () => {
    const title = columnCellStyle(getColumnDef('title'));
    expect(title.minWidth).toBe('150px');
    expect(title.flex).toContain('2');

    const duration = columnCellStyle(getColumnDef('duration'));
    expect(duration.width).toBe('56px');
    expect(duration.flexShrink).toBe('0');
  });

  it('行高常量为 40px（虚拟列表占位计算依赖）', () => {
    expect(TRACK_ROW_HEIGHT).toBe(40);
  });
});
