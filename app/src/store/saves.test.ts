// Saving never fails silently (docs/safety-design.md S7): tries again with
// growing waits, keeps a state per target, never drops an edited value, keeps
// a rescue copy, and holds the window open while something is not saved.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  docSave: vi.fn(),
  rescueSave: vi.fn(),
  rescueFolder: vi.fn(),
  reveal: vi.fn(),
}));
vi.mock('../api', () => ({ api }));

import { SaveSession } from '../editor/session';
import { debouncedSaver } from '../lib/useDebouncedSave';
import { RETRY_DELAYS, markFailed, markSaved, registerSave, resetSaves, retryDelay, worstSave } from './saves';
import { get, set } from './state';
import { confirmClose } from './ui';

const OUTCOME = { rev: 'r2', conflict: false, snapshot: null, counts: { withSpaces: 0, withoutSpaces: 0 }, pages: 0 };

/** Lets the promise chains of a save run. */
async function settle() {
  for (let i = 0; i < 10; i++) await Promise.resolve();
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-03T12:00:00Z'));
  resetSaves();
  set({ overview: { project: { id: 'proj1' } } as never, toast: null, closeAsk: null });
  for (const fn of Object.values(api)) fn.mockReset();
});

afterEach(() => {
  resetSaves();
  vi.useRealTimers();
});

describe('retry waits', () => {
  it('grows 2, 5, 15, 30, 60 seconds, then stays at 60', () => {
    expect(RETRY_DELAYS).toEqual([2000, 5000, 15000, 30000, 60000]);
    expect([1, 2, 3, 4, 5, 6, 9].map(retryDelay)).toEqual([2000, 5000, 15000, 30000, 60000, 60000, 60000]);
  });

  it('tries a failing chapter again on that schedule until it saves', async () => {
    api.docSave.mockRejectedValue('디스크 공간 부족');
    const session = new SaveSession('C:/작품', 'doc1', () => ({ type: 'doc', content: [] }), () => {}, 'r1');
    session.changed();
    await vi.advanceTimersByTimeAsync(800);
    expect(api.docSave).toHaveBeenCalledTimes(1);
    const failed = get().saves['doc:doc1'];
    expect(failed.state).toBe('error');
    expect(failed.error).toContain('디스크가 가득 찼습니다');
    expect(failed.retryAt).toBe(Date.now() + 2000);

    const calls = [1];
    for (const wait of [2000, 5000, 15000, 30000, 60000, 60000]) {
      await vi.advanceTimersByTimeAsync(wait - 1);
      expect(api.docSave).toHaveBeenCalledTimes(calls.length);
      await vi.advanceTimersByTimeAsync(1);
      calls.push(1);
      expect(api.docSave).toHaveBeenCalledTimes(calls.length);
    }

    api.docSave.mockResolvedValue(OUTCOME);
    await vi.advanceTimersByTimeAsync(60000);
    expect(get().saves['doc:doc1'].state).toBe('saved');
    expect(session.pending).toBe(false);
    // No more tries once saved.
    const count = api.docSave.mock.calls.length;
    await vi.advanceTimersByTimeAsync(120000);
    expect(api.docSave).toHaveBeenCalledTimes(count);
    await session.dispose();
  });
});

describe('state per target', () => {
  it('shows the worst: a saved card does not hide a failed chapter', () => {
    registerSave('doc:a', { retry: async () => {}, pending: () => true });
    registerSave('card:b', { retry: async () => {}, pending: () => false });
    markFailed('doc:a', '이 위치에 쓸 권한이 없음');
    markSaved('card:b');
    const worst = worstSave(get().saves);
    expect(worst.state).toBe('error');
    expect(worst.error).toContain('작품 폴더에 쓸 수 없습니다');
    expect(worst.failing).toBe(1);

    markSaved('doc:a');
    expect(worstSave(get().saves).state).toBe('saved');
  });

  it('shows saving over saved', () => {
    expect(worstSave({ a: { state: 'saved' }, b: { state: 'saving' } }).state).toBe('saving');
    expect(worstSave({ a: { state: 'error', since: 1 }, b: { state: 'saving' } }).state).toBe('error');
  });
});

