// Creation journal (창작 일지) from the editor: writing sessions and large
// pastes, sent to the Rust side (crates/core/src/journal.rs). Counts and
// times only, never text (docs/creation-proof.md §3).
//
// A session runs per document from its first edit until five minutes pass
// without one; it also ends when the project or the window closes, or the
// app is hidden (a phone may stop it there). A paste counts as from inside
// the app when its text is the same as something copied or cut in the app
// shortly before.

import type { Node, Slice } from '@tiptap/pm/model';
import type { Transaction } from '@tiptap/pm/state';
import { ReplaceAroundStep, ReplaceStep } from '@tiptap/pm/transform';
import { api } from '../api';
import { registerCloser } from '../lib/flush';

/** A session ends after this long without an edit. */
export const SESSION_IDLE_MS = 5 * 60 * 1000;
/** Shorter pastes are not worth a line (same as `journal::PASTE_MIN_CHARS`). */
export const PASTE_MIN_CHARS = 100;
/** How many recent in-app copies a paste is compared with. */
const COPIES_KEPT = 10;
/** A paste event older than this belongs to no transaction. */
const PASTE_EVENT_MS = 2000;

interface Session {
  root: string;
  doc: string;
  start: string;
  end: string;
  inserted: number;
  deleted: number;
  timer: ReturnType<typeof setTimeout>;
}

const open = new Map<string, Session>();
let on = true;

/** Follows this device's setting; turning it off drops sessions under way. */
export function setJournalOn(value: boolean) {
  on = value;
  if (!value) {
    for (const s of open.values()) clearTimeout(s.timer);
    open.clear();
  }
}

/** Characters, as `count.rs` counts them (line breaks do not count). */
function chars(text: string): number {
  let n = 0;
  for (const c of text) if (c !== '\n') n += 1;
  return n;
}

function textIn(doc: Node, from: number, to: number): number {
  return from < to ? chars(doc.textBetween(from, to, '', '')) : 0;
}

function textOf(slice: Slice): number {
  return chars(slice.content.textBetween(0, slice.content.size, '', ''));
}

/**
 * Characters a transaction put in and took out. While an input method is
 * composing (Korean), each jamo replaces the syllable before it, so only the
 * change in length counts: 한 typed as ㅎ → 하 → 한 is one character in.
 */
export function editCounts(tr: Transaction, composing = false): { inserted: number; deleted: number } {
  let inserted = 0;
  let deleted = 0;
  tr.steps.forEach((step, i) => {
    const before = tr.docs[i];
    if (step instanceof ReplaceStep) {
      deleted += textIn(before, step.from, step.to);
      inserted += textOf(step.slice);
    } else if (step instanceof ReplaceAroundStep) {
      // Wrapping or unwrapping keeps the text between gapFrom and gapTo.
      deleted += textIn(before, step.from, step.gapFrom) + textIn(before, step.gapTo, step.to);
      inserted += textOf(step.slice);
    }
  });
  if (composing || tr.getMeta('composition') != null) {
    const net = inserted - deleted;
    return { inserted: Math.max(net, 0), deleted: Math.max(-net, 0) };
  }
  return { inserted, deleted };
}

// ---------------------------------------------------------------------------
// Copies made in the app, and the paste under way

const copies: string[] = [];
let lastPaste: { outside: boolean; at: number } | null = null;

/** Text compared without spaces and line breaks (copies keep them differently). */
function squeeze(text: string): string {
  return text.replace(/\s+/g, '');
}

/** Remembers text copied or cut in the app (also the 클립보드 export). */
export function noteInAppCopy(text: string) {
  const t = squeeze(text);
  if (!t) return;
  const at = copies.indexOf(t);
  if (at >= 0) copies.splice(at, 1);
  copies.unshift(t);
  copies.length = Math.min(copies.length, COPIES_KEPT);
}

/** Whether pasted text was copied or cut in the app shortly before. */
export function fromInside(text: string): boolean {
  const t = squeeze(text);
  return !!t && copies.includes(t);
}

function sessionKey(root: string, doc: string) {
  return `${root}\n${doc}`;
}

async function send(root: string, event: Parameters<typeof api.journalEvent>[1]) {
  try {
    await api.journalEvent(root, event);
  } catch {
    // The journal never gets in the way of writing.
  }
}

function end(key: string): Promise<void> {
  const s = open.get(key);
  if (!s) return Promise.resolve();
  clearTimeout(s.timer);
  open.delete(key);
  return send(s.root, { kind: 'session', doc: s.doc, start: s.start, end: s.end, inserted: s.inserted, deleted: s.deleted });
}

/** Ends every session under way (the project or the window is closing). */
export function endSessions(): Promise<void> {
  return Promise.all([...open.keys()].map(end)).then(() => {});
}

/** Counts an edit the writer made in document `doc` (not one passed on or reloaded). */
export function noteEdit(root: string, doc: string, tr: Transaction, composing = false) {
  if (!on) return;
  const { inserted, deleted } = editCounts(tr, composing);
  if (!inserted && !deleted) return;
  const key = sessionKey(root, doc);
  const now = new Date().toISOString();
  const timer = setTimeout(() => void end(key), SESSION_IDLE_MS);
  const s = open.get(key);
  if (s) {
    clearTimeout(s.timer);
    s.inserted += inserted;
    s.deleted += deleted;
    s.end = now;
    s.timer = timer;
  } else {
    open.set(key, { root, doc, start: now, end: now, inserted, deleted, timer });
  }

  const pasted = tr.getMeta('paste') === true || tr.getMeta('uiEvent') === 'paste';
  if (pasted && inserted >= PASTE_MIN_CHARS) {
    const recent = lastPaste && Date.now() - lastPaste.at < PASTE_EVENT_MS ? lastPaste : null;
    void send(root, { kind: 'paste', doc, chars: inserted, outside: recent?.outside ?? true });
  }
  if (pasted) lastPaste = null;
}

let installed = false;

/**
 * Listens for copies, cuts and pastes in the window, ends sessions when the
 * app is hidden or closes, and joins the closers in lib/flush.ts. Once.
 */
export function installJournal() {
  if (installed || typeof document === 'undefined') return;
  installed = true;
  const copied = () => noteInAppCopy(document.getSelection()?.toString() ?? '');
  document.addEventListener('copy', copied, true);
  document.addEventListener('cut', copied, true);
  document.addEventListener(
    'paste',
    (e) => {
      const text = e.clipboardData?.getData('text/plain') ?? '';
      lastPaste = { outside: !fromInside(text), at: Date.now() };
    },
    true,
  );
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') void endSessions();
  });
  window.addEventListener('pagehide', () => void endSessions());
  registerCloser(endSessions);
}
