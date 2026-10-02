// Changes from other devices, copies, moving a project.

import { api } from '../api';
import type { Change, CopyAction, CopyInfo, SnapshotInfo } from '../api/types';
import { peerOf, reloadDoc, sessionOf } from '../editor/shared';
import { shortPath } from '../lib/format';
import { get, set } from './state';
import { patchSummary, saveEverything, showToast, toastError } from './ui';
import { enterProject, leaveProject, openProject, refreshOverview } from './project';
import { selectDoc } from './docs';
import { refreshCardCounts } from './cards';
import { loadNotes } from './notes';

// ---------------------------------------------------------------------------
// Other devices (crates/core/src/copies.rs, changes.rs)
//
// While a project is open, the app hears about changes it did not make:
// another device's edits brought in by a sync program. An open document the
// writer is not changing just takes them (its text before is kept as a
// record); one being changed on both sides waits for the writer to pick
// (DocPane banner, 둘 다 보기). Copies that sync programs leave are listed next
// to their originals.

let stopWatch: (() => void) | null = null;
const later = new Map<string, ReturnType<typeof setTimeout>>();

/** Runs `fn` once after a burst of calls with the same key. */
function soon(key: string, fn: () => void, ms = 250) {
  clearTimeout(later.get(key));
  later.set(key, setTimeout(() => {
    later.delete(key);
    fn();
  }, ms));
}

export function startWatching(root: string) {
  stopWatch?.();
  stopWatch = api.watchProject(root, (changes) => onChanges(root, changes));
}

export function stopWatching() {
  stopWatch?.();
  stopWatch = null;
}

function onChanges(root: string, changes: Change[]) {
  if (get().overview?.root !== root) return;
  let refresh = false;
  let notes = false;
  const cards: string[] = [];
  for (const c of changes) {
    switch (c.kind) {
      case 'doc':
        refresh = true;
        if (c.rev) void externalDocChange(c.id, c.rev);
        break;
      case 'card':
        refresh = true;
        cards.push(c.id);
        break;
      case 'note':
        notes = true;
        break;
      case 'records':
        if (c.docId === get().activeDocId) set({ recordsVersion: get().recordsVersion + 1 });
        break;
      default:
        refresh = true;
    }
  }
  if (refresh) soon('overview', () => void refreshOverview());
  if (notes) soon('notes', () => void loadNotes());
  if (cards.length) {
    const reloads = { ...get().cardReloads };
    for (const id of cards) reloads[id] = (reloads[id] ?? 0) + 1;
    set({ cardReloads: reloads });
    soon('cardCounts', () => void refreshCardCounts(), 800);
  }
}

/** Another device changed a document's text on disk (its fingerprint is now `rev`). */
async function externalDocChange(docId: string, rev: string) {
  const session = sessionOf(docId);
  if (!session || session.conflict || session.base === rev) return;
  await session.settle();
  if (session.conflict || session.base === rev) return;
  // Changed here too: the save finds the difference and asks the writer.
  if (session.pending) {
    await session.flush();
    return;
  }
  const ov = get().overview;
  const editor = peerOf(docId);
  if (!ov || !editor) return;
  try {
    await api.docKeep(ov.root, docId, editor.getJSON(), 'before-reload').catch(() => null);
    const data = await api.docLoad(ov.root, docId);
    if (session.pending || session.conflict) {
      await session.flush();
      return;
    }
    if (data.rev === session.base) return;
    reloadDoc(docId, data.body, data.rev);
    patchSummary(docId, { counts: data.counts });
    set({ recordsVersion: get().recordsVersion + 1 });
    showToast({
      text: '다른 기기에서 고친 내용을 불러옴',
      action: { label: '이전 글 보기', run: () => void showRecords(docId) },
    });
  } catch (e) {
    toastError('다른 기기에서 고친 내용을 불러오지 못함', e);
  }
}

async function showRecords(docId: string) {
  if (get().activeDocId !== docId) await selectDoc(docId);
  set({ rightOpen: true, rightTab: 'records' });
}

