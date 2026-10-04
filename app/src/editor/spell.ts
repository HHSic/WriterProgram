// 맞춤법 표시: a red wavy line under words the dictionary does not know
// (crates/core/src/spell, checked on the Rust side off the screen's thread).
//
// Paragraphs are checked by their text. Results are kept per text, so only a
// paragraph that changed is sent again, and reopening a chapter shows its
// lines at once. After an edit only the paragraphs it touched are looked at
// (editor/decorate.ts); until a changed paragraph's answer comes back its old
// lines are only moved along. While a Korean syllable is being composed
// nothing is worked out again and no answer is put in (docs/safety-design.md
// K1); the word being typed gets no line until the cursor leaves it.

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, type EditorState, type Transaction } from '@tiptap/pm/state';
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view';
import type { SpellIssue } from '../api/types';
import { afterComposition } from './composition';
import { decorateAll, follow, joinSpans, type BlockDecorator, type Deferred, type Span } from './decorate';
import { blockText } from './search';

/** Checks paragraph texts; one list of issues per text. */
export type SpellCheck = (texts: string[]) => Promise<SpellIssue[][]>;

/** A flagged word in the document. */
export interface SpellHit {
  from: number;
  to: number;
  word: string;
}

export interface SpellSetup {
  check: SpellCheck;
  /** Right click (or long press) on a flagged word. */
  onMenu: (view: EditorView, hit: SpellHit, event: MouseEvent) => void;
}

/** Wait after the last change before asking, ms. */
const DELAY = 350;
/** Paragraphs and characters sent at once, so a long chapter fills in bit by bit. */
const BATCH_TEXTS = 400;
const BATCH_CHARS = 40_000;
/** Paragraph texts remembered; the oldest go first. */
const CACHE_LIMIT = 30_000;
/** More spans waiting than this are kept as the whole document. */
const MAX_WAITING = 2_000;
/** Waiting stretches longer than this (positions) are redrawn in one whole-document pass. */
const WHOLE_PASS = 20_000;

// Results for every editor, by paragraph text. Dropped when the words the
// writer accepts change (forgetSpelling).
const cache = new Map<string, SpellIssue[]>();
let knownKey = '';

/** Keeps the answer for a paragraph text (the checker below; tests too). */
export function rememberSpelling(text: string, issues: SpellIssue[]) {
  cache.delete(text);
  cache.set(text, issues);
  if (cache.size > CACHE_LIMIT) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
}

/**
 * Drops remembered results when the accepted words (card names, word lists,
 * ignored words) are not `key` any more. True when they changed.
 */
export function forgetSpelling(key: string): boolean {
  if (key === knownKey) return false;
  knownKey = key;
  cache.clear();
  return true;
}

export const spellKey = new PluginKey<SpellState>('spelling');

interface SpellState extends Deferred {
  setup: SpellSetup | null;
  /** Paragraphs not checked yet, to look at again when answers come. */
  waiting: Span[];
  /** Where the writer is typing: a flagged word touching it waits. */
  typing: number | null;
}

interface SpellMeta {
  /** Turn on with this setup, or off (null). */
  setup?: SpellSetup | null;
  /** Answers came in. */
  refresh?: true;
  /** Accepted words changed: check everything again, dropping lines on `accept` now. */
  recheck?: { accept?: string };
}

/** Texts to send, per editor, and what is on its way. */
interface Queue {
  texts: Set<string>;
  sent: Set<string>;
}

function issuesInto(issues: SpellIssue[], positions: number[], typing: number | null, out: Decoration[]) {
  for (const issue of issues) {
    if (issue.end > positions.length || issue.end <= issue.start) continue;
    const from = positions[issue.start];
    const to = positions[issue.end - 1] + 1;
    if (typing !== null && typing >= from && typing <= to) continue;
    out.push(Decoration.inline(from, to, { class: 'spell-miss' }, { spell: issue.word }));
  }
}

