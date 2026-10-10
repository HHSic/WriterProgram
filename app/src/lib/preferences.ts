import type { PersonalSettings, RibbonPreferences } from '../api/types/account';
import { loadBookPreferences, saveBookPreferences } from './book';
import { DEFAULT_VIEW, type ViewSettings } from './view';
import { notifyPreferencesChanged } from './preferenceEvents';
import { ACCENTS, PALETTES } from './colors';

const RIBBON_KEY = 'wp.ribbon';
const TABS = ['편집', '서식', '보기', '쪽', '검토'];
export function loadRibbon(): RibbonPreferences {
  try {
    const value = JSON.parse(localStorage.getItem(RIBBON_KEY) || 'null');
    return { tab: TABS.includes(value?.tab) ? value.tab : '편집', collapsed: value?.collapsed === true };
  } catch { return { tab: '편집', collapsed: false }; }
}
export function saveRibbon(value: RibbonPreferences) {
  try {
    const text = JSON.stringify(value);
    if (localStorage.getItem(RIBBON_KEY) === text) return;
    localStorage.setItem(RIBBON_KEY, text);
    notifyPreferencesChanged();
  } catch { /* Preferences remain usable without storage. */ }
}
const ENUMS: Record<string, readonly string[]> = {
  theme: ['system','light','dark'], font: ['noto-serif','gowun-batang','nanum-myeongjo','sans'],
  toolbar: ['auto','always','never'], quoteStyle: ['curly','corner'],
  palette: [...PALETTES.map(p=>p.id),'custom'],
  accent: ['auto',...ACCENTS.map(a=>a.id)],
};
/** Treat downloaded preferences as data; ignore unknown, invalid or device-only keys. */
export function cleanSettings(input: unknown): PersonalSettings {
  if (!input || typeof input !== 'object') return {};
  const source = input as PersonalSettings;
  const view: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(source.view || {})) {
    if (key === 'readVoice' || !(key in DEFAULT_VIEW)) continue;
    const fallback = DEFAULT_VIEW[key as keyof ViewSettings];
    if (typeof fallback === 'boolean' && typeof value === 'boolean') view[key] = value;
    else if (typeof fallback === 'number' && typeof value === 'number' && Number.isFinite(value)) {
      if (['fontSize','width','lineHeight','readRate'].includes(key) && value <= 0) continue;
      if (key === 'fontSize' && value < 1) continue;
      view[key] = value;
    } else if (key === 'customColor' && typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)) view[key] = value;
    else if (ENUMS[key]?.includes(value as string)) view[key] = value;
  }
  const ribbon: Partial<RibbonPreferences> = {};
  if (TABS.includes(source.ribbon?.tab || '')) ribbon.tab = source.ribbon!.tab;
  if (typeof source.ribbon?.collapsed === 'boolean') ribbon.collapsed = source.ribbon.collapsed;
  const book: Record<string, unknown> = {};
  if (['paper','light','dark'].includes(source.book?.theme || '')) book.theme = source.book!.theme;
  for (const key of ['fontSize','pageWidth','pageHeight'] as const) {
    const value = source.book?.[key];
    if (typeof value === 'number' && Number.isFinite(value) && value >= (key === 'fontSize' ? 1 : 20)) book[key] = value;
  }
  return { view, ribbon, book } as PersonalSettings;
}
export function currentSettings(view: ViewSettings): PersonalSettings {
  const { readVoice: _voice, ...portableView } = view;
  const { device: _device, ...book } = loadBookPreferences();
  return { view: portableView, ribbon: loadRibbon(), book };
}
export function mergeSettings(base: PersonalSettings, patch: PersonalSettings): PersonalSettings {
  return cleanSettings({ view: {...base.view,...patch.view}, ribbon: {...base.ribbon,...patch.ribbon}, book: {...base.book,...patch.book} });
}
export function settingsDiff(before: PersonalSettings, after: PersonalSettings): PersonalSettings {
  const patch: Record<string, Record<string, unknown>> = {};
  for (const section of ['view','ribbon','book'] as const) {
    const changed: Record<string,unknown> = {};
    for (const [key,value] of Object.entries(after[section] || {})) {
      if (value !== (before[section] as Record<string,unknown> | undefined)?.[key]) changed[key] = value;
    }
    if (Object.keys(changed).length) patch[section] = changed;
  }
  return patch;
}
export function hasSettings(patch: PersonalSettings): boolean {
  return Object.values(patch).some(section => section && Object.keys(section).length > 0);
}
export function applyAccessorySettings(settings: PersonalSettings) {
  if (settings.ribbon) saveRibbon({...loadRibbon(),...settings.ribbon});
  if (settings.book) saveBookPreferences({...loadBookPreferences(),...settings.book});
}
