import { describe, expect, it } from 'vitest';
import { TextSelection } from '@tiptap/pm/state';
import { COMPOSITION_END, keyInComposition } from './composition';
import { focusKey, focusPlugin, setFocusLook, typewriterDelta, TYPEWRITER_AT } from './focus';
import { chapter, composing, fakeEditor } from './testing';

/** The text of the block marked as the one being written. */
function current(e: ReturnType<typeof fakeEditor>): string[] {
  const set = focusKey.getState(e.state)!.decorations;
  return set.find().map((d) => e.state.doc.textBetween(d.from, d.to));
}

function editor() {
  const e = fakeEditor(chapter('첫 문단', '둘째 문단', null, '넷째'), [focusPlugin()]);
  (e as unknown as { isDestroyed: boolean }).isDestroyed = false;
  return e;
}

function cursorAt(e: ReturnType<typeof editor>, pos: number) {
  e.view.dispatch(e.state.tr.setSelection(TextSelection.create(e.state.doc, pos)));
}

describe('dimming other paragraphs', () => {
  it('marks the paragraph with the cursor, only while on', () => {
    const e = editor();
    cursorAt(e, 8); // in 둘째 문단
    expect(current(e)).toEqual([]);
    e.view.dispatch(e.state.tr.setMeta(focusKey, { dim: true, typewriter: false }));
    expect(current(e)).toEqual(['둘째 문단']);
    cursorAt(e, 2);
    expect(current(e)).toEqual(['첫 문단']);
    e.view.dispatch(e.state.tr.setMeta(focusKey, { dim: false, typewriter: false }));
    expect(current(e)).toEqual([]);
  });

  it('only moves along while a syllable is composed, and catches up when it ends', () => {
    const e = editor();
    e.view.dispatch(e.state.tr.setMeta(focusKey, { dim: true, typewriter: false }));
    cursorAt(e, 5); // end of 첫 문단
    const before = focusKey.getState(e.state)!.decorations;
    // ㅇ → 아 typed at the end of the first paragraph during a composition.
    e.view.dispatch(composing(e.state.tr.insertText('ㅇ', 5)));
    e.view.dispatch(composing(e.state.tr.insertText('아', 5, 6)));
    const during = focusKey.getState(e.state)!.decorations;
    expect(during.find()).toHaveLength(1);
    expect(during.find()[0].from).toBe(before.find()[0].from);
    expect(current(e)).toEqual(['첫 문단아']);
    e.view.dispatch(e.state.tr.setMeta(COMPOSITION_END, true));
    expect(current(e)).toEqual(['첫 문단아']);

    // Even when the selection moves during a composition, the mark waits for its end.
    e.view.dispatch(composing(e.state.tr.setSelection(TextSelection.create(e.state.doc, 10))));
    expect(current(e)).toEqual(['첫 문단아']);
    e.view.dispatch(e.state.tr.setMeta(COMPOSITION_END, true));
    expect(current(e)).toEqual(['둘째 문단']);
  });

  it('is set from 보기 설정 without adding to the undo history', () => {
    const e = editor();
    (e as unknown as { view: { hasFocus: () => boolean } }).view.hasFocus = () => false;
    setFocusLook(e, { dim: true, typewriter: true });
    expect(focusKey.getState(e.state)).toMatchObject({ dim: true, typewriter: true });
    setFocusLook(e, null);
    expect(focusKey.getState(e.state)).toMatchObject({ dim: false, typewriter: false });
  });
});

describe('typewriter scrolling', () => {
  it('scrolls the caret to the same height of the page', () => {
    // A 1000px tall scroller from y=100: the line belongs at 100 + 420.
    expect(typewriterDelta(520, 100, 1000)).toBe(0);
    expect(typewriterDelta(900, 100, 1000)).toBe(900 - (100 + 1000 * TYPEWRITER_AT));
    expect(typewriterDelta(200, 100, 1000)).toBeLessThan(0);
  });
});

describe('Esc during a composition', () => {
  it('is the input method’s, not a key to leave 집중 모드', () => {
    expect(keyInComposition({ isComposing: true })).toBe(true);
    expect(keyInComposition({ keyCode: 229 })).toBe(true);
    expect(keyInComposition({}, { composing: true, isDestroyed: false })).toBe(true);
    expect(keyInComposition({}, { composing: false, isDestroyed: false })).toBe(false);
    expect(keyInComposition({ keyCode: 27 }, null)).toBe(false);
  });
});
