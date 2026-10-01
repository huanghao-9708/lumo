/** Keep unverified desktop modes in development/explicit experiment builds (M4 release gate). */
export const DESKTOP_MODES_ENABLED = import.meta.env.DEV || import.meta.env.VITE_LUMO_DESKTOP_MODES === '1';