/**
 * Whole-document pass, block by block (quick on a long chapter): lines from
 * known answers; a paragraph without one keeps the lines it had in `old` and
 * is queued.
 */
function rebuild(doc: PmNode, old: DecorationSet, queue: Queue): { set: DecorationSet; missing: boolean } {
  // decorateAll visits text blocks in document order; on its quick path it
  // gives positions from each block's content (pos -1), so the block's own
  // start comes from here.
  const starts: number[] = [];
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    starts.push(pos);
    return false;
  });
  let next = 0;
  let missing = false;
  const decorate: BlockDecorator = (block, pos, out) => {
    const at = starts[next++] ?? pos;
    const shift = pos < 0 ? at + 1 : 0;
    const { text, positions } = blockText(block, pos);
    if (!text.trim()) return;
    const issues = cache.get(text);
    if (issues) {
      issuesInto(issues, positions, null, out);
      return;
    }
    if (!queue.sent.has(text)) queue.texts.add(text);
    missing = true;
    for (const d of old.find(at + 1, at + block.nodeSize - 1)) {
      out.push(Decoration.inline(d.from - shift, d.to - shift, { class: 'spell-miss' }, d.spec));
    }
  };
  return { set: decorateAll(doc, decorate), missing };
}

/**
 * The text blocks touching `spans` with their lines worked out again where
 * the answer is known; others keep the lines they had and are queued
 * (`missed` gets their spans).
 */
function respell(
  doc: PmNode,
  set: DecorationSet,
  spans: Span[],
  queue: Queue,
  typing: number | null,
  missed: Span[],
): DecorationSet {
  const done = new Set<number>();
  for (const span of spans) {
    const from = Math.max(0, span.from - 1);
    const to = Math.min(doc.content.size, span.to + 1);
    doc.nodesBetween(from, to, (node, pos) => {
      if (!node.isTextblock) return true;
      if (done.has(pos)) return false;
      done.add(pos);
      const { text, positions } = blockText(node, pos);
      const issues = text.trim() ? cache.get(text) : [];
      if (!issues) {
        if (!queue.sent.has(text)) queue.texts.add(text);
        missed.push({ from: pos, to: pos + node.nodeSize });
        return false;
      }
      set = set.remove(set.find(pos + 1, pos + node.nodeSize - 1));
      const out: Decoration[] = [];
      issuesInto(issues, positions, typing, out);
      if (out.length) set = set.add(doc, out);
      return false;
    });
  }
  return set;
}

function spanLength(spans: Span[]): number {
  return spans.reduce((n, s) => n + s.to - s.from, 0);
}

function whole(doc: PmNode): Span[] {
  return [{ from: 0, to: doc.content.size }];
}

function joinWaiting(doc: PmNode, spans: Span[]): Span[] {
  return spans.length > MAX_WAITING ? whole(doc) : joinSpans(spans);
}

function mapWaiting(waiting: Span[], tr: Transaction): Span[] {
  if (!tr.docChanged || !waiting.length) return waiting;
  return waiting.map((s) => ({ from: tr.mapping.map(s.from, -1), to: tr.mapping.map(s.to, 1) }));
}

const OFF: SpellState = { setup: null, decorations: DecorationSet.empty, stale: [], waiting: [], typing: null };

