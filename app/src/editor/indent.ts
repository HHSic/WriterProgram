// First-line indent rules on screen (crates/core/src/indent.rs): paragraphs
// the manuscript format leaves flush (a chapter's first paragraph, the one
// after a scene break, paragraphs set in with margins, dialogue) or sets in
// as on 원고지 (dialogue with every line one cell in). A paragraph's own first
// line (data-indent) is drawn by CSS and always wins.

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, type Transaction } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import type { IndentRules } from '../api/types';
import { follow, topLevelSet, type Deferred, type Span } from './decorate';

/** Marks that open a line of dialogue (same as indent.rs). */
const QUOTES = ['“', '"', '「', '『', '‘', "'", '《', '«'];

export function isDialogue(paragraph: PmNode): boolean {
  const text = paragraph.textContent.trimStart();
  return text.length > 0 && QUOTES.includes(text[0]);
}

export type IndentKind = 'flush' | 'hang' | null;

/** What the paragraphs before a block leave for it: no text yet in the
 * chapter, or none since the last scene break. */
interface Carry {
  first: boolean;
  afterScene: boolean;
}

/** The kind of top-level block `node`, moving `carry` past it. */
function step(node: PmNode, rules: IndentRules, carry: Carry): IndentKind {
  if (node.type.name !== 'paragraph') {
    if (node.type.name === 'sceneBreak') carry.afterScene = true;
    return null;
  }
  let kind: IndentKind = null;
  const own = node.attrs.indent !== null && node.attrs.indent !== undefined;
  if (!own) {
    const margined = Number(node.attrs.left) > 0 || Number(node.attrs.right) > 0;
    const dialogue = isDialogue(node);
    const flush =
      (rules.chapterFirst && carry.first) ||
      (rules.afterScene && carry.afterScene) ||
      (rules.margined && margined) ||
      (rules.dialogue && dialogue);
    if (flush) kind = 'flush';
    else if (rules.dialogueHang && dialogue) kind = 'hang';
  }
  if (node.content.size > 0) {
    carry.first = false;
    carry.afterScene = false;
  }
  return kind;
}

/** What the rules make of each top-level paragraph, by position. */
export function indentKinds(doc: PmNode, rules: IndentRules | null): Map<number, IndentKind> {
  const out = new Map<number, IndentKind>();
  if (!rules) return out;
  const carry: Carry = { first: true, afterScene: false };
  doc.forEach((node, pos) => {
    const kind = step(node, rules, carry);
    if (kind) out.set(pos, kind);
  });
  return out;
}

/** What the paragraphs before top-level block `index` leave for it; looks
 * back only over empty paragraphs and scene breaks. */
function carryBefore(doc: PmNode, index: number): Carry {
  const carry: Carry = { first: true, afterScene: false };
  for (let i = index - 1; i >= 0; i -= 1) {
    const node = doc.child(i);
    if (node.type.name === 'sceneBreak') carry.afterScene = true;
    else if (node.type.name === 'paragraph' && node.content.size > 0) {
      carry.first = false;
      break;
    }
  }
  return carry;
}

/** What the rules make of top-level block `index` alone (the ruler asks on every keystroke). */
export function indentKindAt(doc: PmNode, rules: IndentRules | null, index: number): IndentKind {
  if (!rules || index < 0 || index >= doc.childCount) return null;
  return step(doc.child(index), rules, carryBefore(doc, index));
}

function decoration(from: number, to: number, kind: IndentKind): Decoration {
  return Decoration.node(from, to, { class: kind === 'flush' ? 'indent-flush' : 'indent-hang' }, { kind });
}

// The indent decorations sit on top-level paragraphs, tens of thousands of
// them in a long chapter: they are kept with topLevelSet and moved along by
// hand, since DecorationSet.map checks each one against the document from
// its start.

/** Moves the decorations through `tr`, dropping those whose paragraph went
 * (as DecorationSet.map does, without checking them against the document). */
