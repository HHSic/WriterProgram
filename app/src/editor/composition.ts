// Input method composition (한글 조합). While a syllable is being put
// together the text around the cursor belongs to the input method; changing
// decorations or text there can split or double the jamo in WebView2. So the
// decoration plugins only move their decorations along while composing and
// work them out again once composition ends, and text from elsewhere (another
// device, applied corrections) waits for the end (docs/safety-design.md K1).
//
// ProseMirror marks every transaction it reads from the DOM during a
// composition with the meta 'composition'. When a composition ends this
// plugin sends one empty transaction marked COMPOSITION_END.

import { Extension } from '@tiptap/core';
import { Plugin, PluginKey, type Transaction } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

/** Meta on the empty transaction sent when a composition has ended. */
export const COMPOSITION_END = 'composition-end';

/** Whether `tr` is input of a composition under way. */
export function composingIn(tr: Transaction): boolean {
  return tr.getMeta('composition') != null && !tr.getMeta(COMPOSITION_END);
}

/** Whether `tr` says a composition has ended. */
export function compositionEnded(tr: Transaction): boolean {
  return tr.getMeta(COMPOSITION_END) === true;
}

/**
 * Whether a key press belongs to a composition under way. Its Esc only
 * cancels the syllable being put together, so it must not also leave a mode
 * or stop something. `view`: the editor the keys go to, if any.
 */
export function keyInComposition(
  e: { isComposing?: boolean; keyCode?: number },
  view?: Pick<EditorView, 'composing' | 'isDestroyed'> | null,
): boolean {
  return !!e.isComposing || e.keyCode === 229 || (!!view && !view.isDestroyed && view.composing);
}

/** What the plugin needs of a view (tests pass a stand-in). */
export type ComposingView = Pick<EditorView, 'composing' | 'isDestroyed' | 'state' | 'dispatch'>;

/**
 * Called on compositionend. ProseMirror reads the last composed text in a
 * microtask after the event, so the end is sent a task later; when another
 * composition has started by then (the next syllable), its own end will do.
 */
export function onCompositionEnd(view: ComposingView) {
  setTimeout(() => {
    if (view.isDestroyed || view.composing) return;
    view.dispatch(view.state.tr.setMeta(COMPOSITION_END, true).setMeta('addToHistory', false));
  }, 0);
}

export const compositionKey = new PluginKey('composition');

export function compositionPlugin(): Plugin {
  return new Plugin({
    key: compositionKey,
    props: {
      handleDOMEvents: {
        compositionend: (view) => {
          onCompositionEnd(view);
          return false;
        },
      },
    },
  });
}

/** Sends COMPOSITION_END when a composition ends (one per editor). */
export const Composition = Extension.create({
  name: 'composition',

  addProseMirrorPlugins() {
    return [compositionPlugin()];
  },
});

/**
 * Runs `fn` now, or once the view's composition has ended and ProseMirror has
 * read the composed text. While syllables follow one another it keeps
 * waiting; a composition left open waits for the writer's next key or click.
 */
export function afterComposition(view: Pick<EditorView, 'composing' | 'isDestroyed' | 'dom'>, fn: () => void) {
  if (!view.composing) {
    fn();
    return;
  }
  const ended = () => {
    setTimeout(() => {
      if (view.isDestroyed) return;
      if (view.composing) view.dom.addEventListener('compositionend', ended, { once: true });
      else fn();
    }, 0);
  };
  view.dom.addEventListener('compositionend', ended, { once: true });
}
