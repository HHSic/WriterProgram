import { beforeEach, expect, it, vi } from 'vitest';
import type { Interpretation, InterpretView, Overview } from '../api/types';
import { useApp } from './state';
import { finishLinking, newSentence, removeIncoming, removeSentence, saveSentence, startLinking } from './interpret';

const mock = vi.hoisted(() => ({ load: vi.fn(), save: vi.fn(), toast: vi.fn(), error: vi.fn() }));
vi.mock('../api', () => ({ api: { interpretLoad: mock.load, interpretSave: mock.save } }));
vi.mock('./ui', () => ({ showToast: mock.toast, toastError: mock.error }));

const reading = (doc: string, sentences: Interpretation['sentences']): Interpretation => ({
  doc, rev: 'r', made: '2026-10-21T13:00:00Z', by: 'Claude', summary: '', scenes: [], sentences,
});
const viewOf = (r: Interpretation, places: InterpretView['places'] = []): InterpretView => ({
  reading: r, read: true, places, gone: [], changed: false, remote: [], incoming: [],
});
const one = reading('d1', [
  { id: 'a', block: 0, quote: '손을 뻗으려다 말았다.', roles: ['복선'], meaning: '', intent: '', links: [] },
  { id: 'b', block: 2, quote: '손이 머뭇거리지 않았다.', roles: [], meaning: '', intent: '', links: [{ to: 'a', kind: '회수', note: '' }] },
]);
const saved = () => mock.save.mock.calls.at(-1)![1] as Interpretation;

beforeEach(() => {
  vi.clearAllMocks();
  useApp.setState({
    overview: { root: '/p' } as Overview,
    interpret: { docId: 'd1', view: viewOf(one, [{ id: 'a', block: 0, start: 0, end: 12 }, { id: 'b', block: 2, start: 0, end: 13 }]) },
    interpretLinking: null,
  });
  mock.save.mockImplementation(async (_root: string, r: Interpretation) => viewOf(r));
});

it('keeps a changed sentence and shows what was saved', async () => {
  await saveSentence('d1', { ...one.sentences[0], meaning: '망설임' });
  expect(saved().sentences[0].meaning).toBe('망설임');
  expect(useApp.getState().interpret?.view?.reading.sentences[0].meaning).toBe('망설임');
  // The loaded reading itself is not changed in place.
  expect(one.sentences[0].meaning).toBe('');
});

it('adds a sentence the writer read under a free id', async () => {
  const s = newSentence('d1', { block: 4, start: 0, end: 8, text: '바람이 불었다.' });
  expect(s).toMatchObject({ id: 'w3', block: 4, quote: '바람이 불었다.' });
  await saveSentence('d1', s);
  expect(saved().sentences.map((x) => x.id)).toEqual(['a', 'b', 'w3']);
});

it('drops a sentence with the links in the chapter pointing at it', async () => {
  await removeSentence('d1', 'a');
  expect(saved().sentences).toEqual([{ ...one.sentences[1], links: [] }]);
});

it('links two sentences of a chapter, reading the target when it had none', async () => {
  startLinking('d1', one.sentences[0]);
  await finishLinking('d1', { block: 2, start: 2, end: 6, text: '머뭇거리지' });
  expect(saved().sentences[0].links).toEqual([{ to: 'b', kind: '회수', note: '' }]);

  useApp.setState({ interpret: { docId: 'd1', view: viewOf(saved(), useApp.getState().interpret!.view!.places) } });
  startLinking('d1', one.sentences[0]);
  await finishLinking('d1', { block: 4, start: 0, end: 8, text: '바람이 불었다.' });
  expect(saved().sentences.at(-1)).toMatchObject({ id: 'w3', quote: '바람이 불었다.' });
  expect(saved().sentences[0].links.at(-1)).toEqual({ to: 'w3', kind: '회수', note: '' });
  expect(useApp.getState().interpretLinking).toBeNull();
});

it('does not link a sentence to itself', async () => {
  startLinking('d1', one.sentences[0]);
  expect(await finishLinking('d1', { block: 0, start: 0, end: 12, text: '손을 뻗으려다 말았다.' })).toBe(false);
  expect(mock.save).not.toHaveBeenCalled();
  expect(useApp.getState().interpretLinking).not.toBeNull();
});

it('links into another chapter from the reading the link started in', async () => {
  startLinking('d1', one.sentences[0]);
  // The writer moved to chapter 5; its reading is what is loaded now.
  useApp.setState({ interpret: { docId: 'd5', view: null } });
  mock.load.mockImplementation(async (_root: string, doc: string) => (doc === 'd1' ? viewOf(structuredClone(one)) : null));
  useApp.setState({ interpretLinking: { ...useApp.getState().interpretLinking!, note: ' 회차를 넘어 ' } });
  expect(await finishLinking('d5', { block: 3, start: 0, end: 9, text: '서랍이 두꺼워졌다.' })).toBe(true);
  expect(mock.save).toHaveBeenCalledWith('/p', expect.objectContaining({ doc: 'd1' }));
  expect(saved().sentences[0].links).toEqual([{ to: '', kind: '회수', note: '회차를 넘어', doc: 'd5', quote: '서랍이 두꺼워졌다.' }]);
  // Chapter 5 is read again to show the link coming in.
  expect(mock.load).toHaveBeenLastCalledWith('/p', 'd5');
});

it('drops a link coming in by changing the reading it comes from', async () => {
  const from = reading('d1', [{ ...one.sentences[0], links: [{ to: '', kind: '회수', note: '', doc: 'd5', quote: '서랍이 두꺼워졌다.' }] }]);
  mock.load.mockImplementation(async (_root: string, doc: string) => (doc === 'd1' ? viewOf(from) : null));
  useApp.setState({ interpret: { docId: 'd5', view: null } });
  await removeIncoming('d5', {
    doc: 'd1', title: '1화', from: 'a', fromQuote: '손을 뻗으려다 말았다.', fromSpot: null,
    kind: '회수', note: '', quote: '서랍이 두꺼워졌다.', place: null,
  });
  expect(saved()).toMatchObject({ doc: 'd1' });
  expect(saved().sentences[0].links).toEqual([]);
});
