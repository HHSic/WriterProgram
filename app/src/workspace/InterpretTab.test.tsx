// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { InterpretView } from '../api/types';
import { InterpretTab } from './InterpretTab';

const mock = vi.hoisted(() => ({
  state: {
    interpretMode: true,
    interpretVersion: 0,
    interpret: null as { docId: string; view: InterpretView | null } | null,
    interpretPick: null as { docId: string; block: number; offset: number; sentence?: { block: number; start: number; end: number; text: string } | null } | null,
    interpretLinking: null as { fromDoc: string; fromId: string; fromQuote: string; kind: string; note: string } | null,
  },
  load: vi.fn(),
  pick: vi.fn(),
  jump: vi.fn(),
  setMode: vi.fn(),
  save: vi.fn(),
  remove: vi.fn(),
  unlink: vi.fn(),
  unlinkIncoming: vi.fn(),
  startLinking: vi.fn(),
  finishLinking: vi.fn(),
}));
vi.mock('../store', () => ({
  useApp: (select: (s: typeof mock.state) => unknown) => select(mock.state),
  loadInterpretation: mock.load,
  pickSentence: mock.pick,
  jumpTo: mock.jump,
  setInterpretMode: mock.setMode,
  emptyReading: (doc: string) => ({ doc, rev: '', made: '', by: '', summary: '', scenes: [], sentences: [] }),
  newSentence: (_doc: string, t: { block: number; text: string }) => ({ id: 'w1', block: t.block, quote: t.text, roles: [], meaning: '', intent: '', links: [] }),
  saveSentence: mock.save,
  removeSentence: mock.remove,
  removeLink: mock.unlink,
  removeIncoming: mock.unlinkIncoming,
  startLinking: mock.startLinking,
  finishLinking: mock.finishLinking,
  setLinking: vi.fn(),
  cancelLinking: vi.fn(),
}));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const view: InterpretView = {
  reading: {
    doc: 'd1',
    rev: 'r',
    made: '2026-10-21T13:00:00Z',
    by: 'Claude',
    summary: '망설임에서 행동으로.',
    scenes: [{ block: 0, title: '첫 목요일', purpose: '하진의 소심함을 세운다' }],
    sentences: [
      { id: 'a', block: 0, quote: '손을 뻗으려다 말았다.', roles: ['복선', '인물'], meaning: '망설임', intent: '변화의 출발점', links: [{ to: 'b', kind: '회수', note: '' }] },
      { id: 'b', block: 2, quote: '손이 머뭇거리지 않았다.', roles: ['회수'], meaning: '', intent: '', links: [] },
      { id: 'c', block: 4, quote: '바람이 불었다.', roles: ['분위기'], meaning: '', intent: '', links: [], unclear: '앞 문장과 겹친다' },
    ],
  },
  places: [
    { id: 'a', block: 0, start: 0, end: 12 },
    { id: 'b', block: 2, start: 0, end: 13 },
    { id: 'c', block: 4, start: 0, end: 8 },
  ],
  read: true,
  gone: [],
  changed: false,
  remote: [],
  incoming: [],
};

let root: Root;
beforeEach(() => {
  vi.clearAllMocks();
  mock.state.interpretMode = true;
  mock.state.interpret = { docId: 'd1', view };
  mock.state.interpretPick = null;
  mock.state.interpretLinking = null;
  mock.save.mockResolvedValue(true);
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => {
  act(() => root.unmount());
  document.body.replaceChildren();
});

async function show() {
  await act(async () => root.render(<InterpretTab docId="d1" />));
}
const buttonWith = (text: string) => [...document.querySelectorAll('button')].find((b) => b.textContent?.includes(text))!;

it('shows the picked sentence with its roles, meaning, intent and links', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 0, offset: 4 };
  await show();
  const text = document.body.textContent!;
  expect(text).toContain('첫 목요일');
  expect(text).toContain('복선');
  expect(text).toContain('의도변화의 출발점');
  await act(async () => buttonWith('손이 머뭇거리지').click());
  expect(mock.jump).toHaveBeenCalledWith({ docId: 'd1', block: 2, start: 0, end: 13 });
  expect(mock.pick).toHaveBeenCalledWith('d1', 2, 0);
});

it('shows links coming in to the picked sentence', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 2, offset: 1 };
  await show();
  expect(document.body.textContent).toContain('← 회수');
});

it('shows the whole reading when nothing is picked, unclear sentences included', async () => {
  await show();
  const text = document.body.textContent!;
  expect(text).toContain('망설임에서 행동으로.');
  expect(text).toContain('연결 1');
  expect(text).toContain('의도가 흐릴 수 있는 문장 1');
  expect(text).toContain('앞 문장과 겹친다');
});

