import { describe, expect, it } from 'vitest';
import type { Applied, ChapterReview, Correction, ExchangeInfo } from '../api/types';
import {
  NO_CHOICES,
  acceptClass,
  acceptRemaining,
  appliedText,
  changeView,
  chooseChange,
  chooseNote,
  classCounts,
  clearClass,
  decisionsOf,
  exchangeStateText,
  howText,
  keepOpen,
  remaining,
  reviewParagraphs,
  sentChaptersText,
  visible,
} from './review';

const span = (b1: number, o1: number, b2 = b1, o2 = o1) => ({ from: { block: b1, offset: o1 }, to: { block: b2, offset: o2 } });

function change(id: string, over: Partial<Correction>): Correction {
  return {
    id,
    kind: 'replace',
    class: 'wording',
    before: '',
    after: '',
    lead: '',
    trail: '',
    at: span(0, 0),
    now: span(0, 0),
    overlap: false,
    how: 'compared',
    author: null,
    date: null,
    state: 'pending',
    ...over,
  };
}

function chapter(changes: Correction[], extra: Partial<ChapterReview> = {}): ChapterReview {
  return { docId: 'd', title: '1화', found: true, gone: false, edited: false, changes, looks: [], notes: [], ...extra };
}

/** Pieces as `text`, `-deleted-`, `+inserted+`. */
const show = (rows: ReturnType<typeof reviewParagraphs>) =>
  rows.map((pieces) => pieces.map((p) => (p.kind === 'del' ? `-${p.text}-` : p.kind === 'ins' ? `+${p.text}+` : p.text)).join(''));

describe('reviewParagraphs', () => {
  it('puts deleted and inserted text where the change is now', () => {
    const rows = reviewParagraphs(
      ['그는 문을 열었다', '비가 올것 같았다.'],
      chapter([
        change('c1', { before: '는', after: '가', now: span(0, 1, 0, 2) }),
        change('c2', { kind: 'insert', after: '.', now: span(0, 9) }),
        change('c3', { kind: 'insert', class: 'spacing', after: ' ', now: span(1, 4) }),
      ]),
    );
    expect(show(rows)).toEqual(['그-는-+가+ 문을 열었다+.+', '비가 올+ +것 같았다.']);
    expect(rows[0][1]).toMatchObject({ kind: 'del', change: 'c1' });
  });

  it('shows a joined paragraph break at the end of the first paragraph', () => {
    const rows = reviewParagraphs(
      ['첫 문장이다.', '이어지는 문장.'],
      chapter([change('c1', { kind: 'merge', before: ' ', after: ' ', now: span(0, 7, 1, 0) })]),
    );
    expect(show(rows)).toEqual(['첫 문장이다.-¶-+ +', '이어지는 문장.']);
    expect(rows[0][1].para).toBe(true);
  });

  it('leaves out decided changes and 겹침', () => {
    const rows = reviewParagraphs(
      ['가나다'],
      chapter([
        change('c1', { before: '나', after: '너', now: span(0, 1, 0, 2), state: 'accepted' }),
        change('c2', { before: '다', after: '', now: null, overlap: true }),
      ]),
    );
    expect(show(rows)).toEqual(['가나다']);
  });

  it('marks looks and notes, with their start once', () => {
    const rows = reviewParagraphs(
      ['편집자가 밑줄 친 문장.'],
      chapter([change('c1', { before: '친', after: '그은', now: span(0, 8, 0, 9) })], {
        looks: [{ id: 'l1', at: span(0, 5, 0, 9), now: span(0, 5, 0, 9), quote: '밑줄 친', underline: true, color: false, highlight: false }],
        notes: [
          { id: 'n1', text: '좋아요', author: '김편집', date: '', at: span(0, 0, 0, 13), now: span(0, 0, 0, 13), quote: '', state: 'pending', memo: null },
        ],
      }),
    );
    const pieces = rows[0];
    expect(show(rows)).toEqual(['편집자가 밑줄 -친-+그은+ 문장.']);
    expect(pieces[0]).toMatchObject({ text: '편집자가 ', looks: [], notes: ['n1'], starts: ['n1'] });
    expect(pieces[1]).toMatchObject({ text: '밑줄 ', looks: ['l1'], starts: ['l1'] });
    expect(pieces[2]).toMatchObject({ kind: 'del', looks: ['l1'], starts: [] });
    // Inserted text inside a look stays in it only when the look goes on after it.
    expect(pieces[3]).toMatchObject({ kind: 'ins', looks: [], notes: ['n1'] });
  });

  it('keeps empty paragraphs and clamps places past the end', () => {
    const rows = reviewParagraphs(['', '짧다'], chapter([change('c1', { kind: 'insert', after: '!', now: span(1, 9) })]));
    expect(show(rows)).toEqual(['', '짧다+!+']);
  });
});

