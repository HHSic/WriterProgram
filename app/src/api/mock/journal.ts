// Creation journal (창작 일지) in memory: counts per project, no chain. Time
// stamps and certificates are pretended: nothing leaves the browser.

import type { Backend, JournalSettings, JournalSummary } from '../types';
import { now, project, wait } from './state';

const settings: JournalSettings = { device: 'mockdevice01', enabled: true, noticed: false, anchor: null };
const journals = new Map<string, JournalSummary>();

function journal(root: string): JournalSummary {
  let j = journals.get(root);
  if (!j) {
    j = {
      since: null,
      saves: 0,
      sessions: 0,
      pastes: 0,
      imports: 0,
      exchanges: 0,
      anchors: 0,
      lastAnchor: null,
      devices: 0,
      thisDevice: 0,
    };
    journals.set(root, j);
  }
  return j;
}

function add(root: string, kind: 'saves' | 'sessions' | 'pastes' | 'exchanges') {
  if (!settings.enabled) return;
  const j = journal(root);
  j.since ??= now();
  j.devices = 1;
  j[kind] += 1;
  j.thisDevice += 1;
}

function previewText(root: string, docs: string[] | null): string {
  const p = project(root);
  const count = docs?.length ?? p.parts.reduce((n, part) => n + part.docs.length, 0);
  return `「${p.info.title}」 ${count}개 회차의 창작 과정 기록입니다(미리보기). 이 증명서는 창작 과정의 기록이며 AI 사용 여부를 판단하지 않습니다.`;
}

/** Called by the stand-in's save, as the Rust save adds a line. */
export function noteMockSave(root: string) {
  add(root, 'saves');
}

/** Called when chapters are sent, a corrected file is read or corrections are applied. */
export function noteMockExchange(root: string) {
  add(root, 'exchanges');
}

export const journalMethods = {
  async journalSettings() {
    return { ...settings };
  },
  async journalSet(patch) {
    if (patch.enabled !== undefined) settings.enabled = patch.enabled;
    if (patch.noticed !== undefined) settings.noticed = patch.noticed;
    if (patch.anchor !== undefined) settings.anchor = patch.anchor;
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
  async journalFlush() {},
  async journalAnchor(root, force) {
    if (settings.anchor !== true) return { state: 'notAllowed', signed: [] };
    if (!settings.enabled) return { state: 'journalOff', signed: [] };
    const j = journal(root);
    if (!j.devices) return { state: 'noJournal', signed: [] };
    const today = new Date().toDateString();
    if (!force && j.lastAnchor && new Date(j.lastAnchor).toDateString() === today) {
      return { state: 'doneToday', signed: [] };
    }
    await wait();
    j.anchors += 3;
    j.lastAnchor = now();
    return { state: 'signed', signed: ['DigiCert', 'Sectigo', 'FreeTSA'] };
  },
  async proofPreview(root, options) {
    return previewText(root, options.docs);
  },
  async proofMake(root, options, folder) {
    await wait();
    const summary = previewText(root, options.docs);
    const dir = `${folder}\\${project(root).info.title} 창작 과정 증명`;
    return { folder: dir, html: `${dir}\\창작 과정 증명서.html`, bundle: `${dir}\\증명자료.json`, summary, ok: true };
  },
} satisfies Partial<Backend>;
