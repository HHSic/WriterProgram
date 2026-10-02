// Creation journal (창작 일지) in memory: counts per project, no chain.

import type { Backend, JournalSettings, JournalSummary } from '../types';
import { now } from './state';

const settings: JournalSettings = { device: 'mockdevice01', enabled: true, noticed: false };
const journals = new Map<string, JournalSummary>();

function journal(root: string): JournalSummary {
  let j = journals.get(root);
  if (!j) {
    j = { since: null, saves: 0, sessions: 0, pastes: 0, imports: 0, devices: 0, thisDevice: 0 };
    journals.set(root, j);
  }
  return j;
}

function add(root: string, kind: 'saves' | 'sessions' | 'pastes') {
  if (!settings.enabled) return;
  const j = journal(root);
  j.since ??= now();
  j.devices = 1;
  j[kind] += 1;
  j.thisDevice += 1;
}

/** Called by the stand-in's save, as the Rust save adds a line. */
export function noteMockSave(root: string) {
  add(root, 'saves');
}

export const journalMethods = {
  async journalSettings() {
    return { ...settings };
  },
  async journalSet(patch) {
    if (patch.enabled !== undefined) settings.enabled = patch.enabled;
    if (patch.noticed !== undefined) settings.noticed = patch.noticed;
    return { ...settings };
  },
  async journalEvent(root, event) {
    if (event.kind === 'session') add(root, 'sessions');
    else if (event.chars >= 100) add(root, 'pastes');
  },
  async journalSummary(root) {
    return { ...journal(root) };
  },
  async journalVerify(root) {
    const j = journal(root);
    return { ok: true, files: j.devices ? [{ device: settings.device, lines: j.thisDevice, firstBad: null }] : [] };
  },
} satisfies Partial<Backend>;