describe('choices', () => {
  const changes = [
    change('a', { class: 'spacing' }),
    change('b', { class: 'spacing', overlap: true, now: null }),
    change('c', { class: 'punctuation' }),
    change('d', { class: 'spacing', state: 'accepted' }),
  ];

  it('accepts a whole class, leaving 겹침 and decided changes', () => {
    const picked = acceptClass(NO_CHOICES, changes, 'spacing');
    expect(picked.changes).toEqual({ a: 'accept' });
    const counts = classCounts(changes, picked);
    expect(counts.spacing).toEqual({ appliable: 1, accepting: 1, open: 1 });
    expect(counts.punctuation).toEqual({ appliable: 1, accepting: 0, open: 1 });
    expect(remaining(changes, picked)).toBe(2);
    expect(clearClass(picked, changes, 'spacing').changes).toEqual({});
  });

  it('accepts what is left and forgets picks that no longer apply', () => {
    const picked = acceptRemaining(chooseChange(NO_CHOICES, 'c', 'reject'), changes);
    expect(picked.changes).toEqual({ a: 'accept', c: 'reject' });
    const kept = keepOpen(chooseNote(chooseChange(picked, 'd', 'accept'), 'gone', 'keep'), [chapter(changes)]);
    expect(kept).toEqual({ changes: { a: 'accept', c: 'reject' }, notes: {} });
  });

  it('tells each change apart and turns choices into decisions', () => {
    let picked = chooseChange(NO_CHOICES, 'a', 'accept');
    picked = chooseChange(picked, 'b', 'reject');
    picked = chooseNote(picked, 'n1', 'keep');
    picked = chooseNote(picked, 'n2', 'drop');
    expect(changeView(changes[0], picked)).toBe('accept');
    expect(changeView(changes[2], picked)).toBe('open');
    expect(changeView(changes[3], picked)).toBe('accepted');
    expect(decisionsOf(picked)).toEqual({ accept: ['a'], reject: ['b'], keepNotes: ['n1'], dropNotes: ['n2'] });
    expect(chooseChange(picked, 'a', null).changes).toEqual({ b: 'reject' });
  });
});

describe('the list of what was sent', () => {
  const ex: ExchangeInfo = {
    id: 'x',
    created: '2026-10-03T03:00:00Z',
    kind: 'hwpx',
    files: ['달빛.hwpx'],
    sceneBreak: '◆',
    chapters: [
      { docId: 'a', title: '문 닫는 시간', heading: '1화 문 닫는 시간', file: '달빛.hwpx', fingerprint: '' },
      { docId: 'b', title: '빗소리', heading: '2화 빗소리', file: '달빛.hwpx', fingerprint: '' },
    ],
    received: [],
    pending: null,
  };

  it('names the chapters and says where it stands', () => {
    expect(sentChaptersText(ex)).toBe('1화 문 닫는 시간 외 1개');
    expect(sentChaptersText({ ...ex, chapters: ex.chapters.slice(1) })).toBe('2화 빗소리');
    expect(exchangeStateText(ex)).toBe('교정본을 아직 받지 않았습니다.');
    const back = { ...ex, received: [{ at: '2026-10-05T03:00:00Z', name: '교정본.hwpx', stored: '', fingerprint: '' }] };
    expect(exchangeStateText({ ...back, pending: 3 })).toBe('10월 5일에 받은 교정본(교정본.hwpx)에 아직 정하지 않은 곳이 3군데 있습니다.');
    expect(exchangeStateText({ ...back, pending: 0 })).toBe('10월 5일에 받은 교정본(교정본.hwpx)을 모두 살펴보았습니다.');
  });

  it('says how a change was marked', () => {
    expect(howText(change('a', { how: 'tracked', author: '김편집', date: null }))).toBe('변경 내용 추적 · 김편집');
    expect(howText(change('a', { how: 'strike' }))).toBe('취소선으로 표시');
    expect(howText(change('a', { how: 'compared' }))).toBe('');
  });
});

describe('words', () => {
  it('shows breaks as marks', () => {
    expect(visible('가 나\n다', '***')).toBe('가¶나↵다***');
  });

  it('says what was applied', () => {
    const applied: Applied = {
      docs: ['d'],
      accepted: ['a', 'b'],
      rejected: ['c'],
      skipped: [{ id: 'e', reason: '…' }],
      memos: ['m'],
      review: { exchange: 'x', file: 'f', stored: 's', at: '', chapters: [] },
    };
    expect(appliedText(applied)).toBe(
      '바뀐 곳 2군데를 받아들이고 1군데는 원래대로 두었습니다. 편집자 메모 1개를 메모로 남겼습니다. 1군데는 반영하지 못해 직접 고쳐야 합니다. 고치기 전 원고는 기록에 ‘교정 반영 전’으로 남았습니다.',
    );
    expect(appliedText({ ...applied, docs: [], accepted: [], rejected: [], skipped: [], memos: [] })).toBe('반영할 것이 없었습니다.');
  });
});
