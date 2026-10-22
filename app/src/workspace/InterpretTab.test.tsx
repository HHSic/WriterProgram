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
    interpretPick: null as { docId: string; block: number; offset: number } | null,
  },
  load: vi.fn(),
  pick: vi.fn(),
  jump: vi.fn(),
  setMode: vi.fn(),
}));
vi.mock('../store', () => ({
  useApp: (select: (s: typeof mock.state) => unknown) => select(mock.state),
  loadInterpretation: mock.load,
  pickSentence: mock.pick,
  jumpTo: mock.jump,
  setInterpretMode: mock.setMode,
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
  gone: [],
  changed: false,
};

let root: Root;
beforeEach(() => {
  vi.clearAllMocks();
  mock.state.interpretMode = true;
  mock.state.interpret = { docId: 'd1', view };
  mock.state.interpretPick = null;
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