function moveIndent(set: DecorationSet, tr: Transaction): DecorationSet {
  const out: Decoration[] = [];
  for (const d of set.find()) {
    const from = tr.mapping.mapResult(d.from, 1);
    const to = tr.mapping.mapResult(d.to, -1);
    if (from.deleted || to.deleted || to.pos <= from.pos) continue;
    out.push(from.pos === d.from && to.pos === d.to ? d : decoration(from.pos, to.pos, (d.spec as { kind: IndentKind }).kind));
  }
  return topLevelSet(tr.doc, out);
}

interface IndentState extends Deferred {
  rules: IndentRules | null;
}

const indentKey = new PluginKey<IndentState>('indentRules');

function decorate(doc: PmNode, rules: IndentRules | null): DecorationSet {
  const decorations: Decoration[] = [];
  if (rules) {
    const carry: Carry = { first: true, afterScene: false };
    doc.forEach((node, pos) => {
      const kind = step(node, rules, carry);
      if (kind) decorations.push(decoration(pos, pos + node.nodeSize, kind));
    });
  }
  return topLevelSet(doc, decorations);
}

/**
 * `set` with the paragraphs touching `spans` and their neighbours worked out
 * again. A paragraph depends on those before it only through Carry, which
 * goes back to nothing after any paragraph with text: so the work starts
 * one block before the change, finds Carry by looking back over empty
 * paragraphs and scene breaks, and stops at the first unchanged paragraph
 * with text after the change (from there on nothing differs).
 */
export function redecorateIndent(doc: PmNode, set: DecorationSet, rules: IndentRules, spans: Span[]): DecorationSet {
  const count = doc.childCount;
  if (!spans.length || !count) return set;
  const size = doc.content.size;
  let lo = count;
  let hi = -1;
  for (const s of spans) {
    lo = Math.min(lo, doc.resolve(Math.max(0, Math.min(size, s.from))).index(0));
    hi = Math.max(hi, doc.resolve(Math.max(0, Math.min(size, s.to))).index(0));
  }
  const start = Math.max(0, lo - 1);
  const last = Math.min(count - 1, hi);

  const carry = carryBefore(doc, start);
  let pos = 0;
  for (let i = 0; i < start; i += 1) pos += doc.child(i).nodeSize;
  const from = pos;
  const fresh: Decoration[] = [];
  for (let i = start; i < count; i += 1) {
    const node = doc.child(i);
    const kind = step(node, rules, carry);
    if (kind) fresh.push(decoration(pos, pos + node.nodeSize, kind));
    pos += node.nodeSize;
    if (i > last && node.type.name === 'paragraph' && node.content.size > 0) break;
  }
  const to = pos;
  const all = set.find();
  const before = all.filter((d) => d.to <= from);
  const after = all.filter((d) => d.from >= to);
  return topLevelSet(doc, [...before, ...fresh, ...after]);
}

/** The plugin behind IndentRulesExtension. */
export function indentPlugin(): Plugin<IndentState> {
  return new Plugin<IndentState>({
    key: indentKey,
    state: {
      init: () => ({ rules: null, decorations: DecorationSet.empty, stale: [] }),
      apply(tr, prev, _old, state) {
        const meta = tr.getMeta(indentKey) as { rules: IndentRules | null } | undefined;
        if (meta) return { rules: meta.rules, decorations: decorate(state.doc, meta.rules), stale: [] };
        const rules = prev.rules;
        if (!rules) return prev;
        return follow(tr, prev, (doc, set, spans) => redecorateIndent(doc, set, rules, spans), moveIndent);
      },
    },
    props: {
      decorations: (state) => indentKey.getState(state)?.decorations,
    },
  });
}

export const IndentRulesExtension = Extension.create({
  name: 'indentRules',

  addProseMirrorPlugins() {
    return [indentPlugin()];
  },
});

/** Follows the manuscript format's rules (none: every paragraph gets the indent). */
export function setIndentRules(editor: Editor, rules: IndentRules | null) {
  if (editor.isDestroyed) return;
  const now = indentKey.getState(editor.state)?.rules ?? null;
  if (JSON.stringify(now) === JSON.stringify(rules)) return;
  editor.view.dispatch(editor.state.tr.setMeta(indentKey, { rules }).setMeta('addToHistory', false));
}
