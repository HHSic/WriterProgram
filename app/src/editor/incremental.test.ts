// Decorations worked out only where an edit happened must be the same as
// working out the whole chapter again, after any edit.

import { describe, expect, it } from 'vitest';
import type { Node as PmNode } from '@tiptap/pm/model';
import type { EditorState, Transaction } from '@tiptap/pm/state';
import { Decoration, type DecorationSet } from '@tiptap/pm/view';
import type { CardSummary, IndentRules } from '../api/types';
import { cardPlugin, nameIndex, setCardNames } from './cards';
import { COMPOSITION_END } from './composition';
import { decorateAll } from './decorate';
import { findNames } from './names';
import { indentKinds, indentPlugin, setIndentRules } from './indent';
import { blockText } from './search';
import { composing, fakeEditor, schema } from './testing';

/** A small seeded random number generator (mulberry32). */
function random(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const PIECES = ['서하', '서하늘', '가', '는 ', '“', '”', ' ', '윤서하', '말했다.', '「', '하늘', 'ㄱ'];
const RULES: IndentRules[] = [
  { chapterFirst: true, afterScene: true, margined: true, dialogue: true, dialogueHang: false },
  { chapterFirst: true, afterScene: true, margined: false, dialogue: false, dialogueHang: true },
];
const CARDS: CardSummary[] = [
  { id: 'a', cardType: 'person', name: '서하', aliases: ['윤서하'], highlight: true, summary: '' },
  { id: 'b', cardType: 'place', name: '하늘', aliases: [], highlight: true, summary: '' },
];

function startDoc(rnd: () => number): PmNode {
  const blocks: PmNode[] = [];
  for (let i = 0; i < 12; i += 1) {
    const r = rnd();
    if (r < 0.12) blocks.push(schema.nodes.sceneBreak.create());
    else if (r < 0.3) blocks.push(schema.nodes.paragraph.create());
    else {
      const attrs = rnd() < 0.15 ? { left: 2 } : rnd() < 0.1 ? { indent: 1 } : null;
      let text = '';
      while (rnd() < 0.7) text += PIECES[Math.floor(rnd() * PIECES.length)];
      blocks.push(schema.nodes.paragraph.create(attrs, text ? schema.text(text) : null));
    }
  }
  return schema.nodes.doc.create(null, blocks);
}

/** Some position inside a paragraph's text, or null when there is none. */
function textPos(doc: PmNode, rnd: () => number): number | null {
  const places: number[] = [];
  doc.forEach((node, pos) => {
    if (node.type.name === 'paragraph') for (let i = 0; i <= node.content.size; i += 1) places.push(pos + 1 + i);
  });
  return places.length ? places[Math.floor(rnd() * places.length)] : null;
}

/** A random edit of the kinds writers make. */
function edit(state: EditorState, rnd: () => number): Transaction | null {
  const tr = state.tr;
  const doc = state.doc;
  const at = textPos(doc, rnd);
  const r = rnd();
  if (at === null) return tr.insert(doc.content.size, schema.nodes.paragraph.create(null, schema.text('서하')));
  if (r < 0.35) return tr.insertText(PIECES[Math.floor(rnd() * PIECES.length)], at);
  if (r < 0.5) {
    const to = textPos(doc, rnd);
    if (to === null || to === at) return null;
    return tr.delete(Math.min(at, to), Math.max(at, to));
  }
  if (r < 0.62) return tr.split(at);
  if (r < 0.7) {
    const $at = doc.resolve(at);
    return tr.insert($at.after(1), schema.nodes.sceneBreak.create());
  }
  if (r < 0.8) {
    // Empty a paragraph (it then no longer ends "first" or "after a scene").
    const $at = doc.resolve(at);
    return tr.delete($at.start(1), $at.end(1));
  }
  if (r < 0.88) {
    const $at = doc.resolve(at);
    return tr.setNodeMarkup($at.before(1), undefined, { ...$at.parent.attrs, left: $at.parent.attrs.left ? 0 : 3 });
  }
  if (r < 0.94) {
    // Remove a whole block.
    const $at = doc.resolve(at);
    if (doc.childCount < 2) return null;
    return tr.delete($at.before(1), $at.after(1));
  }
  // Join with the paragraph before, as Backspace at its start does.
  const $at = doc.resolve(at);
  const before = $at.before(1);
  if (before === 0 || doc.resolve(before).nodeBefore?.type.name !== 'paragraph') return null;
  return tr.join(before);
}

function indentList(set: DecorationSet | undefined): string[] {
  return (set?.find() ?? []).map((d) => `${d.from}-${d.to}:${(d.spec as { kind: string }).kind}`);
}

function fullIndent(doc: PmNode, rules: IndentRules): string[] {
  return [...indentKinds(doc, rules)].map(([pos, kind]) => `${pos}-${pos + doc.nodeAt(pos)!.nodeSize}:${kind}`);
}

function cardList(set: DecorationSet | undefined): string[] {
  return (set?.find() ?? []).map((d) => `${d.from}-${d.to}`).sort();
}

function fullCards(doc: PmNode): string[] {
  const index = nameIndex(CARDS)!;
  const set = decorateAll(doc, (block, pos, out) => {
    const { text, positions } = blockText(block, pos);
    for (const m of findNames(index.regex, text)) {
      out.push(Decoration.inline(positions[m.index], positions[m.index + m.name.length - 1] + 1, {}));
    }
  });
  return set
    .find()
    .map((d) => `${d.from}-${d.to}`)
    .sort();
}

describe('decorations worked out where the text changed', () => {
  for (const [r, rules] of RULES.entries()) {
    for (let seed = 1; seed <= 25; seed += 1) {
      it(`match the whole chapter worked out again (rules ${r}, seed ${seed})`, () => {
        const rnd = random(seed * 7919 + r);
        const indent = indentPlugin();
        const cards = cardPlugin(() => {});
        const e = fakeEditor(startDoc(rnd), [indent, cards]);
        setIndentRules(e, rules);
        setCardNames(e, nameIndex(CARDS));
        let open = false;
        for (let i = 0; i < 60; i += 1) {
          const tr = edit(e.state, rnd);
          if (!tr) continue;
          // Some runs of edits come from a composition, closed later.
          if (rnd() < 0.3) {
            composing(tr);
            open = true;
          } else if (open && rnd() < 0.5) {
            e.view.dispatch(e.state.tr.setMeta(COMPOSITION_END, true));
            open = false;
          }
          e.view.dispatch(tr);
          // Changes made while composing wait for the end or the next edit.
          if (tr.getMeta('composition') != null || !tr.docChanged) continue;
          open = false;
          const doc = e.state.doc;
          expect(indentList(indent.getState(e.state)?.decorations)).toEqual(fullIndent(doc, rules));
          expect(cardList(cards.getState(e.state)?.decorations)).toEqual(fullCards(doc));
        }
        e.view.dispatch(e.state.tr.setMeta(COMPOSITION_END, true));
        expect(indentList(indent.getState(e.state)?.decorations)).toEqual(fullIndent(e.state.doc, rules));
        expect(cardList(cards.getState(e.state)?.decorations)).toEqual(fullCards(e.state.doc));
      });
    }
  }
});