it('says how to make a reading when there is none, and loads it when the mode is off', async () => {
  mock.state.interpretMode = false;
  mock.state.interpret = { docId: 'd1', view: null };
  await show();
  expect(document.body.textContent).toContain('아직 해석하지 않았습니다');
  expect(mock.load).toHaveBeenCalledWith('d1');
  await act(async () => buttonWith('해석 모드 켜기').click());
  expect(mock.setMode).toHaveBeenCalledWith(true);
});

const across: InterpretView = {
  ...view,
  remote: [{ from: 'a', link: 1, doc: 'd5', title: '5화', quote: '서랍이 두꺼워졌다.', kind: '회수', note: '', spot: { doc: 'd5', title: '5화', block: 3, start: 0, end: 9 } }],
  incoming: [{
    doc: 'd0', title: '프롤로그', from: 'p1', fromQuote: '대문은 늘 잠겨 있었다.', fromSpot: { doc: 'd0', title: '프롤로그', block: 1, start: 0, end: 12 },
    kind: '복선', note: '', quote: '손을 뻗으려다 말았다.', place: { id: '', block: 0, start: 0, end: 12 },
  }],
};

it('shows links into other chapters and coming from them, and goes there', async () => {
  mock.state.interpret = { docId: 'd1', view: across };
  mock.state.interpretPick = { docId: 'd1', block: 0, offset: 4 };
  await show();
  const text = document.body.textContent!;
  expect(text).toContain('「5화」');
  expect(text).toContain('「프롤로그」');
  await act(async () => buttonWith('서랍이 두꺼워졌다').click());
  expect(mock.jump).toHaveBeenCalledWith({ docId: 'd5', block: 3, start: 0, end: 9 });
  expect(mock.pick).not.toHaveBeenCalled();
  await act(async () => document.querySelectorAll<HTMLButtonElement>('[aria-label="연결 지우기"]')[1].click());
  expect(mock.unlink).toHaveBeenCalledWith('d1', 'a', 1);
  await act(async () => document.querySelectorAll<HTMLButtonElement>('[aria-label="연결 지우기"]')[2].click());
  expect(mock.unlinkIncoming).toHaveBeenCalledWith('d1', across.incoming[0]);
});

it('lists the links across chapters in the whole reading', async () => {
  mock.state.interpret = { docId: 'd1', view: across };
  await show();
  expect(document.body.textContent).toContain('다른 회차와의 연결 2');
});

it('lets the writer change the reading of a sentence', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 0, offset: 4 };
  await show();
  await act(async () => buttonWith('고치기').click());
  const meaning = document.querySelector<HTMLTextAreaElement>('textarea')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!.call(meaning, '  더 깊은 망설임 ');
    meaning.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await act(async () => buttonWith('인물').click());
  await act(async () => buttonWith('저장').click());
  expect(mock.save).toHaveBeenCalledWith('d1', expect.objectContaining({ id: 'a', meaning: '더 깊은 망설임', roles: ['복선'] }));
  expect(document.querySelector('form')).toBeNull();
});

it('lets the writer read a sentence that has no reading', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 6, offset: 1, sentence: { block: 6, start: 0, end: 7, text: '비가 왔다.' } };
  await show();
  await act(async () => buttonWith('이 문장 해석 추가').click());
  expect(document.querySelector('blockquote')?.textContent).toBe('비가 왔다.');
  await act(async () => buttonWith('해석 남기기').click());
  expect(mock.save).toHaveBeenCalledWith('d1', expect.objectContaining({ id: 'w1', block: 6, quote: '비가 왔다.' }));
});

it('asks before taking a reading out', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 0, offset: 4 };
  await show();
  await act(async () => buttonWith('지우기').click());
  expect(mock.remove).not.toHaveBeenCalled();
  await act(async () => buttonWith('해석 지우기').click());
  expect(mock.remove).toHaveBeenCalledWith('d1', 'a');
});

it('links the sentence picked while 연결 추가 is under way', async () => {
  mock.state.interpretPick = { docId: 'd1', block: 0, offset: 4 };
  await show();
  await act(async () => buttonWith('연결 추가').click());
  expect(mock.startLinking).toHaveBeenCalledWith('d1', view.reading.sentences[0]);

  const target = { block: 6, start: 0, end: 7, text: '비가 왔다.' };
  mock.state.interpretLinking = { fromDoc: 'd1', fromId: 'a', fromQuote: '손을 뻗으려다 말았다.', kind: '회수', note: '' };
  mock.state.interpretPick = { docId: 'd1', block: 6, offset: 1, sentence: target };
  await show();
  expect(document.body.textContent).toContain('이을 문장“비가 왔다.”');
  await act(async () => buttonWith('이 문장과 잇기').click());
  expect(mock.finishLinking).toHaveBeenCalledWith('d1', target);
});
