// Save state per target (docs/safety-design.md S7). Everything that saves on
// its own (a chapter's text `doc:<id>`, its title and synopsis `meta:<id>`, a
// card `card:<id>`, a note `note:<id>`) registers here and reports how each
// save went. The top bar shows the worst of them (error > saving > saved), so
// one target's success never hides another's failure.
//
// A failed save is tried again after 2, 5, 15, 30 and 60 seconds, then every
// 60 seconds. When a target keeps failing for over a minute, what it could
// not save is written to the app's data folder (비상 보관) so a crash or a
// closed window cannot take it. A target whose screen has gone away stays
// here until its last edit is saved.

import { api } from '../api';
import type { RescueContent } from '../api/types';
import { errorText } from '../lib/format';
import { saveReason } from '../lib/saveReason';
import { type SaveState, get, set } from './state';

/** Waits before each try after a failure; the last one repeats. */
export const RETRY_DELAYS = [2000, 5000, 15000, 30000, 60000];
/** Failing this long writes a rescue copy. */
export const RESCUE_AFTER = 60_000;
/** A rescue copy is written again at most this often while failures go on. */
const RESCUE_EVERY = 30_000;

/** The wait before the next try after `failures` failures in a row (from 1). */
export function retryDelay(failures: number): number {
  return RETRY_DELAYS[Math.min(Math.max(failures, 1), RETRY_DELAYS.length) - 1];
}

export interface SaveTarget {
  /** Sends the waiting edits again (the target's own flush). */
  retry: () => Promise<void>;
  /** True while edits wait to be sent (including ones whose save failed). */
  pending: () => boolean;
  /** What to keep in the rescue folder when saves keep failing. */
  rescue?: () => { item: string; content: RescueContent } | null;
}

interface Entry {
  targets: Set<SaveTarget>;
  /** Targets whose screen has gone away; kept until they have nothing to save. */
  detached: Set<SaveTarget>;
  failures: number;
  timer?: ReturnType<typeof setTimeout>;
  projectId: string | null;
  rescuedAt?: number;
}

const entries = new Map<string, Entry>();

function stateOf(key: string): SaveState | undefined {
  return get().saves[key];
}

function put(key: string, state: SaveState | null) {
  const saves = { ...get().saves };
  if (state) saves[key] = state;
  else delete saves[key];
  set({ saves });
}

function forget(key: string) {
  const entry = entries.get(key);
  if (entry) clearTimeout(entry.timer);
  entries.delete(key);
  put(key, null);
}

/** Drops targets that went away and have nothing left to save. */
function tidy(key: string) {
  const entry = entries.get(key);
  if (!entry) return;
  for (const t of entry.detached) {
    if (!t.pending()) {
      entry.detached.delete(t);
      entry.targets.delete(t);
    }
  }
  if (entry.targets.size === 0 && stateOf(key)?.state !== 'error') forget(key);
}

/** Registers a target; the returned function is called when its screen goes away. */
export function registerSave(key: string, target: SaveTarget): () => void {
  let entry = entries.get(key);
  if (!entry) {
    entry = { targets: new Set(), detached: new Set(), failures: 0, projectId: get().overview?.project.id ?? null };
    entries.set(key, entry);
  }
  entry.targets.add(target);
  return () => {
    const e = entries.get(key);
    if (!e || !e.targets.has(target)) return;
    e.detached.add(target);
    tidy(key);
  };
}

/** Edits are waiting to be saved. An error stays shown until a save succeeds. */
export function markSaving(key: string) {
  const now = stateOf(key);
  if (now?.state === 'error' || now?.state === 'saving') return;
  put(key, { state: 'saving' });
}

/** Everything of this target is on disk. */
export function markSaved(key: string) {
  const entry = entries.get(key);
  if (entry) {
    clearTimeout(entry.timer);
    entry.timer = undefined;
    entry.failures = 0;
    entry.rescuedAt = undefined;
  }
  put(key, { state: 'saved' });
  tidy(key);
  // A screen that went away may still hold edits of the same target (the
  // chapter closed and opened again while failing): send those too.
  const stale = entry ? [...entry.detached].filter((t) => t.pending()) : [];
  for (const t of stale) void t.retry();
}

