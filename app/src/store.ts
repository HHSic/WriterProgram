import { create } from 'zustand';
import type { Editor } from '@tiptap/core';
import { api } from './api';
import type { Counts, DocStatus, DocSummary, NewDoc, Overview, PartView, Section } from './api/types';
import { errorText } from './lib/format';
import { flushAll } from './lib/flush';
import { loadView, saveView, type ViewSettings } from './lib/view';

export type SaveState = { state: 'saved' | 'saving' | 'error'; error?: string };
export type RightTab = 'outline' | 'records';

export type Dialog =
  | { kind: 'newProject' }
  | { kind: 'trash' }
  | { kind: 'export' }
  | { kind: 'view' }
  | {
      kind: 'prompt';
      title: string;
      label: string;
      value: string;
      confirm: string;
      inputMode?: 'text' | 'numeric';
      onSubmit: (value: string) => Promise<void> | void;
    }
  | {
      kind: 'confirm';
      title: string;
      message: string;
      confirm: string;
      danger?: boolean;
      onConfirm: () => Promise<void> | void;
    };

export interface Toast {
  text: string;
  tone?: 'error';
  action?: { label: string; run: () => void };
}

interface AppState {
  overview: Overview | null;
  activeDocId: string | null;
  editor: Editor | null;
  save: SaveState;
  liveCounts: Counts | null;
  selection: { withSpaces: number; withoutSpaces: number } | null;
  rightTab: RightTab;
  rightOpen: boolean;
  view: ViewSettings;
  dialog: Dialog | null;
  toast: Toast | null;
  /** Bumped to reload the open document from disk (after going back to a record). */
  docVersion: number;
  /** Bumped when the records of the open document change. */
  recordsVersion: number;
}

export const useApp = create<AppState>(() => ({
  overview: null,
  activeDocId: null,
  editor: null,
  save: { state: 'saved' },
  liveCounts: null,
  selection: null,
  rightTab: 'outline',
  rightOpen: true,
  view: loadView(),
  dialog: null,
  toast: null,
  docVersion: 0,
  recordsVersion: 0,
}));

const set = useApp.setState;
const get = useApp.getState;

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

function root(): string {
  const ov = get().overview;
  if (!ov) throw new Error('no project open');
  return ov.root;
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

// ---------------------------------------------------------------------------
// Projects

const LAST_DOC_KEY = 'wp.lastDoc.';

function rememberDoc(projectId: string, docId: string) {
  try {
    localStorage.setItem(LAST_DOC_KEY + projectId, docId);
  } catch {
    // Not critical.
  }
}

function initialDoc(ov: Overview): string | null {
  try {
    const last = localStorage.getItem(LAST_DOC_KEY + ov.project.id);
    if (last && findDoc(ov, last)) return last;
  } catch {
    // Fall through to the first chapter.
  }
  return allManuscript(ov)[0]?.id ?? ov.planning[0]?.id ?? null;
}

export function enterProject(ov: Overview) {
  set({ overview: ov, activeDocId: initialDoc(ov), save: { state: 'saved' }, liveCounts: null, selection: null });
  api.setWindowTitle(`${ov.project.title} · WriterProgram`);
}

export async function openProject(path: string): Promise<boolean> {
  try {
    enterProject(await api.projectOpen(path));
    return true;
  } catch (e) {
    toastError('작품을 열지 못함', e);
    return false;
  }
}

export async function leaveProject() {
  if (!(await saveEverything())) return;
  set({ overview: null, activeDocId: null, editor: null, liveCounts: null, selection: null, dialog: null });
  api.setWindowTitle('WriterProgram');
}

export async function refreshOverview() {
  try {
    set({ overview: await api.projectOverview(root()) });
  } catch (e) {
    toastError('작품 정보를 불러오지 못함', e);
  }
}

// ---------------------------------------------------------------------------
// Documents

export async function selectDoc(id: string) {
  if (get().activeDocId === id) return;
  if (!(await saveEverything())) return;
  set({ activeDocId: id, liveCounts: null, selection: null });
  const ov = get().overview;
  if (ov) rememberDoc(ov.project.id, id);
}

export async function addDoc(spec: NewDoc) {
  try {
    if (!(await saveEverything())) return;
    const id = await api.docAdd(root(), spec);
    await refreshOverview();
    await selectDoc(id);
  } catch (e) {
    toastError('새 문서를 만들지 못함', e);
  }
}

export async function setStatus(id: string, status: DocStatus) {
  try {
    await api.docUpdateMeta(root(), id, { status });
    patchSummary(id, { status });
  } catch (e) {
    toastError('상태를 바꾸지 못함', e);
  }
}

export async function setTarget(id: string, target: number | null) {
  try {
    await api.docUpdateMeta(root(), id, { target });
    patchSummary(id, { target });
  } catch (e) {
    toastError('목표 분량을 바꾸지 못함', e);
  }
}

export async function renameDoc(id: string, title: string) {
  try {
    await api.docUpdateMeta(root(), id, { title });
    patchSummary(id, { title: title.trim() });
  } catch (e) {
    toastError('이름을 바꾸지 못함', e);
  }
}

export async function moveDoc(id: string, partId: string | null, index: number) {
  try {
    await api.docMove(root(), id, partId, index);
    await refreshOverview();
  } catch (e) {
    toastError('옮기지 못함', e);
  }
}

export async function trashDoc(id: string) {
  const ov = get().overview;
  if (!ov) return;
  try {
    if (!(await saveEverything())) return;
    if (get().activeDocId === id) {
      // Open a neighbour from the same list: the next one, or the one before.
      const list = findDoc(ov, id)?.section === 'planning' ? ov.planning : allManuscript(ov);
      const i = list.findIndex((d) => d.id === id);
      const next = list[i + 1] ?? list[i - 1] ?? null;
      set({ activeDocId: next?.id ?? null, editor: null });
    }
    const item = await api.docTrash(ov.root, id);
    await refreshOverview();
    showToast({
      text: '휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void (async () => {
            await api.trashRestore(ov.root, item.id);
            await refreshOverview();
          })();
        },
      },
    });
  } catch (e) {
    toastError('휴지통으로 옮기지 못함', e);
  }
}

// ---------------------------------------------------------------------------
// Parts

export async function addPart() {
  try {
    await api.partAdd(root(), '');
    await refreshOverview();
  } catch (e) {
    toastError('부를 만들지 못함', e);
  }
}

export async function renamePart(id: string, title: string) {
  try {
    await api.partRename(root(), id, title);
    await refreshOverview();
  } catch (e) {
    toastError('부 이름을 바꾸지 못함', e);
  }
}

export async function removePart(id: string) {
  try {
    await api.partRemove(root(), id);
    await refreshOverview();
  } catch (e) {
    toastError('부를 지우지 못함', e);
  }
}
