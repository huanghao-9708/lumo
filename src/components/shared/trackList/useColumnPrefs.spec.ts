import { beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import {
  resolveTrackColumnsWithPrefs,
  useColumnPrefs,
  type TrackColumnPrefs,
} from './useColumnPrefs';

describe('useColumnPrefs', () => {
  beforeEach(() => {
    window.localStorage.clear();
    // 重置模块级单例（重新 import 不可行，直接通过 API 复位）
    const { resetToDefault } = useColumnPrefs();
    resetToDefault();
  });

  it('默认：无显式偏好、非详细视图', () => {
    const { prefs } = useColumnPrefs();
    expect(prefs.value.detailedView).toBe(false);
    expect(prefs.value.columns).toEqual({});
  });

  it('开启详细视图后包含全部可选列（宽度充足时）', () => {
    const { setDetailedView, prefs } = useColumnPrefs();
    setDetailedView(true);
    const got = resolveTrackColumnsWithPrefs(979, {}, prefs.value).map(c => c.id);
    expect(got).toContain('year');
    expect(got).toContain('genre');
    expect(got).toContain('fileSize');
  });

  it('单列显式停用优先于详细视图', () => {
    const { setDetailedView, setColumnPref, prefs } = useColumnPrefs();
    setDetailedView(true);
    setColumnPref('genre', false);
    const got = resolveTrackColumnsWithPrefs(979, {}, prefs.value).map(c => c.id);
    expect(got).toContain('year');
    expect(got).not.toContain('genre');
  });

  it('单列显式启用优先于宽度收纳（窄容器仍显示）', () => {
    const { setColumnPref, prefs } = useColumnPrefs();
    setColumnPref('year', true);
    const got = resolveTrackColumnsWithPrefs(500, {}, prefs.value).map(c => c.id);
    expect(got).toContain('year');
  });

  it('页面上下文与偏好叠加：显式停用并入 hidden', () => {
    const { setColumnPref, prefs } = useColumnPrefs();
    setColumnPref('artist', false);
    const got = resolveTrackColumnsWithPrefs(979, {}, prefs.value).map(c => c.id);
    expect(got).not.toContain('artist');
  });

  it('偏好写入 localStorage（lumo_ 键约定，watch 异步落盘）', async () => {
    const { setColumnPref } = useColumnPrefs();
    setColumnPref('year', true);
    await nextTick();
    const raw = window.localStorage.getItem('lumo_track_columns');
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!) as TrackColumnPrefs;
    expect(parsed.columns.year).toBe(true);
  });

  it('损坏的持久化数据按默认处理不抛错', () => {
    window.localStorage.setItem('lumo_track_columns', '{oops');
    expect(() => useColumnPrefs()).not.toThrow();
  });

  it('详细视图开关不清除单列显式偏好（用户选择优先级最高）', () => {
    const { setDetailedView, setColumnPref, explicitPref, prefs } = useColumnPrefs();
    setColumnPref('fileSize', true);
    setDetailedView(true);
    setDetailedView(false);
    expect(explicitPref('fileSize')).toBe(true);
    expect(explicitPref('year')).toBeUndefined();
    // 关闭详细视图后，钉住的大小列仍然显示
    const got = resolveTrackColumnsWithPrefs(979, {}, prefs.value).map(c => c.id);
    expect(got).toContain('fileSize');
    expect(got).not.toContain('year');
  });
});
