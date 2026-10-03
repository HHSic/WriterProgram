// Setting card names in the manuscript: highlighted, and clicking one shows
// the card (본문 자동 강조, docs/screens.md S8).

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import type { CardSummary } from '../api/types';
import { decorateAll, follow, redecorateSpans, type BlockDecorator, type Deferred } from './decorate';
import { findNames } from './names';
import { blockText } from './search';

/** Names shorter than this are not highlighted (too many false hits). */
const MIN_NAME_CHARS = 2;

export interface NameIndex {
  regex: RegExp;
  /** name → card id */
  owners: Map<string, string>;
}

/** One pattern for all highlighted cards, longest names first. */
export function nameIndex(cards: CardSummary[]): NameIndex | null {
  const owners = new Map<string, string>();
  for (const card of cards) {
    if (!card.highlight) continue;
    for (const raw of [card.name, ...card.aliases]) {
      const name = raw.trim();
      if ([...name].length >= MIN_NAME_CHARS && !owners.has(name)) owners.set(name, card.id);
    }
  }
  if (!owners.size) return null;
  const names = [...owners.keys()].sort((a, b) => [...b].length - [...a].length);
  const source = names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
  return { regex: new RegExp(source, 'gu'), owners };
}

/** What changes the highlighting: highlighted cards with their names. */
export function nameKey(cards: CardSummary[]): string {
  return cards
    .filter((c) => c.highlight)
    .map((c) => [c.id, c.name, ...c.aliases].join('\t'))
    .join('\n');
}

/** Cards whose names appear in a document, with how often. */
export function castOf(doc: PmNode, index: NameIndex | null): Map<string, number> {
  const found = new Map<string, number>();
  if (!index) return found;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    for (const m of findNames(index.regex, blockText(node, pos).text)) {
      const id = index.owners.get(m.name);
      if (id) found.set(id, (found.get(id) ?? 0) + 1);
    }
    return false;
  });
  return found;
}

const cardKey = new PluginKey<CardState>('cardNames');

interface CardState extends Deferred {
  index: NameIndex | null;
}

function namesIn(index: NameIndex): BlockDecorator {
  return (block, pos, out) => {
    const { text, positions } = blockText(block, pos);
    for (const m of findNames(index.regex, text)) {
      const id = index.owners.get(m.name);
      if (!id) continue;
      const from = positions[m.index];
      const to = positions[m.index + m.name.length - 1] + 1;
      out.push(Decoration.inline(from, to, { class: 'card-name', 'data-card': id }));
    }
  };
}

function decorate(doc: PmNode, index: NameIndex | null): DecorationSet {
  return index ? decorateAll(doc, namesIn(index)) : DecorationSet.empty;
}

/** The plugin behind CardHighlight. */
export function cardPlugin(onOpen: (cardId: string) => void): Plugin<CardState> {
  return new Plugin<CardState>({
    key: cardKey,
    state: {
      init: () => ({ index: null, decorations: DecorationSet.empty, stale: [] }),
      apply(tr, prev, _old, state) {
        const meta = tr.getMeta(cardKey) as { index: NameIndex | null } | undefined;
        if (meta) return { index: meta.index, decorations: decorate(state.doc, meta.index), stale: [] };
        const index = prev.index;
        if (!index) return prev;
        return follow(tr, prev, (doc, set, spans) => redecorateSpans(doc, set, spans, namesIn(index)));
      },
    },
    props: {
      decorations: (state) => cardKey.getState(state)?.decorations,
      handleClick(view, _pos, event) {
        const target = event.target instanceof HTMLElement ? event.target.closest('[data-card]') : null;
        const id = target?.getAttribute('data-card');
        if (id && view.editable) onOpen(id);
        return false;
      },
    },
  });
}

/** Highlights card names; `onOpen` is called with a card id when one is clicked. */
export const CardHighlight = Extension.create<{ onOpen: (cardId: string) => void }>({
  name: 'cardHighlight',

  addOptions() {
    return { onOpen: () => {} };
  },

  addProseMirrorPlugins() {
    return [cardPlugin(this.options.onOpen)];
  },
});

export function setCardNames(editor: Editor, index: NameIndex | null) {
  if (editor.isDestroyed) return;
  editor.view.dispatch(editor.state.tr.setMeta(cardKey, { index }).setMeta('addToHistory', false));
}
