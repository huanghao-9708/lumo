import { describe, expect, it } from 'vitest';
import {
  FIELD_PLACEHOLDER,
  formatAudioInfo,
  formatFileSize,
  formatSampleRate,
  formatTrackGenres,
  formatTrackYear,
} from './format';

describe('formatAudioInfo', () => {
  it('只有格式时只显示格式（不以扩展名推断音质）', () => {
    expect(formatAudioInfo({ format: 'flac' })).toBe('FLAC');
  });

  it('有比特率显示 kbps（bps → kbps 换算）', () => {
    expect(formatAudioInfo({ format: 'mp3', bitrate: 320_000 })).toBe('MP3 · 320 kbps');
  });

  it('位深和采样率成对出现才展示 Hi-Res 参数', () => {
    expect(formatAudioInfo({ format: 'flac', bitDepth: 24, sampleRate: 96_000 }))
      .toBe('FLAC · 24bit/96kHz');
    // 半截信息不展示，避免误导
    expect(formatAudioInfo({ format: 'flac', bitDepth: 24 })).toBe('FLAC');
    expect(formatAudioInfo({ format: 'flac', sampleRate: 96_000 })).toBe('FLAC');
  });

  it('位深优先于比特率', () => {
    expect(formatAudioInfo({ format: 'flac', bitrate: 1_000_000, bitDepth: 16, sampleRate: 44_100 }))
      .toBe('FLAC · 16bit/44.1kHz');
  });

  it('缺失格式显示占位符', () => {
    expect(formatAudioInfo({})).toBe(FIELD_PLACEHOLDER);
    expect(formatAudioInfo({ format: null })).toBe(FIELD_PLACEHOLDER);
  });

  it('数值为零视为缺失', () => {
    expect(formatAudioInfo({ format: 'mp3', bitrate: 0 })).toBe('MP3');
  });
});

describe('formatSampleRate / formatTrackYear / formatFileSize / formatTrackGenres', () => {
  it('采样率去掉多余的 0', () => {
    expect(formatSampleRate(44_100)).toBe('44.1kHz');
    expect(formatSampleRate(48_000)).toBe('48kHz');
  });

  it('年份：缺失与异常值显示 —', () => {
    expect(formatTrackYear(2020)).toBe('2020');
    expect(formatTrackYear(null)).toBe(FIELD_PLACEHOLDER);
    expect(formatTrackYear(0)).toBe(FIELD_PLACEHOLDER);
    expect(formatTrackYear(3025)).toBe(FIELD_PLACEHOLDER);
  });

  it('文件大小：KB / MB 分级，缺失显示 —', () => {
    expect(formatFileSize(null)).toBe(FIELD_PLACEHOLDER);
    expect(formatFileSize(0)).toBe(FIELD_PLACEHOLDER);
    expect(formatFileSize(968 * 1024)).toBe('968 KB');
    expect(formatFileSize(8 * 1024 * 1024)).toBe('8 MB');
    expect(formatFileSize(Math.round(8.4 * 1024 * 1024))).toBe('8.4 MB');
    expect(formatFileSize(42 * 1024 * 1024)).toBe('42 MB');
  });

  it('流派：原样展示，缺失显示 —', () => {
    expect(formatTrackGenres('Pop; Rock')).toBe('Pop; Rock');
    expect(formatTrackGenres(null)).toBe(FIELD_PLACEHOLDER);
    expect(formatTrackGenres('  ')).toBe(FIELD_PLACEHOLDER);
  });
});
