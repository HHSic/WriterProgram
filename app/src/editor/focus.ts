// 집중 모드 in the editor (docs/screens.md S6): the paragraph being written
// stays clear while the others fade (styles/focus.css), and typewriter
// scrolling keeps the line being written at the same height on screen.
//
// The current paragraph is a node decoration. Like the other decorations it
// only moves along while a 한글 syllable is being composed and is worked out
// again once the composition ends (editor/composition.ts), so the text under
// the input method is never redrawn.

import { Extension, type Editor } from '@tiptap/core';
import { Plugin, PluginKey, type EditorState } from '@tiptap/pm/state';
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view';
import { composingIn, compositionEnded } from './composition';

export interface FocusLook {
  /** 다른 문단 흐리게 */
  dim: boolean;
  /** 타자기 스크롤 */
  typewriter: boolean;
}

interface FocusState extends FocusLook {
  decorations: DecorationSet;
}

const OFF: FocusLook = { dim: false, typewriter: false };

export const focusKey = new PluginKey<FocusState>('focusWriting');

/** Where the line being written stays: this share of the visible height from the top. */
export const TYPEWRITER_AT = 0.42;

/** How far to scroll so the caret's top sits at TYPEWRITER_AT of the scroller's height. */
export function typewriterDelta(caretTop: number, boxTop: number, boxHeight: number): number {
  return caretTop - (boxTop + boxHeight * TYPEWRITER_AT);
}

/** The top-level block the selection's head is in (a paragraph, or a scene break). */
export function currentBlock(state: EditorState): DecorationSet {
  const $head = state.selection.$head;
  let from: number;
  let to: number;
  if ($head.depth >= 1) {
    from = $head.before(1);
    to = $head.after(1);
  } else {
    const node = $head.nodeAfter;
    if (!node) return DecorationSet.empty;
    from = $head.pos;
    to = from + node.nodeSize;
  }
  return DecorationSet.create(state.doc, [Decoration.node(from, to, { class: 'is-current' })]);
}

function scrollCaret(view: EditorView) {
  const scroller = view.dom.closest<HTMLElement>('.doc-scroll');
  if (!scroller) return;
  let caret: { top: number };
  try {
    caret = view.coordsAtPos(view.state.selection.head);
  } catch {
    return;
  }
  const box = scroller.getBoundingClientRect();
  const delta = typewriterDelta(caret.top, box.top, box.height);
  if (Math.abs(delta) > 2) scroller.scrollTop += delta;
}

export function focusPlugin(): Plugin<FocusState> {
  return new Plugin<FocusState>({
    key: focusKey,
    state: {
      init: () => ({ ...OFF, decorations: DecorationSet.empty }),
      apply(tr, prev, _old, state) {
        const meta = tr.getMeta(focusKey) as FocusLook | undefined;
        const look = meta ?? prev;
        if (!look.dim) return prev.dim || meta ? { ...look, decorations: DecorationSet.empty } : prev;
        // While composing, the decoration only moves along with the text.
        if (composingIn(tr)) return { ...look, decorations: prev.decorations.map(tr.mapping, tr.doc) };
        if (!meta && !tr.docChanged && !tr.selectionSet && !compositionEnded(tr)) return prev;
        return { ...look, decorations: currentBlock(state) };
      },
    },
    props: {
      decorations: (state) => focusKey.getState(state)?.decorations,
      attributes: (state): Record<string, string> => (focusKey.getState(state)?.dim ? { class: 'focus-dim' } : {}),
      // Typing and moving with the keys ask for the selection to be shown:
      // with typewriter scrolling its line goes to the same height instead.
      // (A click asks for nothing, so the page does not jump under the mouse.)
      handleScrollToSelection: (view) => {
        if (!focusKey.getState(view.state)?.typewriter) return false;
        scrollCaret(view);
        return true;
      },
    },
  });
}

export const FocusWriting = Extension.create({
  name: 'focusWriting',

  addProseMirrorPlugins() {
    return [focusPlugin()];
  },
});

/** Turns 집중 모드's dimming and typewriter scrolling on or off (null: both off). */
export function setFocusLook(editor: Editor, look: FocusLook | null) {
  const want = look ?? OFF;
  const now = focusKey.getState(editor.state);
  if (editor.isDestroyed || !now || (now.dim === want.dim && now.typewriter === want.typewriter)) return;
  editor.view.dispatch(editor.state.tr.setMeta(focusKey, want).setMeta('addToHistory', false));
  if (want.typewriter && editor.view.hasFocus()) scrollCaret(editor.view);
}