export function markConflict(docId: string, record: SnapshotInfo | null) {
  set({
    conflicts: { ...get().conflicts, [docId]: { record, at: new Date().toISOString() } },
    recordsVersion: get().recordsVersion + 1,
  });
}

export function clearConflict(docId: string) {
  if (!(docId in get().conflicts)) return;
  const conflicts = { ...get().conflicts };
  delete conflicts[docId];
  set({ conflicts });
}

/** 이 기기 것으로 저장: this device's text goes over the other's, which is kept as a record. */
export async function keepMine(docId: string) {
  const session = sessionOf(docId);
  if (!session) return;
  try {
    const out = await session.saveOver();
    clearConflict(docId);
    patchSummary(docId, { counts: out.counts, pages: out.pages });
    set({ recordsVersion: get().recordsVersion + 1 });
    showToast({ text: '이 기기에서 쓴 글로 저장함 · 다른 기기의 글은 기록에 남음' });
  } catch (e) {
    toastError('저장하지 못함', e);
  }
}

/** 다른 기기 것 불러오기: this device's text is already kept as a record. */
export async function takeTheirs(docId: string) {
  const ov = get().overview;
  if (!ov) return;
  try {
    const data = await api.docLoad(ov.root, docId);
    reloadDoc(docId, data.body, data.rev);
    clearConflict(docId);
    patchSummary(docId, { counts: data.counts });
    set({ recordsVersion: get().recordsVersion + 1 });
    showToast({ text: '다른 기기의 글을 불러옴 · 이 기기에서 쓴 글은 기록에 남음' });
  } catch (e) {
    toastError('불러오지 못함', e);
  }
}

const COPY_DONE: Record<CopyAction, string> = {
  take: '사본으로 바꿈',
  discard: '사본을 휴지통으로 옮김',
  keepBoth: '사본을 따로 둠',
};

/** Deals with a copy left by a sync program (see CopyAction). */
export async function resolveCopy(copy: CopyInfo, action: CopyAction): Promise<boolean> {
  const ov = get().overview;
  if (!ov) return false;
  if (!(await saveEverything())) return false;
  try {
    await api.copyResolve(ov.root, copy.section, copy.file, action);
    const isDoc = copy.section === 'manuscript' || copy.section === 'planning';
    if (action === 'take' && isDoc && sessionOf(copy.of)) {
      const data = await api.docLoad(ov.root, copy.of);
      reloadDoc(copy.of, data.body, data.rev);
    }
    if (action === 'take' && copy.section === 'cards') {
      set({ cardReloads: { ...get().cardReloads, [copy.of]: (get().cardReloads[copy.of] ?? 0) + 1 } });
    }
    await refreshOverview();
    if (copy.section === 'notes') await loadNotes();
    if (copy.section === 'cards') void refreshCardCounts();
    set({ recordsVersion: get().recordsVersion + 1 });
    const kept =
      action !== 'take' ? '' : isDoc ? ' · 바꾸기 전 글은 기록에 남음' : ' · 바꾸기 전 것은 휴지통에 있음';
    showToast({ text: COPY_DONE[action] + kept });
    return true;
  } catch (e) {
    toastError('사본을 정리하지 못함', e);
    return false;
  }
}

/** Moves the project folder (for example into OneDrive) and opens it there. */
export async function moveProject(dest: string): Promise<boolean> {
  const ov = get().overview;
  if (!ov) return false;
  if (!(await saveEverything())) return false;
  const from = ov.root;
  set({ busy: '작품 폴더를 옮기는 중', dialog: null });
  try {
    await leaveProject();
    if (get().overview) return false;
    const out = await api.projectMove(from, dest);
    enterProject(out.overview);
    showToast({
      text: out.leftBehind
        ? `옮김 · 원래 폴더의 일부를 지우지 못해 그대로 둠 (${shortPath(from)})`
        : `옮김 · ${shortPath(out.overview.root)}`,
    });
    return true;
  } catch (e) {
    toastError('작품 폴더를 옮기지 못함', e);
    await openProject(from);
    return false;
  } finally {
    set({ busy: null });
  }
}
