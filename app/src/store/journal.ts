// Creation journal (창작 일지): this device's on/off switch and the first-use
// notice (docs/creation-proof.md §7). The editor's part is editor/journal.ts.

import { api } from '../api';
import type { JournalSettings } from '../api/types';
import { setJournalOn } from '../editor/journal';
import { openDialog, showToast, toastError } from './ui';

/**
 * Reads this device's setting when a project opens. The first time, tells
 * the writer the journal is kept, with a way to turn it off. `quiet` holds
 * the notice back (another toast is showing); it comes next time.
 */
export async function loadJournal(quiet: boolean) {
  let settings: JournalSettings;
  try {
    settings = await api.journalSettings();
  } catch {
    return;
  }
  setJournalOn(settings.enabled);
  if (settings.noticed || !settings.enabled || quiet) return;
  showToast({
    text: '쓰는 과정을 이 기기에 기록합니다(내용은 기록하지 않음)',
    action: { label: '끄기…', run: () => openDialog({ kind: 'project', tab: 'basic' }) },
  });
  try {
    await api.journalSet({ noticed: true });
  } catch {
    // Shown again next time; no harm.
  }
}

/** Turns the journal on or off on this device (all its projects). */
export async function setJournalEnabled(enabled: boolean): Promise<JournalSettings | null> {
  try {
    const settings = await api.journalSet({ enabled, noticed: true });
    setJournalOn(settings.enabled);
    return settings;
  } catch (e) {
    toastError('창작 일지 설정을 바꾸지 못함', e);
    return null;
  }
}
