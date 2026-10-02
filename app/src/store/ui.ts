// Small helpers: toasts, dialogs, find, jumps, view settings.

import type { DocSummary, Overview, PartView, Section } from '../api/types';
import { flushAll } from '../lib/flush';
import { errorText } from '../lib/format';
import { type ViewSettings, saveView } from '../lib/view';
import { type Dialog, type FindRequest, type Jump, type Toast, get, set } from './state';
import { selectDoc } from './docs';

// ---------------------------------------------------------------------------
// Small helpers

export interface DocPlace {
  doc: DocSummary;
  section: Section;
  part: PartView | null;
  /** Position among all manuscript documents, from 1. */
  number: number | null;
}

export function findDoc(ov: Overview, id: string): DocPlace | null {
  let n = 0;
  for (const part of ov.parts) {
    for (const doc of part.docs) {
      n += 1;
      if (doc.id === id) return { doc, section: 'manuscript', part, number: n };
    }
  }
  const doc = ov.planning.find((d) => d.id === id);
  return doc ? { doc, section: 'planning', part: null, number: null } : null;
}

export function allManuscript(ov: Overview): DocSummary[] {
  return ov.parts.flatMap((p) => p.docs);
}

/** Waits for every pending save. False when something could not be saved. */
export async function saveEverything(): Promise<boolean> {
  await flushAll();
  const save = get().save;
  if (save.state === 'error') {
    set({ toast: { text: `저장하지 못함 · ${save.error ?? ''}`, tone: 'error' } });
    return false;
  }
  return true;
}

export function showToast(toast: Toast) {
  set({ toast });
}

export function toastError(lead: string, e: unknown) {
  set({ toast: { text: `${lead} · ${errorText(e)}`, tone: 'error' } });
}

export function openDialog(dialog: Dialog) {
  set({ dialog });
}

export function closeDialog() {
  set({ dialog: null });
}

export function openFind(request: Omit<FindRequest, 'nonce'>) {
  set({ rightOpen: true, rightTab: 'find', findRequest: { ...request, nonce: Date.now() } });
}

/** Opens the document if needed, then selects the range (see editor/search.ts). */
export async function jumpTo(jump: Jump) {
  set({ pendingJump: jump });
  if (get().activeDocId !== jump.docId) await selectDoc(jump.docId);
  else set({ pendingJump: { ...jump } });
}

export function setView(patch: Partial<ViewSettings>) {
  const view = { ...get().view, ...patch };
  saveView(view);
  set({ view });
}

/** Updates one document's line in the overview without asking the disk. */
export function patchSummary(id: string, patch: Partial<Omit<DocSummary, 'id'>>) {
  const ov = get().overview;
  if (!ov) return;
  const apply = (d: DocSummary) => (d.id === id ? { ...d, ...patch } : d);
  set({
    overview: {
      ...ov,
      parts: ov.parts.map((p) => ({ ...p, docs: p.docs.map(apply) })),
      planning: ov.planning.map(apply),
    },
  });
}