export function spellPlugin(): Plugin<SpellState> {
  const queue: Queue = { texts: new Set(), sent: new Set() };
  return new Plugin<SpellState>({
    key: spellKey,
    state: {
      init: () => OFF,
      apply(tr, prev, _old, state): SpellState {
        const meta = tr.getMeta(spellKey) as SpellMeta | undefined;
        if (meta?.setup !== undefined) {
          queue.texts.clear();
          if (!meta.setup) return OFF;
          const { set, missing } = rebuild(state.doc, DecorationSet.empty, queue);
          return { setup: meta.setup, decorations: set, stale: [], waiting: missing ? whole(state.doc) : [], typing: null };
        }
        if (!prev.setup) return prev;
        if (meta?.recheck) {
          const word = meta.recheck.accept;
          let set = prev.decorations;
          if (word) set = set.remove(set.find(undefined, undefined, (spec) => spec.spell === word));
          // Results were dropped (forgetSpelling): every paragraph is queued
          // and keeps its lines until its answer comes.
          const rebuilt = rebuild(state.doc, set, queue);
          return { ...prev, decorations: rebuilt.set, waiting: rebuilt.missing ? whole(state.doc) : [] };
        }
        if (meta?.refresh) {
          if (!prev.waiting.length) return prev;
          if (spanLength(prev.waiting) > WHOLE_PASS) {
            const rebuilt = rebuild(state.doc, prev.decorations, queue);
            return { ...prev, decorations: rebuilt.set, waiting: rebuilt.missing ? whole(state.doc) : [] };
          }
          const missed: Span[] = [];
          const set = respell(state.doc, prev.decorations, prev.waiting, queue, prev.typing, missed);
          return { ...prev, decorations: set, waiting: joinWaiting(state.doc, missed) };
        }

        // Where the writer is typing, and whether the cursor just left a word.
        let typing = prev.typing;
        let left: Span[] = [];
        if (tr.docChanged && state.selection.empty) typing = state.selection.head;
        else if (tr.selectionSet && !tr.docChanged && typing !== null) {
          left = [{ from: typing, to: typing }];
          typing = null;
        } else if (typing !== null && tr.docChanged) typing = tr.mapping.map(typing);

        const missed: Span[] = [];
        let next = follow(tr, prev, (doc, set, spans) => respell(doc, set, spans, queue, typing, missed));
        let waiting = mapWaiting(prev.waiting, tr);
        if (left.length && !tr.getMeta('composition')) {
          const set = respell(state.doc, next.decorations, left, queue, typing, missed);
          if (set !== next.decorations) next = { ...next, decorations: set };
        }
        if (missed.length) waiting = joinWaiting(state.doc, [...waiting, ...missed]);
        if (next === prev && waiting === prev.waiting && typing === prev.typing) return prev;
        return { ...next, waiting, typing };
      },
    },
    view: (view) => checker(view, queue),
    props: {
      decorations: (state) => spellKey.getState(state)?.decorations,
      handleDOMEvents: {
        contextmenu(view, event) {
          const setup = spellKey.getState(view.state)?.setup;
          const hit = setup ? hitAt(view, event) : null;
          if (!setup || !hit) return false;
          event.preventDefault();
          setup.onMenu(view, hit, event);
          return true;
        },
      },
    },
  });
}

/** Puts in the lines of paragraphs whose answers came (never while composing: see checker). */
export function showAnswers(view: Pick<EditorView, 'state' | 'dispatch' | 'isDestroyed'>) {
  if (view.isDestroyed || !spellKey.getState(view.state)?.setup) return;
  view.dispatch(view.state.tr.setMeta(spellKey, { refresh: true } satisfies SpellMeta).setMeta('addToHistory', false));
}

