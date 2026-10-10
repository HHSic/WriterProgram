export const PREFERENCES_CHANGED = 'writer:preferences-changed';
export function notifyPreferencesChanged() {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(PREFERENCES_CHANGED));
}
