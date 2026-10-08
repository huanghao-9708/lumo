import { describe, expect, it } from 'vitest';
import {
  collectLegacyPreferences,
  LEGACY_PREFERENCES_EXPORT_FORMAT,
  LEGACY_PREFERENCES_EXPORT_VERSION,
} from './migrationExport';

describe('legacy preference export', () => {
  it('exports only versioned allowlisted local preferences and excludes secrets', () => {
    const values = new Map([
      ['lumo_dark_mode', '1'],
      ['lumo_track_columns', '{"detailedView":true,"columns":{"year":true}}'],
      ['lumo_artwork_cache_mb', '32'],
      ['ai_api_key', 'secret-key'],
      ['lumo_webdav_password', 'secret-password'],
    ]);
    const exported = collectLegacyPreferences(
      { getItem: key => values.get(key) ?? null },
      '2026-10-03T00:00:00.000Z',
    );

    expect(exported).toEqual({
      format: LEGACY_PREFERENCES_EXPORT_FORMAT,
      version: LEGACY_PREFERENCES_EXPORT_VERSION,
      exportedAt: '2026-10-03T00:00:00.000Z',
      preferences: {
        lumo_dark_mode: '1',
        lumo_track_columns: '{"detailedView":true,"columns":{"year":true}}',
        lumo_artwork_cache_mb: '32',
      },
    });
    expect(JSON.stringify(exported)).not.toContain('secret');
  });

  it('does not modify source storage and represents absent values as absent', () => {
    const exported = collectLegacyPreferences({
      getItem: key => (key === 'lumo_volume' ? '65' : null),
    });

    expect(exported.preferences).toEqual({ lumo_volume: '65' });
  });
});
