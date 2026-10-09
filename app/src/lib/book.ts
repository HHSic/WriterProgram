/** Reading preferences stay on this device; the manuscript is never changed. */
export type BookDevice = 'auto' | 'phone' | 'pc';
export type BookTheme = 'paper' | 'light' | 'dark';
export interface BookPreferences { device: BookDevice; theme: BookTheme; fontSize: number }
const KEY = 'wp.bookReader';
const DEFAULTS: BookPreferences = { device: 'auto', theme: 'paper', fontSize: 18 };

export function loadBookPreferences(): BookPreferences {
  try {
    const value = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    return {
      device: ['auto', 'phone', 'pc'].includes(value?.device) ? value.device : DEFAULTS.device,
      theme: ['paper', 'light', 'dark'].includes(value?.theme) ? value.theme : DEFAULTS.theme,
      fontSize: Number.isFinite(value?.fontSize) ? Math.max(14, Math.min(30, value.fontSize)) : DEFAULTS.fontSize,
    };
  } catch { return { ...DEFAULTS }; }
}

export function saveBookPreferences(value: BookPreferences) {
  try { localStorage.setItem(KEY, JSON.stringify(value)); } catch { /* Reading still works without storage. */ }
}

export const BOOK_GAP = 48;
export function bookColumns(width: number, device: BookDevice): number {
  return device !== 'phone' && width >= 640 ? 2 : 1;
}

export function bookLayout(width: number, extent: number, columns: number) {
  const columnWidth = Math.max(1, (width - BOOK_GAP * (columns - 1)) / columns);
  const pages = Math.max(1, Math.ceil((extent + BOOK_GAP - 0.5) / (columnWidth + BOOK_GAP)));
  return { columns, pages, spreads: Math.ceil(pages / columns), step: width + BOOK_GAP };
}
