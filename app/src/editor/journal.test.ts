import { describe, expect, it, vi } from 'vitest';
import { Schema } from '@tiptap/pm/model';
import { EditorState, type Transaction } from '@tiptap/pm/state';
import { api } from '../api';
import { editCounts, endSessions, fromInside, noteComposed, noteEdit, noteInAppCopy } from './journal';

vi.mock('../api', () => ({ api: { journalEvent: vi.fn(async () => {}) } }));

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

describe('writing sessions', () => {
  /** Applies edits one after another; `make` builds each from the state so far. */
  function run(start: EditorState, ...make: ((s: EditorState) => Transaction)[]): Transaction[] {
    let s = start;
    return make.map((m) => {
      const tr = m(s);
      s = s.apply(tr);
      return tr;
    });
  }

  async function sent() {
    const calls = vi.mocked(api.journalEvent).mock.calls;
    await endSessions();
    const last = calls[calls.length - 1]?.[1];
    vi.mocked(api.journalEvent).mockClear();
    return last && last.kind === 'session' ? { inserted: last.inserted, deleted: last.deleted } : null;
  }

  const composed = (tr: Transaction) => tr.setMeta('composition', 1);

  it('count a composition once it ends', async () => {
    // 한글: ㅎ → 하 → 한, then ㄱ → 그 → 글; a jamo taken back on the way.
    const trs = run(
      state(''),
      (s) => composed(s.tr.insertText('ㅎ', 1)),
      (s) => composed(s.tr.insertText('하', 1, 2)),
      (s) => composed(s.tr.insertText('한', 1, 2)),
      (s) => composed(s.tr.insertText('한ㄱ', 1, 2)),
      (s) => composed(s.tr.insertText('한', 1, 3)),
      (s) => composed(s.tr.insertText('한ㄱ', 1, 2)),
      (s) => composed(s.tr.insertText('그', 2, 3)),
      (s) => composed(s.tr.insertText('글', 2, 3)),
    );
    for (const tr of trs) noteEdit('root', 'doc', tr, true);
    noteComposed('root', 'doc');
    expect(await sent()).toEqual({ inserted: 2, deleted: 0 });
  });

  it('count a composition when the next plain edit comes', async () => {
    const trs = run(
      state('글'),
      (s) => composed(s.tr.insertText('ㅆ', 2)),
      (s) => composed(s.tr.insertText('써', 2, 3)),
      (s) => s.tr.insertText(' ', 3),
    );
    noteEdit('root', 'doc', trs[0], true);
    noteEdit('root', 'doc', trs[1], true);
    noteEdit('root', 'doc', trs[2], false);
    expect(await sent()).toEqual({ inserted: 2, deleted: 0 });
  });

  it('count a composition still open when the session ends', async () => {
    const trs = run(state(''), (s) => composed(s.tr.insertText('ㄷ', 1)));
    noteEdit('root', 'doc', trs[0], true);
    expect(await sent()).toEqual({ inserted: 1, deleted: 0 });
  });
});