/** A save failed: show why and try again later. */
export function markFailed(key: string, e: unknown) {
  const entry = entries.get(key);
  const before = stateOf(key);
  const now = Date.now();
  const since = before?.state === 'error' && before.since ? before.since : now;
  const state: SaveState = { state: 'error', error: saveReason(errorText(e)), since, rescued: before?.rescued };
  if (entry) {
    entry.failures += 1;
    const delay = retryDelay(entry.failures);
    clearTimeout(entry.timer);
    entry.timer = setTimeout(() => {
      entry.timer = undefined;
      void retryTargets(entry);
    }, delay);
    state.retryAt = now + delay;
  }
  put(key, state);
  if (entry && now - since >= RESCUE_AFTER && (!entry.rescuedAt || now - entry.rescuedAt >= RESCUE_EVERY)) {
    void rescue(key);
  }
}

function retryTargets(entry: Entry): Promise<void> {
  return Promise.all([...entry.targets].map((t) => t.retry())).then(() => undefined);
}

/** 지금 다시: tries every failing or waiting target now. */
export async function retryNow(): Promise<void> {
  const keys = new Set(unsavedKeys());
  const failing = [...entries.entries()].filter(([key]) => keys.has(key));
  await Promise.all(
    failing.map(([, entry]) => {
      clearTimeout(entry.timer);
      entry.timer = undefined;
      return retryTargets(entry);
    }),
  );
}

/**
 * Writes the rescue copy of one target. Answers the file's path, or null when
 * there was nothing to write or writing failed too.
 */
export async function rescue(key: string): Promise<string | null> {
  const entry = entries.get(key);
  const projectId = entry?.projectId ?? get().overview?.project.id;
  if (!entry || !projectId) return null;
  const since = stateOf(key)?.since ?? Date.now();
  let path: string | null = null;
  entry.rescuedAt = Date.now();
  for (const target of entry.targets) {
    const what = target.rescue?.();
    if (!what) continue;
    try {
      path = await api.rescueSave(projectId, what.item, since, what.content);
    } catch {
      // The app's own folder failed too; the error stays shown and the
      // close dialog keeps asking.
      return null;
    }
  }
  const now = stateOf(key);
  if (path && now?.state === 'error') {
    const first = !now.rescued;
    put(key, { ...now, rescued: path });
    if (first) {
      set({
        toast: {
          text: '작품 폴더에 저장하지 못해 비상 위치에 보관했습니다',
          tone: 'error',
          action: { label: '위치 열기', run: () => void openRescueFolder() },
        },
      });
    }
  }
  return path;
}

/** Writes rescue copies of every target that has something unsaved. True when all were written. */
export async function rescueAll(): Promise<boolean> {
  const keys = unsavedKeys();
  const paths = await Promise.all(keys.map((key) => rescue(key)));
  return paths.every((p) => p !== null);
}

export async function openRescueFolder() {
  const id = get().overview?.project.id;
  if (!id) return;
  try {
    await api.reveal(await api.rescueFolder(id));
  } catch (e) {
    set({ toast: { text: `위치를 열지 못함 · ${errorText(e)}`, tone: 'error' } });
  }
}

/** Targets that failed or still have edits waiting. */
export function unsavedKeys(): string[] {
  return [...entries.entries()]
    .filter(([key, entry]) => stateOf(key)?.state === 'error' || [...entry.targets].some((t) => t.pending()))
    .map(([key]) => key);
}

/** The state the top bar shows: the worst of all targets, and how many failed. */
export function worstSave(saves: Record<string, SaveState>): SaveState & { failing: number } {
  const all = Object.values(saves);
  const errors = all.filter((s) => s.state === 'error');
  if (errors.length) {
    // The one failing longest, with the soonest next try.
    const first = errors.reduce((a, b) => ((a.since ?? 0) <= (b.since ?? 0) ? a : b));
    const retryAt = Math.min(...errors.map((s) => s.retryAt ?? Infinity));
    const rescued = errors.find((s) => s.rescued)?.rescued;
    return { ...first, retryAt: Number.isFinite(retryAt) ? retryAt : undefined, rescued, failing: errors.length };
  }
  if (all.some((s) => s.state === 'saving')) return { state: 'saving', failing: 0 };
  return { state: 'saved', failing: 0 };
}

/** For tests: forgets every target. */
export function resetSaves() {
  for (const entry of entries.values()) clearTimeout(entry.timer);
  entries.clear();
  set({ saves: {} });
}
