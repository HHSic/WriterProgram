// Decorations worked out one text block at a time. After an edit only the
// blocks it touched are looked at again, so typing in a long chapter stays
// quick (used by card names and whitespace marks).

import type { Node as PmNode } from '@tiptap/pm/model';
import type { Transaction } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

/** Adds the decorations of one text block at document position `pos`. */
export type BlockDecorator = (block: PmNode, pos: number, out: Decoration[]) => void;

export function decorateAll(doc: PmNode, decorateBlock: BlockDecorator): DecorationSet {
  const out: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    decorateBlock(node, pos, out);
    return false;
  });
  return DecorationSet.create(doc, out);
}

export function redecorate(tr: Transaction, prev: DecorationSet, decorateBlock: BlockDecorator): DecorationSet {
  const doc = tr.doc;
  let set = prev.map(tr.mapping, doc);
  const done = new Set<number>();
  tr.mapping.maps.forEach((map, i) => {
    const later = tr.mapping.slice(i + 1);
    map.forEach((_oldFrom, _oldTo, newFrom, newTo) => {
      const from = Math.max(0, later.map(newFrom, -1) - 1);
      const to = Math.min(doc.content.size, later.map(newTo, 1) + 1);
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
    });
  });
  return set;
}