describe('edited values are never dropped', () => {
  it('keeps a failed title and joins it with a synopsis typed since', async () => {
    const save = vi.fn().mockRejectedValueOnce('파일이 다른 프로그램에 잡혀 있음').mockResolvedValue(undefined);
    type Meta = { title?: string; synopsis?: string };
    const saver = debouncedSaver<Meta>({ key: 'meta:d1', save, merge: (failed, newer) => ({ ...failed, ...newer }) });
    registerSave('meta:d1', saver.target);

    saver.schedule({ title: '새 제목' });
    await vi.advanceTimersByTimeAsync(600);
    expect(save).toHaveBeenLastCalledWith({ title: '새 제목' });
    expect(get().saves['meta:d1'].state).toBe('error');
    expect(get().saves['meta:d1'].error).toContain('다른 프로그램');
    expect(saver.pending).toEqual({ title: '새 제목' });

    saver.schedule({ ...saver.pending, synopsis: '줄거리' });
    await vi.advanceTimersByTimeAsync(600);
    expect(save).toHaveBeenLastCalledWith({ title: '새 제목', synopsis: '줄거리' });
    expect(saver.pending).toBeNull();
    expect(get().saves['meta:d1'].state).toBe('saved');
  });

  it('puts a value that failed on its way back under one changed meanwhile', async () => {
    let fail!: (e: unknown) => void;
    const save = vi.fn().mockImplementationOnce(() => new Promise((_, reject) => (fail = reject)));
    type Meta = { title?: string; synopsis?: string };
    const saver = debouncedSaver<Meta>({ key: 'meta:d2', save, merge: (failed, newer) => ({ ...failed, ...newer }) });
    registerSave('meta:d2', saver.target);

    saver.schedule({ title: '하나', synopsis: '처음' });
    void saver.flush();
    await settle();
    saver.schedule({ title: '둘' });
    fail('디스크 공간 부족');
    await settle();
    expect(saver.pending).toEqual({ title: '둘', synopsis: '처음' });
  });

  it('keeps retrying a card whose screen has gone away', async () => {
    const save = vi.fn().mockRejectedValueOnce('디스크 공간 부족').mockResolvedValue(undefined);
    const saver = debouncedSaver<{ name: string }>({ key: 'card:c1', save });
    const unregister = registerSave('card:c1', saver.target);
    saver.schedule({ name: '서하' });
    void saver.flush();
    unregister();
    await settle();
    expect(get().saves['card:c1'].state).toBe('error');

    await vi.advanceTimersByTimeAsync(2000);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ name: '서하' });
    // Saved and gone: nothing left for this card.
    expect(get().saves['card:c1']).toBeUndefined();
  });
});

describe('rescue copy', () => {
  it('writes what could not be saved once saving has failed for over a minute', async () => {
    api.docSave.mockRejectedValue('디스크 공간 부족');
    api.rescueSave.mockResolvedValue('C:/rescue/proj1/doc9-20261003-210000.md');
    const body = { type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: '마지막 문장' }] }] };
    const session = new SaveSession('C:/작품', 'doc9', () => body, () => {}, 'r1');
    const started = Date.now();
    session.changed();
    await vi.advanceTimersByTimeAsync(800 + 2000 + 5000 + 15000 + 30000);
    expect(api.rescueSave).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(60000);
    expect(api.rescueSave).toHaveBeenCalledWith('proj1', 'doc9', started + 800, { body });
    expect(get().saves['doc:doc9'].rescued).toContain('doc9');
    expect(get().toast?.text).toBe('작품 폴더에 저장하지 못해 비상 위치에 보관했습니다');
    expect(get().toast?.action?.label).toBe('위치 열기');
    api.docSave.mockResolvedValue(OUTCOME);
    await session.dispose();
  });
});

describe('closing the window', () => {
  it('closes at once when everything is saved', async () => {
    registerSave('doc:x', { retry: async () => {}, pending: () => false });
    await expect(confirmClose()).resolves.toBe(true);
    expect(get().closeAsk).toBeNull();
  });

  it('asks while a target is failing, and stays open when the writer says so', async () => {
    registerSave('note:n1', { retry: async () => {}, pending: () => true });
    markFailed('note:n1', '디스크 공간 부족');
    const closing = confirmClose();
    await settle();
    const ask = get().closeAsk;
    expect(ask).not.toBeNull();
    ask!.resolve(false);
    await expect(closing).resolves.toBe(false);
    expect(get().closeAsk).toBeNull();
  });

  it('asks while edits still wait to be sent', async () => {
    registerSave('card:k', { retry: async () => {}, pending: () => true });
    const closing = confirmClose();
    await settle();
    get().closeAsk!.resolve(true);
    await expect(closing).resolves.toBe(true);
  });
});
