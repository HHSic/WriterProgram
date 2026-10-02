import { describe, expect, it } from 'vitest';
import { Schema } from '@tiptap/pm/model';
import { EditorState } from '@tiptap/pm/state';
import { editCounts, fromInside, noteInAppCopy } from './journal';

const schema = new Schema({
  nodes: {
    doc: { content: 'paragraph+' },
    paragraph: { content: 'text*' },
    text: {},
  },
});

function state(...paras: string[]) {
  const doc = schema.node(
    'doc',
    null,
    paras.map((p) => schema.node('paragraph', null, p ? [schema.text(p)] : [])),
  );
  return EditorState.create({ doc });
}

describe('editCounts', () => {
  it('counts typed and deleted characters', () => {
    const s = state('달빛 서점');
    // Positions: 1 is the start of the first paragraph's text.
    expect(editCounts(s.tr.insertText('의 손님', 6))).toEqual({ inserted: 4, deleted: 0 });
    expect(editCounts(s.tr.delete(1, 3))).toEqual({ inserted: 0, deleted: 2 });
    expect(editCounts(s.tr.insertText('별', 1, 3))).toEqual({ inserted: 1, deleted: 2 });
  });

  it('counts only the change in length while composing', () => {
    const s = state('하');
    // 하 → 한: the syllable is replaced, nothing new.
    expect(editCounts(s.tr.insertText('한', 1, 2), true)).toEqual({ inserted: 0, deleted: 0 });
    expect(editCounts(s.tr.insertText('한ㄱ', 1, 2), true)).toEqual({ inserted: 1, deleted: 0 });
    const tr = s.tr.insertText('', 1, 2).setMeta('composition', 1);
    expect(editCounts(tr)).toEqual({ inserted: 0, deleted: 1 });
  });

  it('a new paragraph is no character', () => {
    const s = state('첫 문단');
    expect(editCounts(s.tr.split(3))).toEqual({ inserted: 0, deleted: 0 });
  });
});

describe('paste from inside the app', () => {
  it('matches text copied in the app, whatever the spacing', () => {
    noteInAppCopy('셔터를 반쯤 내렸을 때\n\n종이 울렸다.');
    expect(fromInside('셔터를 반쯤 내렸을 때\n종이 울렸다.')).toBe(true);
    expect(fromInside('다른 곳에서 가져온 글')).toBe(false);
    expect(fromInside('')).toBe(false);
  });
});
