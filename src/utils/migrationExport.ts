/** Versioned, local-only export of the legacy WebView preferences whitelist. */
export const LEGACY_PREFERENCES_EXPORT_FORMAT = 'lumo-local-preferences';
export const LEGACY_PREFERENCES_EXPORT_VERSION = 1;

export const LEGACY_PREFERENCE_KEYS = [
  'lumo_dark_mode',
  'lumo_follow_system',
  'lumo_fetch_lyrics',
  'lumo_fetch_covers',
  'lumo_current_index',
  'lumo_current_track_id',
  'lumo_progress_ms',
  'lumo_play_mode',
  'lumo_volume',
  'lumo_playback_rate',
  'lumo_hide_small_albums',
  'lumo_hide_minor_artists',
  'lumo_track_columns',
  'lumo_window_size',
  'lumo_artwork_cache_mb',
] as const;

export interface LegacyPreferencesExport {
  format: typeof LEGACY_PREFERENCES_EXPORT_FORMAT;
  version: typeof LEGACY_PREFERENCES_EXPORT_VERSION;
  exportedAt: string;
  preferences: Partial<Record<(typeof LEGACY_PREFERENCE_KEYS)[number], string>>;
}

export function collectLegacyPreferences(
  storage: Pick<Storage, 'getItem'>,
  exportedAt = new Date().toISOString(),
): LegacyPreferencesExport {
  const preferences: LegacyPreferencesExport['preferences'] = {};
  for (const key of LEGACY_PREFERENCE_KEYS) {
    const value = storage.getItem(key);
    if (value !== null) preferences[key] = value;
  }
  return {
    format: LEGACY_PREFERENCES_EXPORT_FORMAT,
    version: LEGACY_PREFERENCES_EXPORT_VERSION,
    exportedAt,
    preferences,
  };
}

export function downloadLegacyPreferencesExport(
  storage: Pick<Storage, 'getItem'> = window.localStorage,
  now = new Date(),
): number {
  const file = collectLegacyPreferences(storage, now.toISOString());
  const blob = new Blob([JSON.stringify(file, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `lumo-preferences-${now.toISOString().slice(0, 10)}.json`;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
  return Object.keys(file.preferences).length;
}
