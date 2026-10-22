import { describe, expect, it } from 'vitest';
import { TextSelection } from '@tiptap/pm/state';
import { DecorationSet } from '@tiptap/pm/view';
import { InterpretHighlight, cursorPlace, placeAt, setInterpretMarks } from './interpret';
import { chapter, fakeEditor } from './testing';

const plugins = InterpretHighlight.config.addProseMirrorPlugins!.call({} as never);

describe('해석 모드 in the editor', () => {
  it('finds the block and offset under the cursor', () => {
    const doc = chapter('손을 뻗으려다 말았다.', null, '😀 손이 머뭇거리지 않았다.');
    const editor = fakeEditor(doc, []);
    // Inside the third block, after the emoji (two UTF-16 units) and a space.
    let pos = 0;
    for (let i = 0; i < 2; i += 1) pos += doc.child(i).nodeSize;
    const at = (offset: number) => TextSelection.create(doc, pos + 1 + offset);
    editor.state = editor.state.apply(editor.state.tr.setSelection(at(3)));
    expect(cursorPlace(editor.state)).toEqual({ block: 2, offset: 3 });
  });

  it('picks the sentence a place covers', () => {
    const places = [
      { id: 'a', block: 0, start: 0, end: 12 },
      { id: 'b', block: 2, start: 3, end: 15 },
    ];
    expect(placeAt(places, 2, 5)?.id).toBe('b');
    expect(placeAt(places, 2, 15)?.id).toBe('b');
    expect(placeAt(places, 1, 0)).toBeNull();
  });

  it('marks the sentences and the one looked at', () => {
    const doc = chapter('손을 뻗으려다 말았다. 비가 왔다.', '손이 머뭇거리지 않았다.');
    const editor = fakeEditor(doc, plugins);
    setInterpretMarks(editor, {
      places: [
        { id: 'a', block: 0, start: 0, end: 12 },
        { id: 'b', block: 1, start: 0, end: 13 },
      ],
      current: 'b',
    });
    const set = plugins[0].getState(editor.state) as DecorationSet;
    const found = set.find().map((d) => (d as unknown as { type: { attrs: { class: string } } }).type.attrs.class);
    expect(found).toEqual(['interpret-mark', 'interpret-mark on']);
    setInterpretMarks(editor, null);
    expect((plugins[0].getState(editor.state) as DecorationSet).find()).toHaveLength(0);
  });
});
