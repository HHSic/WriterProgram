import { create } from 'zustand';
import type { Editor } from '@tiptap/core';
import { api } from './api';
import type {
  CardSummary,
  Change,
  CopyAction,
  CopyInfo,
  Counts,
  DriveLink,
  DriveProvider,
  DocStatus,
  DocSummary,
  FormatCatalog,
  ManuscriptFormat,
  NewDoc,
  NewNote,
  Note,
  Overview,
  PartView,
  ProjectPatch,
  Section,
  SnapshotInfo,
} from './api/types';
import { blocksFromNode, countBlocks } from './editor/counts';
import { addMark, removeMarks } from './editor/notes';
import { editorsOf, peerOf, reloadDoc, sessionOf } from './editor/shared';
import { errorText, shortPath } from './lib/format';
import { flushAll } from './lib/flush';
import { newId } from './lib/ids';
import {
  activeTab,
  dropTargets,
  fromSaved,
  makePane,
  moveTab,
  navigate,
  openTab,
  removeTab,
  sameTarget,
  step,
  toSaved,
  type Pane,
  type SplitDir,
  type Target,
} from './lib/tabs';
import { loadView, saveView, type ViewSettings } from './lib/view';

export type { Pane, SplitDir, Target } from './lib/tabs';

export type SaveState = { state: 'saved' | 'saving' | 'error'; error?: string };
export type RightTab = 'outline' | 'notes' | 'cast' | 'find' | 'records';
export type FindScope = 'doc' | 'part' | 'all';

/** Asks the find panel to open with a search (from the sidebar or Ctrl+F). */
export interface FindRequest {
  text?: string;
  scope?: FindScope;
  focus: 'find' | 'replace';
  nonce: number;
}

/** A place to move to once its document has opened. */
export interface Jump {
  docId: string;
  block: number;
  start: number;
  end: number;
}

export type SettingsTab = 'basic' | 'goal' | 'format';

export type Dialog =
  | { kind: 'newProject' }
  | { kind: 'project'; tab?: SettingsTab }
  | { kind: 'trash' }
  | { kind: 'export' }
  /** 가져오기: txt, md, docx into chapters, into `partId` when given. */
  | { kind: 'import'; partId?: string }
  | { kind: 'view' }
  | { kind: 'symbols' }
  /** 둘 다 보기: this device's text next to another device's, or a document next to its copy. */
  | { kind: 'compare'; docId: string; copy?: CopyInfo }
  /** Every copy left by sync programs (다른 기기 사본). */
  | { kind: 'copies' }
  /** 기기 간 맞추기: connecting drives. */
  | { kind: 'drives' }
  /** Bringing a project from a drive to this device. */
  | { kind: 'driveImport' }
  /** Moving the project folder (저장 위치 옮기기). */
  | { kind: 'move' }
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

/** A document whose text another device changed while it was being edited here. */
export interface DocConflict {
  /** This device's text, kept as a record when its save was refused. */
  record: SnapshotInfo | null;
  at: string;
}

export interface Toast {
  text: string;
  tone?: 'error';
  action?: { label: string; run: () => void };
}

interface AppState {
  overview: Overview | null;
  /** The middle column: one pane of tabs, or two side by side (분할). */
  panes: Pane[];
  /** The pane being worked in; the right column and status bar follow it. */
  focus: number;
  split: SplitDir;
  /** What the focused pane's open tab shows, and the same split by kind. */
  activeTarget: Target | null;
  activeDocId: string | null;
  activeCardId: string | null;
  /** Editor of the focused pane's open tab, when that is a document. */
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
  /** Manuscript formats, fonts and paper sizes offered in settings. */
  catalog: FormatCatalog | null;
  findRequest: FindRequest | null;
  pendingJump: Jump | null;
  /** Card shown over the right panel (미리보기). */
  previewCardId: string | null;
  /** 부 picked in the tree: where 새 회차 goes, until another chapter opens. */
  selectedPartId: string | null;
  /** For each card, in how many chapters it appears. */
  cardCounts: Record<string, number>;
  /** Every note (메모) in the project. */
  notes: Note[];
  /** The note being looked at: open in the 메모 tab, its text marked. */
  focusNoteId: string | null;
  /** A note whose text to select once its document is open. */
  pendingNote: string | null;
  /** Documents another device changed while they were being edited here. */
  conflicts: Record<string, DocConflict>;
  /** Bumped per card when another device changed it (open card editors reload). */
  cardReloads: Record<string, number>;
  /** Shown over everything while the app is busy with the whole project. */
  busy: string | null;
  /** The drive folder the open project is kept in step with, if any. */
  link: DriveLink | null;
  /** A pass with the drive is running. */
  syncing: boolean;
  /** Bumped when a browser tab's page or title changes (tab labels follow). */
  webVersion: number;
}

