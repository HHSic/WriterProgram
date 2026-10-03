// 따옴표·말줄임표 자동 바꾸기 (docs/mvp-scope.md, off by default; 보기 설정,
// per device): " and ' become “ ” ‘ ’ (or 「 」 『 』), open or closed by
// what comes before; ... becomes …, -- becomes —.
//
// Never while a 한글 syllable is being composed: the plugin only looks at
// text typed outside a composition (ProseMirror's handleTextInput with
// `view.composing` false), and unlike prosemirror-inputrules it does not run
// again when a composition ends. Ctrl+Z right after a change takes back just
// the change, leaving what was typed (like `undoInputRule`).

import { Extension, type Editor } from '@tiptap/core';
import { Plugin, PluginKey, type Command, type Transaction } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';
import { editorKeys } from '../lib/shortcuts';

export type QuoteStyle = 'curly' | 'corner';

export interface AutoTypeSettings {
  on: boolean;
  quotes: QuoteStyle;
}

/** The last change, to take back with Ctrl+Z (same shape as prosemirror-inputrules keeps). */
interface Undoable {
  transform: Transaction;
  from: number;
  to: number;
  text: string;
}

interface AutoTypeState extends AutoTypeSettings {
  undo: Undoable | null;
}

export const autoTypeKey = new PluginKey<AutoTypeState>('autoType');

const QUOTES: Record<QuoteStyle, Record<'"' | "'", [string, string]>> = {
  curly: { '"': ['“', '”'], "'": ['‘', '’'] },
  corner: { '"': ['「', '」'], "'": ['『', '』'] },
};

/** After these (or at the start of a paragraph) a quote opens; after anything else it closes. */
const OPENS_AFTER = /[\s  　￼([{<《〈「『“‘—–]/u;

/** Text before the cursor that the rules look at. */
const LOOK_BACK = 40;

/**
 * What typing `typed` after `before` (the paragraph's text up to the
 * cursor) becomes: `remove` characters before the cursor and the typed one
 * replaced by `insert`. Null leaves the typing as it is.
 */
export function autoReplace(before: string, typed: string, quotes: QuoteStyle): { remove: number; insert: string } | null {
  if (typed === '"' || typed === "'") {
    const prev = before.slice(-1);
    const [open, close] = QUOTES[quotes][typed];
    return { remove: 0, insert: prev === '' || OPENS_AFTER.test(prev) ? open : close };
  }
  if (typed === '.' && before.endsWith('..')) return { remove: 2, insert: '…' };
  // A paragraph of just dashes is on its way to "---" (장면 나눔), so it is left alone.
  if (typed === '-' && before.endsWith('-') && !/^-+$/.test(before.trim())) return { remove: 1, insert: '—' };
  return null;
}

/** What the plugin needs of a view (tests pass a stand-in). */
export type TypingView = Pick<EditorView, 'composing' | 'state' | 'dispatch'>;

/** handleTextInput: turns the typed character into its typographic form when a rule says so. */
export function typeText(view: TypingView, from: number, to: number, text: string): boolean {
  const settings = autoTypeKey.getState(view.state);
  // Nothing during a composition: the text there belongs to the input method.
  if (!settings?.on || view.composing) return false;
  const { state } = view;
  const $from = state.doc.resolve(from);
  if (!$from.parent.isTextblock || $from.parent.type.spec.code) return false;
  const before = $from.parent.textBetween(Math.max(0, $from.parentOffset - LOOK_BACK), $from.parentOffset, undefined, '￼');
  const change = autoReplace(before, text, settings.quotes);
  if (!change) return false;
  const tr = state.tr.insertText(change.insert, from - change.remove, to);
  tr.setMeta(autoTypeKey, { undo: { transform: tr, from, to, text } });
  view.dispatch(tr);
  return true;
}

/** Takes back the last change if it was the last thing done: the typed text comes back as typed. */
export const undoAutoType: Command = (state, dispatch) => {
  const undo = autoTypeKey.getState(state)?.undo;
  if (!undo) return false;
  if (dispatch) {
    const tr = state.tr;
    const done = undo.transform;
    for (let i = done.steps.length - 1; i >= 0; i--) tr.step(done.steps[i].invert(done.docs[i]));
    const marks = tr.doc.resolve(undo.from).marks();
    tr.replaceWith(undo.from, undo.to, state.schema.text(undo.text, marks));
    dispatch(tr);
  }
  return true;
};

export function autoTypePlugin(): Plugin<AutoTypeState> {
  return new Plugin<AutoTypeState>({
    key: autoTypeKey,
    state: {
      init: () => ({ on: false, quotes: 'curly', undo: null }),
      apply(tr, prev) {
        const meta = tr.getMeta(autoTypeKey) as Partial<AutoTypeState> | undefined;
        if (meta) return { ...prev, undo: null, ...meta };
        return tr.docChanged || tr.selectionSet ? (prev.undo ? { ...prev, undo: null } : prev) : prev;
      },
    },
    props: {
      handleTextInput: (view, from, to, text) => typeText(view, from, to, text),
    },
  });
}

export const AutoType = Extension.create({
  name: 'autoType',
  // Before the history's Ctrl+Z, which would take back the typing too.
  priority: 200,

  addKeyboardShortcuts() {
    return editorKeys('undo', () => undoAutoType(this.editor.state, this.editor.view.dispatch));
  },

  addProseMirrorPlugins() {
    return [autoTypePlugin()];
  },
});

/** Turns the automatic changes on or off and picks the quotes (from 보기 설정). */
export function setAutoType(editor: Editor, settings: AutoTypeSettings) {
  const now = autoTypeKey.getState(editor.state);
  if (editor.isDestroyed || !now || (now.on === settings.on && now.quotes === settings.quotes)) return;
  editor.view.dispatch(editor.state.tr.setMeta(autoTypeKey, { on: settings.on, quotes: settings.quotes }).setMeta('addToHistory', false));
}
