// 해석 모드 in the editor: sentences that have a reading are underlined
// lightly, the one being looked at strongly (see workspace/InterpretTab.tsx).

import { Extension, type Editor } from '@tiptap/core';
import type { EditorState } from '@tiptap/pm/state';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import type { InterpretPlace } from '../api/types';
import { blockText, rangeInBlock } from './search';

const interpretKey = new PluginKey<DecorationSet>('interpret');

export interface InterpretMarks {
  places: InterpretPlace[];
  /** Id of the sentence being looked at. */
  current: string | null;
}

export const InterpretHighlight = Extension.create({
  name: 'interpretHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: interpretKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, prev) {
            const meta = tr.getMeta(interpretKey) as InterpretMarks | null | undefined;
            if (meta === undefined) return prev.map(tr.mapping, tr.doc);
            if (!meta) return DecorationSet.empty;
            const decos: Decoration[] = [];
            for (const p of meta.places) {
              const range = rangeInBlock(tr.doc, p.block, p.start, p.end);
              if (!range) continue;
              const cls = p.id === meta.current ? 'interpret-mark on' : 'interpret-mark';
              decos.push(Decoration.inline(range.from, range.to, { class: cls }));
            }
            return DecorationSet.create(tr.doc, decos);
          },
        },
        props: {
          decorations: (state) => interpretKey.getState(state),
        },
      }),
    ];
  },
});

/** Shows the marks, or clears them with `null`. */
export function setInterpretMarks(editor: Editor, marks: InterpretMarks | null) {
  if (editor.isDestroyed) return;
  editor.view.dispatch(editor.state.tr.setMeta(interpretKey, marks).setMeta('addToHistory', false));
}

/** The top-level block the cursor is in and its UTF-16 offset there. */
export function cursorPlace(state: EditorState): { block: number; offset: number } | null {
  const { $head } = state.selection;
  if ($head.depth < 1) return null;
  const block = $head.index(0);
  const node = state.doc.child(block);
  if (!node.isTextblock) return { block, offset: 0 };
  const { positions } = blockText(node, $head.before(1));
  const offset = positions.findIndex((p) => p >= $head.pos);
  return { block, offset: offset < 0 ? positions.length : offset };
}

/** The reading's sentence at `offset` in `block`, if any. */
export function placeAt(places: InterpretPlace[], block: number, offset: number): InterpretPlace | null {
  return places.find((p) => p.block === block && p.start <= offset && offset <= p.end) ?? null;
}