export const useApp = create<AppState>(() => ({
  overview: null,
  panes: [],
  focus: 0,
  split: 'row',
  activeTarget: null,
  activeDocId: null,
  activeCardId: null,
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
  catalog: null,
  findRequest: null,
  pendingJump: null,
  previewCardId: null,
  selectedPartId: null,
  cardCounts: {},
  notes: [],
  focusNoteId: null,
  pendingNote: null,
  conflicts: {},
  cardReloads: {},
  busy: null,
  link: null,
  syncing: false,
  webVersion: 0,
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

// ---------------------------------------------------------------------------
// Tabs and panes (lib/tabs.ts)

/** Editors of open document tabs, by tab key. */
const tabEditors = new Map<string, Editor>();
/** Closed tabs, for 닫은 탭 다시 열기 (Ctrl+Shift+T). */
const closedTabs: Target[] = [];
/** The last time a click showed a document in place of another (see openDocInNewTab). */
let lastReplace: { tab: string; at: number; prev: Target } | null = null;

const LAYOUT_KEY = 'wp.layout.';

function withPane(panes: Pane[], index: number, pane: Pane): Pane[] {
  return panes.map((p, i) => (i === index ? pane : p));
}

/** Sets the panes and everything that follows from them. */
function commit(panes: Pane[], focus = get().focus, split = get().split) {
  const f = Math.min(Math.max(focus, 0), Math.max(panes.length - 1, 0));
  const tab = activeTab(panes[f]);
  const target = tab?.target ?? null;
  const editor = tab ? (tabEditors.get(tab.key) ?? null) : null;
  const editorChanged = editor !== get().editor;
  const docId = target?.kind === 'doc' ? target.id : null;
  set({
    panes,
    focus: f,
    split,
    activeTarget: target,
    activeDocId: docId,
    // Opening another chapter moves 새 회차 to that chapter's part.
    ...(docId && docId !== get().activeDocId ? { selectedPartId: null } : {}),
    activeCardId: target?.kind === 'card' ? target.id : null,
    editor,
    ...(editorChanged
      ? { liveCounts: editor ? countBlocks(blocksFromNode(editor.state.doc)) : null, selection: null }
      : {}),
  });
  const ov = get().overview;
  if (!ov) return;
  if (target?.kind === 'doc') rememberDoc(ov.project.id, target.id);
  try {
    localStorage.setItem(LAYOUT_KEY + ov.project.id, JSON.stringify(toSaved(panes, f, split)));
  } catch {
    // Not critical.
  }
}

function targetExists(ov: Overview, t: Target): boolean {
  switch (t.kind) {
    case 'doc':
      return findDoc(ov, t.id) !== null;
    case 'card':
      return ov.cards.some((c) => c.id === t.id);
    case 'table':
      return ov.parts.some((p) => p.id === t.id);
    case 'notes':
    case 'web':
      return true;
  }
}

function initialLayout(ov: Overview): { panes: Pane[]; focus: number; split: SplitDir } {
  try {
    const raw = localStorage.getItem(LAYOUT_KEY + ov.project.id);
    const saved = raw ? fromSaved(JSON.parse(raw), (t) => targetExists(ov, t)) : null;
    if (saved) return saved;
  } catch {
    // Start fresh.
  }
  const doc = initialDoc(ov);
  return { panes: [makePane(doc ? [{ kind: 'doc', id: doc }] : [])], focus: 0, split: 'row' };
}

/** Closes tabs showing something that is gone, in every pane. */
function dropEverywhere(gone: (t: Target) => boolean, fallback: Target | null) {
  const panes = get().panes.map((p) => dropTargets(p, gone, fallback));
  const kept = panes.filter((p) => p.tabs.length);
  commit(panes.length > 1 && kept.length ? kept : panes.slice(0, 1), get().focus);
}

/** A document tab tells when its editor is ready (`present`) and when it goes. */
export function registerEditor(tabKey: string, editor: Editor, present: boolean) {
  if (present) tabEditors.set(tabKey, editor);
  else if (tabEditors.get(tabKey) === editor) tabEditors.delete(tabKey);
  const { panes, focus } = get();
  if (activeTab(panes[focus])?.key === tabKey) commit(panes, focus);
}

/** True for the tab the writer is working in (its counts go to the status bar). */
export function isFocusedTab(tabKey: string): boolean {
  const { panes, focus } = get();
  return activeTab(panes[focus])?.key === tabKey;
}

/**
 * Shows something in the focused pane. A document replaces the document in
 * the open tab (its 뒤로 remembers it); anything else, or `newTab`, gets a
 * tab of its own. Something already open in a tab of that pane is shown there.
 */
export async function openTarget(target: Target, opts: { newTab?: boolean } = {}) {
  const { panes, focus } = get();
  const pane = panes[focus];
  if (!pane) {
    commit([makePane([target])], 0);
    return;
  }
  const existing = pane.tabs.find((t) => sameTarget(t.target, target));
  if (existing) {
    if (pane.active !== existing.key) commit(withPane(panes, focus, { ...pane, active: existing.key }), focus);
    return;
  }
  const current = activeTab(pane);
  if (opts.newTab || !current || current.target.kind !== 'doc' || target.kind !== 'doc') {
    commit(withPane(panes, focus, openTab(pane, target)), focus);
    return;
  }
  // The document shown now goes away: make sure it is saved first.
  if (!(await saveEverything())) return;
  const now = get();
  const p = now.panes[now.focus];
  const cur = activeTab(p);
  if (!p || !cur || cur.key !== current.key) return;
  const other = p.tabs.find((t) => sameTarget(t.target, target));
  if (other) {
    commit(withPane(now.panes, now.focus, { ...p, active: other.key }), now.focus);
    return;
  }
  lastReplace = { tab: cur.key, at: Date.now(), prev: cur.target };
  commit(
    withPane(now.panes, now.focus, { ...p, tabs: p.tabs.map((t) => (t.key === cur.key ? navigate(t, target) : t)) }),
  );
}

/**
 * Double click in the tree: a new tab. Its first click already showed the
 * document in place of another; that one goes back to its tab.
 */
export async function openDocInNewTab(id: string) {
  const target: Target = { kind: 'doc', id };
  const { panes, focus } = get();
  const pane = panes[focus];
  const cur = activeTab(pane);
  const last = lastReplace;
  if (pane && cur && last && last.tab === cur.key && Date.now() - last.at < 1500 && sameTarget(cur.target, target)) {
    lastReplace = null;
    const restored = { ...cur, target: last.prev, back: cur.back.slice(0, -1) };
    const next = openTab({ ...pane, tabs: pane.tabs.map((t) => (t.key === cur.key ? restored : t)) }, target);
    commit(withPane(panes, focus, next), focus);
    return;
  }
  await openTarget(target, { newTab: true });
}

export function activateTab(paneIndex: number, key: string) {
  const { panes } = get();
  const pane = panes[paneIndex];
  if (!pane?.tabs.some((t) => t.key === key)) return;
  commit(withPane(panes, paneIndex, { ...pane, active: key }), paneIndex);
}

/** Ctrl+Tab / Ctrl+Shift+Tab in the focused pane. */
export function cycleTab(dir: 1 | -1) {
  const { panes, focus } = get();
  const pane = panes[focus];
  if (!pane || pane.tabs.length < 2) return;
  const i = pane.tabs.findIndex((t) => t.key === pane.active);
  const next = pane.tabs[(i + dir + pane.tabs.length) % pane.tabs.length];
  activateTab(focus, next.key);
}

export function reorderTab(paneIndex: number, key: string, before: string | null) {
  const { panes } = get();
  const pane = panes[paneIndex];
  if (pane) commit(withPane(panes, paneIndex, moveTab(pane, key, before)));
}

export async function closeTab(paneIndex: number, key: string) {
  if (!(await saveEverything())) return;
  const { panes, focus } = get();
  const pane = panes[paneIndex];
  const tab = pane?.tabs.find((t) => t.key === key);
  if (!pane || !tab) return;
  closedTabs.push(tab.target);
  if (closedTabs.length > 20) closedTabs.shift();
  const next = removeTab(pane, key);
  if (!next.tabs.length && panes.length > 1) {
    commit(
      panes.filter((_, i) => i !== paneIndex),
      0,
    );
    return;
  }
  commit(withPane(panes, paneIndex, next), focus);
}

export function closeActiveTab() {
  const { panes, focus } = get();
  const tab = activeTab(panes[focus]);
  if (tab) void closeTab(focus, tab.key);
}

export async function reopenClosedTab() {
  const ov = get().overview;
  while (ov && closedTabs.length) {
    const target = closedTabs.pop()!;
    if (targetExists(ov, target)) {
      await openTarget(target, { newTab: true });
      return;
    }
  }
}

/** 뒤로 (-1) / 앞으로 (+1) in the focused tab. */
export async function goBack(dir: -1 | 1 = -1) {
  const ov = get().overview;
  const { panes, focus } = get();
  const tab = activeTab(panes[focus]);
  if (!ov || !tab) return;
  const moved = step(tab, dir, (t) => targetExists(ov, t));
  if (!moved) return;
  if (!(await saveEverything())) return;
  const now = get();
  const pane = now.panes[now.focus];
  if (!pane || activeTab(pane)?.key !== tab.key) return;
  commit(withPane(now.panes, now.focus, { ...pane, tabs: pane.tabs.map((t) => (t.key === tab.key ? moved : t)) }));
}

/** Shows something else in the focused tab (개요 표: another part). */
export function showInTab(target: Target) {
  const { panes, focus } = get();
  const pane = panes[focus];
  const tab = activeTab(pane);
  if (!pane || !tab || sameTarget(tab.target, target)) return;
  commit(withPane(panes, focus, { ...pane, tabs: pane.tabs.map((t) => (t.key === tab.key ? navigate(t, target) : t)) }));
}

export function focusPane(index: number) {
  const { panes, focus } = get();
  if (index !== focus && panes[index]) commit(panes, index);
}

/** Splits the middle in two (the new pane starts with what is open), or changes the direction. */
export function splitView(dir: SplitDir) {
  const { panes, focus } = get();
  if (panes.length > 1) {
    commit(panes, focus, dir);
    return;
  }
  const tab = activeTab(panes[0]);
  commit([...panes, makePane(tab ? [tab.target] : [])], 1, dir);
}

/** Back to one pane; tabs of the other one move over to it. */
export async function unsplit() {
  if (get().panes.length < 2) return;
  if (!(await saveEverything())) return;
  const { panes, focus } = get();
  const kept = panes[focus];
  if (!kept) return;
  let merged = kept;
  for (const [i, pane] of panes.entries()) {
    if (i === focus) continue;
    for (const tab of pane.tabs) {
      if (!merged.tabs.some((t) => sameTarget(t.target, tab.target))) merged = { ...merged, tabs: [...merged.tabs, tab] };
    }
  }
  commit([{ ...merged, locked: false }], 0);
}

export function toggleLock(paneIndex: number) {
  const { panes } = get();
  const pane = panes[paneIndex];
  if (pane) commit(withPane(panes, paneIndex, { ...pane, locked: !pane.locked }));
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
  stopAutoSync();
  tabEditors.clear();
  closedTabs.length = 0;
  lastReplace = null;
  set({
    overview: ov,
    previewCardId: null,
    cardCounts: {},
    notes: [],
    focusNoteId: null,
    pendingNote: null,
    selectedPartId: null,
    conflicts: {},
    cardReloads: {},
    link: null,
    save: { state: 'saved' },
    liveCounts: null,
    selection: null,
  });
  const layout = initialLayout(ov);
  commit(layout.panes, layout.focus, layout.split);
  api.setWindowTitle(`${ov.project.title} · WriterProgram`);
  startWatching(ov.root);
  void loadLink(ov);
  void loadCatalog();
  void refreshCardCounts();
  void loadNotes();
  if (ov.copies.length) {
    showToast({
      text: `다른 기기에서 생긴 사본 ${ov.copies.length}개가 있음`,
      action: { label: '살펴보기', run: () => openDialog({ kind: 'copies' }) },
    });
  }
}

export async function loadCatalog() {
  try {
    set({ catalog: await api.formatCatalog() });
  } catch (e) {
    toastError('원고 서식 목록을 읽지 못함', e);
  }
}

/** Saves project settings and reloads the overview (page estimates depend on the format). */
export async function updateProject(patch: ProjectPatch): Promise<boolean> {
  try {
    const project = await api.projectUpdate(root(), patch);
    const ov = get().overview;
    if (ov) set({ overview: { ...ov, project } });
    await refreshOverview();
    api.setWindowTitle(`${project.title} · WriterProgram`);
    return true;
  } catch (e) {
    toastError('작품 설정을 저장하지 못함', e);
    return false;
  }
}

export async function applyFormat(format: ManuscriptFormat) {
  return updateProject({ manuscriptFormat: format });
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
  stopWatching();
  stopAutoSync();
  set({
    link: null,
    overview: null,
    panes: [],
    focus: 0,
    activeTarget: null,
    activeDocId: null,
    activeCardId: null,
    editor: null,
    liveCounts: null,
    selection: null,
    dialog: null,
    previewCardId: null,
  });
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

/** Opens a document in the tab being worked in, or in a new tab. */
export function selectDoc(id: string, newTab = false) {
  return openTarget({ kind: 'doc', id }, { newTab });
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
    // Its tabs close; a pane left empty shows a neighbour from the same list:
    // the next one, or the one before.
    const list = findDoc(ov, id)?.section === 'planning' ? ov.planning : allManuscript(ov);
    const i = list.findIndex((d) => d.id === id);
    const next = list[i + 1] ?? list[i - 1] ?? null;
    dropEverywhere((t) => t.kind === 'doc' && t.id === id, next ? { kind: 'doc', id: next.id } : null);
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

// ---------------------------------------------------------------------------
// Setting cards

export function previewCard(id: string | null) {
  set({ previewCardId: id, rightOpen: id ? true : get().rightOpen });
}

/** Opens a card for editing in a tab of its own. */
export async function openCard(id: string) {
  set({ previewCardId: null });
  await openTarget({ kind: 'card', id }, { newTab: true });
  void refreshCardCounts();
}

export async function refreshCardCounts() {
  try {
    const counts = await api.cardCounts(root());
    set({ cardCounts: Object.fromEntries(counts) });
  } catch {
    // Counts are a nicety; the list works without them.
  }
}

export async function createCard(typeId: string, name: string) {
  try {
    const card = await api.cardCreate(root(), typeId, name);
    await refreshOverview();
    await openCard(card.id);
  } catch (e) {
    toastError('카드를 만들지 못함', e);
  }
}

/** Replaces one card's line in the overview after it was saved. */
export function patchCardSummary(summary: CardSummary) {
  const ov = get().overview;
  if (!ov) return;
  const cards = ov.cards.some((c) => c.id === summary.id)
    ? ov.cards.map((c) => (c.id === summary.id ? summary : c))
    : [...ov.cards, summary];
  cards.sort((a, b) => a.name.localeCompare(b.name));
  set({ overview: { ...ov, cards } });
}

export async function trashCard(id: string) {
  const ov = get().overview;
  if (!ov) return;
  try {
    if (!(await saveEverything())) return;
    const item = await api.cardTrash(ov.root, id);
    dropEverywhere((t) => t.kind === 'card' && t.id === id, null);
    if (get().previewCardId === id) set({ previewCardId: null });
    await refreshOverview();
    showToast({
      text: '카드를 휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void api.trashRestore(ov.root, item.id).then(refreshOverview);
        },
      },
    });
  } catch (e) {
    toastError('휴지통으로 옮기지 못함', e);
  }
}

// ---------------------------------------------------------------------------
// Notes (메모)

export async function loadNotes() {
  try {
    set({ notes: await api.noteList(root()) });
  } catch (e) {
    toastError('메모를 읽지 못함', e);
  }
}

/** Shows a note in the 메모 tab (and marks its text). */
export function focusNote(id: string | null) {
  if (!id) {
    set({ focusNoteId: null });
    return;
  }
  set({ focusNoteId: id, rightOpen: true, rightTab: 'notes', previewCardId: null });
}

/** Replaces one note after it was saved. */
export function patchNote(note: Note) {
  const notes = get().notes;
  set({ notes: notes.some((n) => n.id === note.id) ? notes.map((n) => (n.id === note.id ? note : n)) : [...notes, note] });
}

export async function addNote(spec: NewNote): Promise<Note | null> {
  try {
    const note = await api.noteCreate(root(), spec);
    set({ notes: [...get().notes, note] });
    focusNote(note.id);
    return note;
  } catch (e) {
    toastError('메모를 만들지 못함', e);
    return null;
  }
}

/** A note on the text selected in the open document (Ctrl+Alt+M, 메모 button). */
export async function addTextNote() {
  const { editor, activeDocId } = get();
  if (!editor || !activeDocId || !editor.isEditable) return;
  const { from, to, empty } = editor.state.selection;
  if (empty) {
    showToast({ text: '메모를 붙일 글을 먼저 고르세요' });
    return;
  }
  // Mark first, so the note follows the text even while it is being made.
  const id = newId();
  const quote = editor.state.doc.textBetween(from, to, ' ');
  addMark(editor, id, { from, to });
  const note = await addNote({ id, anchor: 'text', target: activeDocId, quote });
  if (!note) for (const e of editorsOf(activeDocId)) removeMarks(e, id);
}

/** Opens what a note is on: its document with the text selected, or its card. */
export async function showNote(note: Note) {
  if (note.anchor === 'text' || note.anchor === 'doc') {
    if (note.anchor === 'text') set({ pendingNote: note.id });
    await selectDoc(note.target);
  } else if (note.anchor === 'card') {
    await openCard(note.target);
  }
  focusNote(note.id);
}

export async function trashNote(id: string) {
  const ov = get().overview;
  const note = get().notes.find((n) => n.id === id);
  if (!ov || !note) return;
  try {
    if (note.anchor === 'text') {
      // The open document saves without the mark before the note goes.
      for (const e of editorsOf(note.target)) removeMarks(e, id);
      await flushAll();
    }
    const item = await api.noteTrash(ov.root, id);
    set({
      notes: get().notes.filter((n) => n.id !== id),
      focusNoteId: get().focusNoteId === id ? null : get().focusNoteId,
    });
    void refreshOverview();
    showToast({
      text: '메모를 휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void api
            .trashRestore(ov.root, item.id)
            .then(() => Promise.all([loadNotes(), refreshOverview()]))
            .catch((e: unknown) => toastError('되돌리지 못함', e));
        },
      },
    });
  } catch (e) {
    toastError('메모를 휴지통으로 옮기지 못함', e);
  }
}

/** Open notes on each document, for the tree. */
export function openNoteCounts(notes: Note[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const n of notes) {
    if (n.done || (n.anchor !== 'text' && n.anchor !== 'doc')) continue;
    counts.set(n.target, (counts.get(n.target) ?? 0) + 1);
  }
  return counts;
}

/** Picks a part in the tree as the place for 새 회차. */
export function selectPart(id: string | null) {
  set({ selectedPartId: id });
}

/** Where 새 회차 goes: the part picked in the tree, or else the open chapter's part. */
export function newDocPartId(): string | undefined {
  const { overview: ov, selectedPartId, activeDocId } = get();
  if (!ov) return undefined;
  if (selectedPartId && ov.parts.some((p) => p.id === selectedPartId)) return selectedPartId;
  return ov.parts.find((p) => p.docs.some((d) => d.id === activeDocId))?.id;
}

export function openNotesBoard() {
  return openTarget({ kind: 'notes', id: 'all' });
}

export function openTable(partId: string) {
  return openTarget({ kind: 'table', id: partId });
}

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

function startWatching(root: string) {
  stopWatch?.();
  stopWatch = api.watchProject(root, (changes) => onChanges(root, changes));
}

function stopWatching() {
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

// ---------------------------------------------------------------------------
// Keeping a project in step with a drive (crates/sync, app drives.rs)
//
// A linked project is brought in step when it opens, every minute while it
// is open, and when the window comes back to the front. Changes that arrive
// reach open documents through the folder watcher, like a sync program's.

const SYNC_EVERY_MS = 60_000;
let syncTimer: ReturnType<typeof setInterval> | undefined;
const onFocus = () => void syncNow({ quiet: true });

async function loadLink(ov: Overview) {
  const link = await api.projectLinkGet(ov.project.id).catch(() => null);
  if (get().overview?.root !== ov.root) return;
  set({ link });
  if (link) startAutoSync();
}

function startAutoSync() {
  stopAutoSync();
  void syncNow({ quiet: true });
  syncTimer = setInterval(() => void syncNow({ quiet: true }), SYNC_EVERY_MS);
  window.addEventListener('focus', onFocus);
}

function stopAutoSync() {
  clearInterval(syncTimer);
  syncTimer = undefined;
  window.removeEventListener('focus', onFocus);
}

/** One pass with the drive now. Quiet passes only speak up about problems in the status bar. */
export async function syncNow(opts: { quiet?: boolean } = {}) {
  const ov = get().overview;
  if (!ov || !get().link || get().syncing) return;
  // Edits waiting to be saved go first, so they travel in this pass.
  if (!(await saveEverything())) return;
  set({ syncing: true });
  try {
    const out = await api.projectSync(ov.root, ov.project.id);
    if (get().overview?.root !== ov.root) return;
    set({ link: out.link });
    const r = out.report;
    if (r && (r.downloaded.length || r.removedHere.length || r.copies.length || r.merged)) {
      await refreshOverview();
      void loadNotes();
      void refreshCardCounts();
    }
    if (!opts.quiet) {
      showToast(
        out.link.error
          ? { text: `드라이브와 맞추지 못함 · ${out.link.error}`, tone: 'error' }
          : { text: '드라이브와 맞춤' },
      );
    }
  } catch (e) {
    if (!opts.quiet) toastError('드라이브와 맞추지 못함', e);
  } finally {
    set({ syncing: false });
  }
}

/** Starts keeping the open project in step with a drive. */
export async function linkProject(provider: DriveProvider): Promise<boolean> {
  const ov = get().overview;
  if (!ov) return false;
  try {
    const link = await api.projectLink(ov.project.id, ov.project.title, provider);
    set({ link });
    startAutoSync();
    return true;
  } catch (e) {
    toastError('드라이브와 맞추기를 시작하지 못함', e);
    return false;
  }
}

/** Stops keeping the open project in step (nothing is removed anywhere). */
export async function unlinkProject() {
  const ov = get().overview;
  if (!ov) return;
  try {
    await api.projectUnlink(ov.project.id);
    stopAutoSync();
    set({ link: null });
  } catch (e) {
    toastError('그만두지 못함', e);
  }
}

// ---------------------------------------------------------------------------
// In-app browser (prototype): web pages in tabs of the middle column.

const WEB_KEY = 'wp.web.';

export interface WebPage {
  url: string;
  title: string;
}

/** The last page of each browser tab, per project. */
export function webPages(projectId: string): Record<string, WebPage> {
  try {
    return JSON.parse(localStorage.getItem(WEB_KEY + projectId) ?? '{}') as Record<string, WebPage>;
  } catch {
    return {};
  }
}

export function rememberWebPage(projectId: string, id: string, page: Partial<WebPage>) {
  const all = webPages(projectId);
  all[id] = { url: all[id]?.url ?? '', title: all[id]?.title ?? '', ...page };
  try {
    localStorage.setItem(WEB_KEY + projectId, JSON.stringify(all));
  } catch {
    // Not critical.
  }
  set({ webVersion: get().webVersion + 1 });
}

/** Opens a new browser tab in the pane being worked in. */
export function openWeb(url = '') {
  const ov = get().overview;
  const id = newId();
  if (ov && url) rememberWebPage(ov.project.id, id, { url });
  return openTarget({ kind: 'web', id }, { newTab: true });
}

/** 자료로 보관 (prototype): the page goes to 메모함 as a project note. */
export async function clipPage(label: string) {
  try {
    const clip = await api.browserClip(label);
    const lines = [clip.title.trim() || clip.url, clip.url];
    if (clip.text.trim()) lines.push('', `“${clip.text.trim()}”`);
    const note = await addNote({ anchor: 'project', text: lines.join('\n') });
    if (note) showToast({ text: '메모함에 보관함' });
  } catch (e) {
    toastError('보관하지 못함', e);
  }
}
