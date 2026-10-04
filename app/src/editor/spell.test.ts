// 맞춤법 표시: where the lines go, that nothing is worked out again while a
// syllable is composed, and that old lines stay until a paragraph's answer
// comes back.

import { beforeEach, describe, expect, it } from 'vitest';
import { TextSelection } from '@tiptap/pm/state';
import type { SpellIssue } from '../api/types';
import { COMPOSITION_END } from './composition';
import {
  dropSpelling,
  forgetSpelling,
  recheckSpelling,
  rememberSpelling,
  setSpelling,
  showAnswers,
  spellPlugin,
  spellingAt,
  spellingHits,
  type SpellSetup,
} from './spell';
import { chapter, composing, fakeEditor } from './testing';

let asked: string[][] = [];
const SETUP: SpellSetup = {
  check: async (texts) => {
    asked.push(texts);
    return texts.map(() => []);
  },
  onMenu: () => {},
};

let generation = 0;
beforeEach(() => {
  // A fresh set of accepted words drops every remembered answer.
  forgetSpelling(`test ${(generation += 1)}`);
  asked = [];
});

function issue(text: string, word: string, nth = 0): SpellIssue {
  let at = -1;
  for (let i = 0; i <= nth; i += 1) at = text.indexOf(word, at + 1);
  return { start: at, end: at + word.length, word };
}

function words(state: ReturnType<typeof fakeEditor>['state']): string[] {
  return spellingHits(state).map((h) => state.doc.textBetween(h.from, h.to));
}

describe('맞춤법 표시', () => {
  it('puts a line on each word the answer names, in UTF-16 units', () => {
    const a = '나무는 멱었다.';
    const b = '😀 멱었다 그리고 멱었다';
    rememberSpelling(a, [issue(a, '멱었다')]);
    rememberSpelling(b, [issue(b, '멱었다'), issue(b, '멱었다', 1)]);
    const editor = fakeEditor(chapter(a, '', null, b), [spellPlugin()]);
    setSpelling(editor, SETUP);
    const hits = spellingHits(editor.state);
    expect(words(editor.state)).toEqual(['멱었다', '멱었다', '멱었다']);
    expect(hits.every((h) => h.word === '멱었다')).toBe(true);
    // First paragraph starts at 0; its text at 1; "멱었다" is 4 units in.
    expect(hits[0]).toEqual({ from: 5, to: 8, word: '멱었다' });
    expect(spellingAt(editor.state, 6)?.word).toBe('멱었다');
    expect(spellingAt(editor.state, 2)).toBeNull();
    setSpelling(editor, null);
    expect(spellingHits(editor.state)).toEqual([]);
  });

  it('waits for answers it does not have, then shows them', () => {
    const a = '사람을 멱었다';
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    expect(spellingHits(editor.state)).toEqual([]);
    rememberSpelling(a, [issue(a, '멱었다')]);
    showAnswers({ ...editor.view, state: editor.state, isDestroyed: false, dispatch: editor.view.dispatch });
    expect(words(editor.state)).toEqual(['멱었다']);
  });

  it('keeps old lines on a changed paragraph until its answer comes', () => {
    const a = '사람을 멱었다';
    rememberSpelling(a, [issue(a, '멱었다')]);
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    // Typed at the start of the paragraph: a text with no answer yet.
    editor.view.dispatch(editor.state.tr.insertText('그 ', 1));
    expect(words(editor.state)).toEqual(['멱었다']);
    const changed = editor.state.doc.firstChild!.textContent;
    rememberSpelling(changed, []);
    showAnswers({ state: editor.state, isDestroyed: false, dispatch: editor.view.dispatch });
    expect(words(editor.state)).toEqual([]);
  });

  it('only moves lines while a syllable is composed, and looks again when it ends', () => {
    const a = '멱었다 나무';
    const b = '멱었다 나무가';
    rememberSpelling(a, [issue(a, '멱었다')]);
    // The answer for the composed text is already known, yet nothing changes until the end.
    rememberSpelling(b, [issue(b, '나무가')]);
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    const before = spellingHits(editor.state);
    editor.view.dispatch(composing(editor.state.tr.insertText('가', 1 + a.length)));
    expect(spellingHits(editor.state)).toEqual(before);
    // Composition over, cursor elsewhere: the paragraph is looked at once.
    editor.view.dispatch(editor.state.tr.setSelection(TextSelection.create(editor.state.doc, 1)).setMeta(COMPOSITION_END, true));
    expect(words(editor.state)).toEqual(['나무가']);
  });

  it('gives no line to the word being typed until the cursor leaves it', () => {
    const a = '사람';
    const b = '사람을 멱';
    rememberSpelling(a, []);
    rememberSpelling(b, [issue(b, '멱')]);
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    // Typed "을 멱" with the cursor ending right after "멱".
    const tr = editor.state.tr.insertText('을 멱', 1 + a.length);
    editor.view.dispatch(tr.setSelection(TextSelection.create(tr.doc, 1 + b.length)));
    expect(words(editor.state)).toEqual([]);
    editor.view.dispatch(editor.state.tr.setSelection(TextSelection.create(editor.state.doc, 1)));
    expect(words(editor.state)).toEqual(['멱']);
  });

  it('drops lines on a word put in the dictionary at once', () => {
    const a = '서하는 멱었다';
    rememberSpelling(a, [issue(a, '서하는'), issue(a, '멱었다')]);
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    dropSpelling({ ...editor.view, state: editor.state, isDestroyed: false } as never, '서하는');
    expect(words(editor.state)).toEqual(['멱었다']);
  });

  it('checks everything again when the accepted words change, keeping lines meanwhile', () => {
    const a = '서하는 멱었다';
    rememberSpelling(a, [issue(a, '멱었다')]);
    const editor = fakeEditor(chapter(a), [spellPlugin()]);
    setSpelling(editor, SETUP);
    expect(forgetSpelling('other words')).toBe(true);
    expect(forgetSpelling('other words')).toBe(false);
    recheckSpelling(editor);
    expect(words(editor.state)).toEqual(['멱었다']);
    rememberSpelling(a, []);
    showAnswers({ state: editor.state, isDestroyed: false, dispatch: editor.view.dispatch });
    expect(words(editor.state)).toEqual([]);
  });
});
