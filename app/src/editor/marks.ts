// 빈칸·문단 부호 보이기: small marks on spaces and special spaces while writing,
// like 한글's 조판 부호. Paragraph ends are drawn by CSS (styles/paragraphs.css
// ".show-marks"), which keeps the text itself untouched for the IME.

import { Extension, type Editor } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { decorateAll, follow, redecorateSpans, type BlockDecorator, type Deferred } from './decorate';
import { blockText } from './search';

const CLASSES: Record<string, string> = {
  ' ': 'ws-space',
  ' ': 'ws-nbsp',
  ' ': 'ws-fixed',
  '　': 'ws-full',
  '\t': 'ws-tab',
};
const SPACES = /[   　\t]/g;

const spaces: BlockDecorator = (block, pos, out) => {
  const { text, positions } = blockText(block, pos);
  for (const m of text.matchAll(SPACES)) {
    if (m.index === undefined) continue;
    const at = positions[m.index];
    out.push(Decoration.inline(at, at + 1, { class: CLASSES[m[0]] }));
  }
};

interface MarksState extends Deferred {
  on: boolean;
}

const marksKey = new PluginKey<MarksState>('whitespaceMarks');

export const WhitespaceMarks = Extension.create({
  name: 'whitespaceMarks',

  addProseMirrorPlugins() {
    return [
      new Plugin<MarksState>({
        key: marksKey,
        state: {
          init: () => ({ on: false, decorations: DecorationSet.empty, stale: [] }),
          apply(tr, prev, _old, state) {
            const meta = tr.getMeta(marksKey) as { on: boolean } | undefined;
            if (meta) return { on: meta.on, decorations: meta.on ? decorateAll(state.doc, spaces) : DecorationSet.empty, stale: [] };
            if (!prev.on) return prev;
            return follow(tr, prev, (doc, set, spans) => redecorateSpans(doc, set, spans, spaces));
          },
        },
        props: {
          decorations: (state) => marksKey.getState(state)?.decorations,
        },
      }),
    ];
  },
});

export function setWhitespaceMarks(editor: Editor, on: boolean) {
  if (editor.isDestroyed || marksKey.getState(editor.state)?.on === on) return;
  editor.view.dispatch(editor.state.tr.setMeta(marksKey, { on }).setMeta('addToHistory', false));
}
