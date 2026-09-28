// Tabs and split panes in the middle column (docs/layout-data.md "가운데 탭",
// "분할"). Plain data and pure helpers; the store owns the state.

/** What a tab shows: a document, a setting card, 메모함 (id "all"), the
 * 개요 표 of a part (id = the part), or a web page (id = the browser tab's). */
export type Target =
  | { kind: 'doc'; id: string }
  | { kind: 'card'; id: string }
  | { kind: 'notes'; id: 'all' }
  | { kind: 'table'; id: string }
  | { kind: 'web'; id: string };

const KINDS = ['doc', 'card', 'notes', 'table', 'web'];

export interface Tab {
  /** Stays the same while the tab shows other things (React key). */
  key: string;
  target: Target;
  /** Earlier and later things shown in this tab (뒤로 / 앞으로). */
  back: Target[];
  forward: Target[];
}

export interface Pane {
  key: string;
  tabs: Tab[];
  active: string | null;
  /** 읽기 전용 잠금: nothing in this pane can be changed. */
  locked: boolean;
}

export type SplitDir = 'row' | 'column';

const HISTORY_LIMIT = 50;

let seq = 0;
function newKey(): string {
  seq += 1;
  return `${Date.now().toString(36)}-${seq.toString(36)}`;
}

export function sameTarget(a: Target, b: Target): boolean {
  return a.kind === b.kind && a.id === b.id;
}

export function makeTab(target: Target): Tab {
  return { key: newKey(), target, back: [], forward: [] };
}

export function makePane(targets: Target[] = [], locked = false): Pane {
  const tabs = targets.map(makeTab);
  return { key: newKey(), tabs, active: tabs[0]?.key ?? null, locked };
}

export function activeTab(pane: Pane | undefined): Tab | null {
  return pane?.tabs.find((t) => t.key === pane.active) ?? null;
}

/** Shows `target` in the tab, remembering what it showed for 뒤로. */
export function navigate(tab: Tab, target: Target): Tab {
  return { ...tab, target, back: [...tab.back, tab.target].slice(-HISTORY_LIMIT), forward: [] };
}

/** One step back (-1) or forward (+1), skipping things that are gone. */
export function step(tab: Tab, dir: -1 | 1, exists: (t: Target) => boolean): Tab | null {
  const back = [...tab.back];
  const forward = [...tab.forward];
  let current = tab.target;
  const from = dir < 0 ? back : forward;
  const to = dir < 0 ? forward : back;
  while (from.length) {
    const next = dir < 0 ? from.pop()! : from.shift()!;
    if (!exists(next)) continue;
    if (dir < 0) to.unshift(current);
    else to.push(current);
    current = next;
    return { ...tab, target: current, back, forward };
  }
  return null;
}

/** Adds a tab after the active one and makes it active. */
export function openTab(pane: Pane, target: Target): Pane {
  const tab = makeTab(target);
  const at = pane.tabs.findIndex((t) => t.key === pane.active);
  const tabs = [...pane.tabs];
  tabs.splice(at < 0 ? tabs.length : at + 1, 0, tab);
  return { ...pane, tabs, active: tab.key };
}

/** Removes a tab; the one after it (or else before it) becomes active. */
export function removeTab(pane: Pane, key: string): Pane {
  const i = pane.tabs.findIndex((t) => t.key === key);
  if (i < 0) return pane;
  const tabs = pane.tabs.filter((t) => t.key !== key);
  const active = pane.active === key ? (tabs[i] ?? tabs[i - 1])?.key ?? null : pane.active;
  return { ...pane, tabs, active };
}

/** Moves tab `key` to where tab `before` is (or to the end). */
export function moveTab(pane: Pane, key: string, before: string | null): Pane {
  const tab = pane.tabs.find((t) => t.key === key);
  if (!tab || key === before) return pane;
  const tabs = pane.tabs.filter((t) => t.key !== key);
  const at = before ? tabs.findIndex((t) => t.key === before) : -1;
  tabs.splice(at < 0 ? tabs.length : at, 0, tab);
  return { ...pane, tabs };
}

/** Takes out every tab showing something that matches (a thing sent to the
 * trash); a pane left empty shows `fallback` if given. */
export function dropTargets(pane: Pane, gone: (t: Target) => boolean, fallback: Target | null): Pane {
  let next = pane;
  for (const tab of pane.tabs) {
    if (gone(tab.target)) next = removeTab(next, tab.key);
  }
  next = {
    ...next,
    tabs: next.tabs.map((t) => ({ ...t, back: t.back.filter((x) => !gone(x)), forward: t.forward.filter((x) => !gone(x)) })),
  };
  if (!next.tabs.length && pane.tabs.length && fallback) next = openTab(next, fallback);
  return next;
}

// Saving the layout -----------------------------------------------------------

export interface SavedLayout {
  panes: { tabs: Target[]; active: number; locked: boolean }[];
  focus: number;
  split: SplitDir;
}

export function toSaved(panes: Pane[], focus: number, split: SplitDir): SavedLayout {
  return {
    panes: panes.map((p) => ({
      tabs: p.tabs.map((t) => t.target),
      active: Math.max(0, p.tabs.findIndex((t) => t.key === p.active)),
      locked: p.locked,
    })),
    focus,
    split,
  };
}

function isTarget(v: unknown): v is Target {
  if (!v || typeof v !== 'object') return false;
  const t = v as Record<string, unknown>;
  return KINDS.includes(t.kind as string) && typeof t.id === 'string';
}

/** Rebuilds saved panes, leaving out things that no longer exist. */
export function fromSaved(
  saved: unknown,
  exists: (t: Target) => boolean,
): { panes: Pane[]; focus: number; split: SplitDir } | null {
  if (!saved || typeof saved !== 'object') return null;
  const s = saved as Partial<SavedLayout>;
  if (!Array.isArray(s.panes)) return null;
  const panes: Pane[] = [];
  let focus = typeof s.focus === 'number' ? s.focus : 0;
  s.panes.slice(0, 2).forEach((p, i) => {
    const targets = Array.isArray(p?.tabs) ? p.tabs.filter(isTarget).filter(exists) : [];
    if (!targets.length) {
      if (i < focus) focus -= 1;
      return;
    }
    const pane = makePane(targets, p.locked === true);
    const active = pane.tabs[Math.min(Math.max(0, p.active ?? 0), pane.tabs.length - 1)];
    panes.push({ ...pane, active: active.key });
  });
  if (!panes.length) return null;
  return {
    panes,
    focus: Math.min(Math.max(0, focus), panes.length - 1),
    split: s.split === 'column' ? 'column' : 'row',
  };
}
