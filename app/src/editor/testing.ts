// Helpers for tests: the manuscript schema and editor states without a DOM.

import { getSchema, type Editor } from '@tiptap/core';
import type { Node as PmNode, Schema } from '@tiptap/pm/model';
import { EditorState, type Plugin, type Transaction } from '@tiptap/pm/state';
import { manuscriptExtensions } from './extensions';

export const schema: Schema = getSchema(manuscriptExtensions('◆', { onCardOpen() {}, onNoteOpen() {}, onNoteAdd() {} }));

/** A chapter from paragraphs; null is a scene break. */
export function chapter(...blocks: (string | null)[]): PmNode {
  return schema.nodes.doc.create(
    null,
    blocks.map((b) =>
      b === null ? schema.nodes.sceneBreak.create() : schema.nodes.paragraph.create(null, b ? schema.text(b) : null),
    ),
  );
}

/** Just enough of an Editor for the set… functions of the decoration plugins. */
export function fakeEditor(doc: PmNode, plugins: Plugin[]) {
  const holder = {
    isDestroyed: false,
    state: EditorState.create({ doc, plugins }),
    view: { dispatch: (tr: Transaction) => (holder.state = holder.state.apply(tr)) },
  };
  return holder as typeof holder & Editor;
}

/** Marks a transaction the way ProseMirror marks input read during a composition. */
export function composing(tr: Transaction, id = 1): Transaction {
  return tr.setMeta('composition', id);
}
