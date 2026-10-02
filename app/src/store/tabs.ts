// Tabs and panes of the middle column (lib/tabs.ts).

import type { Editor } from '@tiptap/core';
import type { Overview } from '../api/types';
import { blocksFromNode, countBlocks } from '../editor/counts';
import { type Pane, type SplitDir, type Target, activeTab, dropTargets, fromSaved, makePane, moveTab, navigate, openTab, removeTab, sameTarget, step, toSaved } from '../lib/tabs';
import { get, set } from './state';
import { allManuscript, findDoc, saveEverything } from './ui';

// ---------------------------------------------------------------------------
// Tabs and panes (lib/tabs.ts)

// The document last open in each project, to open it again next time.
const LAST_DOC_KEY = 'wp.lastDoc.';

export function rememberDoc(projectId: string, docId: string) {
  try {
    localStorage.setItem(LAST_DOC_KEY + projectId, docId);
  } catch {
    // Not critical.
  }
}

export function initialDoc(ov: Overview): string | null {
  try {
    const last = localStorage.getItem(LAST_DOC_KEY + ov.project.id);
    if (last && findDoc(ov, last)) return last;
  } catch {
    // Fall through to the first chapter.
  }
  return allManuscript(ov)[0]?.id ?? ov.planning[0]?.id ?? null;
}

/** Editors of open document tabs, by tab key. */
const tabEditors = new Map<string, Editor>();
/** Closed tabs, for 닫은 탭 다시 열기 (Ctrl+Shift+T). */
const closedTabs: Target[] = [];
/** The last time a click showed a document in place of another (see openDocInNewTab). */
let lastReplace: { tab: string; at: number; prev: Target } | null = null;

const LAYOUT_KEY = 'wp.layout.';

/** Forgets the tabs of the project being left. */
export function resetTabs() {
  tabEditors.clear();
  closedTabs.length = 0;
  lastReplace = null;
}

function withPane(panes: Pane[], index: number, pane: Pane): Pane[] {
  return panes.map((p, i) => (i === index ? pane : p));
}

/** Sets the panes and everything that follows from them. */
export function commit(panes: Pane[], focus = get().focus, split = get().split) {
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

export function initialLayout(ov: Overview): { panes: Pane[]; focus: number; split: SplitDir } {
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
export function dropEverywhere(gone: (t: Target) => boolean, fallback: Target | null) {
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
