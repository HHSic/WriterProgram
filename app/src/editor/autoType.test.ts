import { describe, expect, it } from 'vitest';
import { EditorState, TextSelection, type Transaction } from '@tiptap/pm/state';
import { history, undo } from '@tiptap/pm/history';
import { autoReplace, autoTypeKey, autoTypePlugin, typeText, undoAutoType, type QuoteStyle } from './autoType';
import { chapter } from './testing';

describe('autoReplace', () => {
  const after = (before: string, typed: string, quotes: QuoteStyle = 'curly') => {
    const r = autoReplace(before, typed, quotes);
    return r ? before.slice(0, before.length - r.remove) + r.insert : before + typed;
  };

  it('opens a quote at the start, after a space or an opening mark, and closes it after a letter', () => {
    expect(after('', '"')).toBe('“');
    expect(after('그가 말했다. ', '"')).toBe('그가 말했다. “');
    expect(after('“안녕', '"')).toBe('“안녕”');
    expect(after('(', "'")).toBe('(‘');
    expect(after('“그건 ', "'")).toBe('“그건 ‘');
    expect(after('‘정말', "'")).toBe('‘정말’');
    expect(after('“어?', '"')).toBe('“어?”');
    // After a line break (Shift+Enter) a quote opens again.
    expect(after('첫 줄￼', '"')).toBe('첫 줄￼“');
  });

  it('makes 낫표 when asked', () => {
    expect(after('', '"', 'corner')).toBe('「');
    expect(after('「안녕', '"', 'corner')).toBe('「안녕」');
    expect(after('「그건 ', "'", 'corner')).toBe('「그건 『');
  });

  it('turns three dots into … and two dashes into —', () => {
    expect(after('그런데..', '.')).toBe('그런데…');
    expect(after('그런데.', '.')).toBe('그런데..');
    expect(after('그런데…..', '.')).toBe('그런데……');
    expect(after('그때-', '-')).toBe('그때—');
    // "---" on a line of its own is a scene break, not a dash.
    expect(after('-', '-')).toBe('--');
    expect(after('--', '-')).toBe('---');
  });

  it('leaves everything else alone', () => {
    expect(autoReplace('가', '나', 'curly')).toBeNull();
    expect(autoReplace('가', '".', 'curly')).toBeNull();
  });
});

/** An editor state with the plugin (and history), the cursor at the end of the first paragraph. */
function editor(text: string, on = true) {
  let state = EditorState.create({ doc: chapter(text), plugins: [history(), autoTypePlugin()] });
  state = state.apply(state.tr.setMeta(autoTypeKey, { on, quotes: 'curly' }));
  state = state.apply(state.tr.setSelection(TextSelection.create(state.doc, 1 + text.length)));
  const view = {
    composing: false,
    get state() {
      return state;
    },
    dispatch(tr: Transaction) {
      state = state.apply(tr);
    },
  };
  /** Types `ch` the way ProseMirror does: the plugin first, else plain text. */
  const type = (ch: string) => {
    const { from, to } = state.selection;
    if (!typeText(view, from, to, ch)) view.dispatch(state.tr.insertText(ch, from, to));
  };
  return {
    view,
    type,
    text: () => state.doc.firstChild!.textContent,
    get state() {
      return state;
    },
  };
}

describe('typing', () => {
  it('changes what is typed when on, and nothing when off', () => {
    const on = editor('그가 말했다. ');
    for (const ch of '"안녕..."') on.type(ch);
    expect(on.text()).toBe('그가 말했다. “안녕…”');

    const off = editor('그가 말했다. ', false);
    for (const ch of '"안녕..."') off.type(ch);
    expect(off.text()).toBe('그가 말했다. "안녕..."');
  });

  it('never changes text while a 한글 syllable is being composed', () => {
    const e = editor('“안녕');
    e.view.composing = true;
    const { from, to } = e.state.selection;
    expect(typeText(e.view, from, to, '"')).toBe(false);
    expect(e.text()).toBe('“안녕');
  });

  it('takes back just the change on Ctrl+Z, then the typing on the next', () => {
    const e = editor('그런데');
    for (const ch of '...') e.type(ch);
    expect(e.text()).toBe('그런데…');
    expect(undoAutoType(e.state, e.view.dispatch)).toBe(true);
    expect(e.text()).toBe('그런데...');
    // Only right after the change.
    expect(undoAutoType(e.state, e.view.dispatch)).toBe(false);
    undo(e.state, e.view.dispatch);
    expect(e.text()).toBe('그런데');
  });

  it('forgets the change once the writer moves on', () => {
    const e = editor('');
    e.type('"');
    e.type('네');
    expect(e.text()).toBe('“네');
    expect(undoAutoType(e.state, e.view.dispatch)).toBe(false);
  });
});
