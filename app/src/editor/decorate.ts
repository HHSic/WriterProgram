// Decorations worked out one text block at a time. After an edit only the
// blocks it touched are looked at again, so typing in a long chapter stays
// quick (used by card names, search hits and whitespace marks).
//
// While an input method is composing (editor/composition.ts) decorations are
// only moved along with the text; the stretches typed are remembered and
// looked at once the composition ends.

import type { Node as PmNode } from '@tiptap/pm/model';
import type { Transaction } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { compositionEnded, composingIn } from './composition';

/** Adds the decorations of one text block at document position `pos`. */
export type BlockDecorator = (block: PmNode, pos: number, out: Decoration[]) => void;

/** A stretch of the document, in positions. */
export interface Span {
  from: number;
  to: number;
}

// A chapter has tens of thousands of top-level blocks. DecorationSet.create
// and DecorationSet.map check every decoration against the document from its
// start, which takes seconds for a 1.4 million character chapter. So whole
// sets are put together here block by block, through ProseMirror's own
// DecorationSet constructor (marked internal): `local` holds decorations on
// the blocks themselves and `children` is [start, end, set] for each block
// with decorations inside, positions in the set counted from the block's
// content. Should the constructor ever stop working that way, the slow path
// is used.
const RawSet = DecorationSet as unknown as new (local: Decoration[], children: (number | DecorationSet)[]) => DecorationSet;
const rawWorks = (() => {
  try {
    const inner = new RawSet([Decoration.inline(0, 1, {})], []);
    const found = new RawSet([Decoration.node(4, 6, {})], [0, 3, inner]).find();
    return found.length === 2 && found.some((d) => d.from === 1 && d.to === 2) && found.some((d) => d.from === 4 && d.to === 6);
  } catch {
    return false;
  }
})();

/** A set of decorations on top-level blocks, given in document order. */
export function topLevelSet(doc: PmNode, decorations: Decoration[]): DecorationSet {
  if (!decorations.length) return DecorationSet.empty;
  return rawWorks ? new RawSet(decorations, []) : DecorationSet.create(doc, decorations);
}

export function decorateAll(doc: PmNode, decorateBlock: BlockDecorator): DecorationSet {
  // Blocks inside blocks (not in the manuscript schema) take the slow path.
  let flat = rawWorks;
  doc.forEach((node) => {
    if (!node.isTextblock && !node.isLeaf && node.content.size) flat = false;
  });
  if (!flat) {
    const out: Decoration[] = [];
    doc.descendants((node, pos) => {
      if (!node.isTextblock) return true;
      decorateBlock(node, pos, out);
      return false;
    });
    return DecorationSet.create(doc, out);
  }
  const children: (number | DecorationSet)[] = [];
  doc.forEach((node, pos) => {
    if (!node.isTextblock) return;
    const out: Decoration[] = [];
    // Positions counted from the block's content.
    decorateBlock(node, -1, out);
    if (out.length) children.push(pos, pos + node.nodeSize, DecorationSet.create(node, out));
  });
  return children.length ? new RawSet([], children) : DecorationSet.empty;
}

/** Where `tr` changed the document, in positions of the new document. */
export function changedSpans(tr: Transaction): Span[] {
  const out: Span[] = [];
  tr.mapping.maps.forEach((map, i) => {
    const later = tr.mapping.slice(i + 1);
    map.forEach((_oldFrom, _oldTo, newFrom, newTo) => {
      out.push({ from: later.map(newFrom, -1), to: later.map(newTo, 1) });
    });
  });
  return out;
}

/** Spans in order, overlapping and touching ones joined. */
function joined(spans: Span[]): Span[] {
  const sorted = [...spans].sort((a, b) => a.from - b.from);
  const out: Span[] = [];
  for (const s of sorted) {
    const last = out[out.length - 1];
    if (last && s.from <= last.to) last.to = Math.max(last.to, s.to);
    else out.push({ ...s });
  }
  return out;
}

/** `spans` moved through `tr`, with what `tr` changed added. */
export function carrySpans(spans: Span[], tr: Transaction): Span[] {
  if (!tr.docChanged) return spans;
  const moved = spans.map((s) => ({ from: tr.mapping.map(s.from, -1), to: tr.mapping.map(s.to, 1) }));
  return joined([...moved, ...changedSpans(tr)]);
}

/** `set` with the decorations of the text blocks touching `spans` worked out again. */
export function redecorateSpans(doc: PmNode, set: DecorationSet, spans: Span[], decorateBlock: BlockDecorator): DecorationSet {
  const done = new Set<number>();
  for (const span of spans) {
    const from = Math.max(0, span.from - 1);
    const to = Math.min(doc.content.size, span.to + 1);
    doc.nodesBetween(from, to, (node, pos) => {
      if (!node.isTextblock) return true;
      if (!done.has(pos)) {
        done.add(pos);
        set = set.remove(set.find(pos + 1, pos + node.nodeSize - 1));
        const out: Decoration[] = [];
        decorateBlock(node, pos, out);
        set = set.add(doc, out);
      }
      return false;
    });
  }
  return set;
}

/** Decorations that may be waiting for a composition to end. */
export interface Deferred {
  decorations: DecorationSet;
  /** Stretches changed while composing, not looked at yet. */
  stale: Span[];
}

function mapSet(set: DecorationSet, tr: Transaction): DecorationSet {
  return set.map(tr.mapping, tr.doc);
}

/**
 * Carries decorations through `tr`. While composing they are only moved
 * (`move`) and the changed stretches kept; otherwise `redo` works out again
 * what changed now and while composing. Returns `prev` itself when nothing
 * changed.
 */
export function follow<S extends Deferred>(
  tr: Transaction,
  prev: S,
  redo: (doc: PmNode, set: DecorationSet, spans: Span[]) => DecorationSet,
  move: (set: DecorationSet, tr: Transaction) => DecorationSet = mapSet,
): S {
  if (composingIn(tr)) {
    if (!tr.docChanged) return prev;
    return { ...prev, decorations: move(prev.decorations, tr), stale: carrySpans(prev.stale, tr) };
  }
  if (!tr.docChanged && !(compositionEnded(tr) && prev.stale.length)) return prev;
  const spans = carrySpans(prev.stale, tr);
  const moved = tr.docChanged ? move(prev.decorations, tr) : prev.decorations;
  return { ...prev, decorations: redo(tr.doc, moved, spans), stale: [] };
}