/** Sends queued paragraphs a little after the last change, a batch at a time. */
function checker(view: EditorView, queue: Queue) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let busy = false;
  let failed = false;

  const answer = () => afterComposition(view, () => showAnswers(view));

  const run = () => {
    const setup = spellKey.getState(view.state)?.setup;
    if (view.isDestroyed || !setup || busy) return;
    const texts: string[] = [];
    let chars = 0;
    for (const text of queue.texts) {
      queue.texts.delete(text);
      if (cache.has(text)) continue;
      texts.push(text);
      queue.sent.add(text);
      chars += text.length;
      if (texts.length >= BATCH_TEXTS || chars >= BATCH_CHARS) break;
    }
    if (!texts.length) {
      answer();
      return;
    }
    busy = true;
    failed = false;
    const key = knownKey;
    setup
      .check(texts)
      .then(
        (results) => {
          // Answers for words accepted before the last change of the lists are dropped.
          if (key === knownKey) texts.forEach((t, i) => rememberSpelling(t, results[i] ?? []));
        },
        () => {
          // No dictionary (taken off meanwhile): try again after the next edit.
          failed = true;
          queue.texts.clear();
        },
      )
      .finally(() => {
        texts.forEach((t) => queue.sent.delete(t));
        busy = false;
        if (failed) return;
        answer();
        if (queue.texts.size) schedule();
      });
  };

  const schedule = () => {
    clearTimeout(timer);
    timer = setTimeout(run, DELAY);
  };

  return {
    update() {
      if (queue.texts.size && !busy) schedule();
    },
    destroy() {
      clearTimeout(timer);
    },
  };
}

/** The flagged word under a pointer event, if any. */
function hitAt(view: EditorView, event: MouseEvent): SpellHit | null {
  const found = view.posAtCoords({ left: event.clientX, top: event.clientY });
  if (!found) return null;
  return spellingAt(view.state, found.pos);
}

/** The flagged word at document position `pos`. */
export function spellingAt(state: EditorState, pos: number): SpellHit | null {
  const set = spellKey.getState(state)?.decorations;
  const found = set?.find(pos, pos, (spec) => typeof spec.spell === 'string') ?? [];
  const d = found.find((x) => x.from <= pos && pos <= x.to);
  return d ? { from: d.from, to: d.to, word: (d.spec as { spell: string }).spell } : null;
}

/** Every flagged word in the document, in order (for tests and the panel). */
export function spellingHits(state: EditorState): SpellHit[] {
  const set = spellKey.getState(state)?.decorations;
  return (set?.find() ?? [])
    .map((d) => ({ from: d.from, to: d.to, word: (d.spec as { spell: string }).spell }))
    .sort((a, b) => a.from - b.from);
}

/** Puts `text` in place of a flagged word, if the word is still there. */
export function replaceSpelling(view: EditorView, hit: SpellHit, text: string): boolean {
  if (view.isDestroyed || !view.editable) return false;
  if (view.state.doc.textBetween(hit.from, hit.to, '\n') !== hit.word) return false;
  view.dispatch(view.state.tr.insertText(text, hit.from, hit.to));
  view.focus();
  return true;
}

/** Takes the lines off `word` at once (put in a word list, or ignored); the rest follows on the next check. */
export function dropSpelling(view: EditorView, word: string) {
  if (view.isDestroyed || !spellKey.getState(view.state)?.setup) return;
  for (const [text, issues] of cache) {
    if (issues.some((i) => i.word === word)) cache.set(text, issues.filter((i) => i.word !== word));
  }
  view.dispatch(view.state.tr.setMeta(spellKey, { recheck: { accept: word } } satisfies SpellMeta).setMeta('addToHistory', false));
}

export const Spelling = Extension.create({
  name: 'spelling',

  addProseMirrorPlugins() {
    return [spellPlugin()];
  },
});

/** Turns 맞춤법 표시 on with `setup`, or off with null. */
export function setSpelling(editor: Editor, setup: SpellSetup | null) {
  if (editor.isDestroyed) return;
  const prev = spellKey.getState(editor.state)?.setup ?? null;
  if (prev === setup) return;
  editor.view.dispatch(editor.state.tr.setMeta(spellKey, { setup } satisfies SpellMeta).setMeta('addToHistory', false));
}

/** Checks the whole document again (accepted words changed); lines on `accept` go at once. */
export function recheckSpelling(editor: Editor, accept?: string) {
  if (editor.isDestroyed || !spellKey.getState(editor.state)?.setup) return;
  editor.view.dispatch(
    editor.state.tr.setMeta(spellKey, { recheck: { accept } } satisfies SpellMeta).setMeta('addToHistory', false),
  );
}
