// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { DocData, Overview } from '../api/types';
import { ReaderDialog } from './ReaderDialog';

const mock = vi.hoisted(() => ({
  state: { overview: null as Overview | null, activeDocId: 'second' },
  load: vi.fn(), save: vi.fn(), close: vi.fn(),
}));
vi.mock('../api', () => ({ api: { docLoad: mock.load } }));
vi.mock('../store', () => ({ useApp: (select: (s: typeof mock.state) => unknown) => select(mock.state),
  allManuscript: (ov: Overview) => ov.parts.flatMap((p) => p.docs), saveEverything: mock.save, closeDialog: mock.close }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
let width = 900;
let pages = 5;
let resize: () => void;
function data(id: string, text = '한글 (ง •̀_•́)ง 😊 <script>원고</script>'): DocData {
  return { meta: { id, title: id === 'first' ? '첫 장' : '둘째 장' }, body: { type: 'doc', content: [
    { type: 'paragraph', content: [{ type: 'text', text, marks: [{ type: 'bold' }] }, { type: 'hardBreak' }, { type: 'text', text: '다음 줄' }] },
    { type: 'sceneBreak' }, { type: 'paragraph' },
  ] } } as DocData;
}
beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  width = 900;
  pages = 5;
  mock.state.activeDocId = 'second';
  mock.state.overview = { root: '/book', project: { id: 'book', title: '읽을 작품', kind: 'webnovel' },
    parts: [{ docs: [{ id: 'first', title: '첫 장' }, { id: 'second', title: '둘째 장' }] }],
    planning: [{ id: 'private', title: '비밀 기획' }] } as unknown as Overview;
  mock.save.mockResolvedValue(true);
  mock.load.mockImplementation(async (_root: string, id: string) => data(id));
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('reader-viewport') ? width : 0;
  });
  vi.spyOn(Element.prototype, 'getClientRects').mockImplementation(() => {
    const columns = width >= 640 ? 2 : 1;
    const columnWidth = (width - 48 * (columns - 1)) / columns;
    return [{ right: pages * (columnWidth + 48) - 48 }] as unknown as DOMRectList;
  });
  vi.stubGlobal('ResizeObserver', class { constructor(callback: () => void) { resize = callback; } observe() {} disconnect() {} });
  const host = document.createElement('div'); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); document.body.replaceChildren(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
async function show() { await act(async () => { root.render(<ReaderDialog />); }); }
function button(label: string) {
  const found = [...document.querySelectorAll('button')].find((b) => b.textContent === label || b.getAttribute('aria-label') === label);
  if (!found) throw new Error(`Missing button ${label}`);
  return found;
}
async function click(label: string) { await act(async () => button(label).click()); }
function status() { return document.querySelector('.reader-navigation [role="status"]')?.textContent; }
async function chapter(id: string) {
  await act(async () => {
    const select = document.querySelector<HTMLSelectElement>('[aria-label="읽을 원고"]')!;
    select.value = id; select.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

it('waits for saves, opens the current manuscript and keeps Unicode and text marks', async () => {
  let finish!: (ok: boolean) => void;
  mock.save.mockReturnValue(new Promise<boolean>((resolve) => { finish = resolve; }));
  await show();
  expect(mock.load).not.toHaveBeenCalled();
  expect(document.body.textContent).not.toContain('비밀 기획');
  await act(async () => finish(true));
  expect(mock.load).toHaveBeenCalledExactlyOnceWith('/book', 'second');
  expect(document.querySelector('.reader-pages strong')?.textContent).toBe('한글 (ง •̀_•́)ง 😊 <script>원고</script>');
  expect(document.querySelector('.reader-pages script')).toBeNull();
  expect(document.querySelector('.reader-pages br')).not.toBeNull();
  expect(document.querySelector('[aria-label="장면 나눔"]')).not.toBeNull();
});

it('blocks a stale preview after save failure and supports retry', async () => {
  mock.save.mockResolvedValueOnce(false);
  await show();
  expect(mock.load).not.toHaveBeenCalled();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('저장하지 못했습니다');
  await click('다시 시도');
  expect(mock.load).toHaveBeenCalledWith('/book', 'second');
});

it('turns paired pages, keeps the final odd page accessible, and crosses chapter boundaries', async () => {
  await show();
  expect(status()).toBe('1–2 / 5쪽');
  await click('다음'); expect(status()).toBe('3–4 / 5쪽');
  await click('다음'); expect(status()).toBe('5 / 5쪽');
  expect(button('다음').disabled).toBe(true);
  await click('이전'); await click('이전'); await click('이전');
  expect(mock.load).toHaveBeenLastCalledWith('/book', 'first');
  expect(status()).toBe('5 / 5쪽');
  await click('다음');
  expect(mock.load).toHaveBeenLastCalledWith('/book', 'second');
  expect(status()).toBe('1–2 / 5쪽');
});

it('responds to a narrow screen and clamps the page after repagination', async () => {
  await show(); await click('다음');
  width = 350;
  await act(async () => resize());
  expect(document.querySelector('.reader-paper')?.getAttribute('data-columns')).toBe('1');
  expect(status()).toBe('3 / 5쪽');
  pages = 1;
  await act(async () => resize());
  expect(status()).toBe('1 / 1쪽');
});

it('supports keyboard page turns without stealing chapter-select or composing keys', async () => {
  await show();
  await act(async () => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' })));
  expect(status()).toBe('3–4 / 5쪽');
  await act(async () => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', isComposing: true })));
  expect(status()).toBe('3–4 / 5쪽');
  await act(async () => document.querySelector('select')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })));
  expect(status()).toBe('3–4 / 5쪽');
});

it('ignores late chapter results and allows recovery from a read failure', async () => {
  let finish!: (value: DocData) => void;
  mock.load.mockReturnValueOnce(new Promise<DocData>((resolve) => { finish = resolve; }));
  await show(); await chapter('first');
  await act(async () => finish(data('second', '늦은 원고')));
  expect(document.querySelector('.reader-pages h3')?.textContent).toBe('1화 · 첫 장');
  expect(document.body.textContent).not.toContain('늦은 원고');
  mock.load.mockRejectedValueOnce('파일을 읽지 못함');
  await chapter('second');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('파일을 읽지 못함');
  await click('다시 시도');
  expect(document.querySelector('.reader-pages h3')?.textContent).toBe('2화 · 둘째 장');
});

it('stores reading appearance locally and leaves the original document intact', async () => {
  const source = data('second'); const original = JSON.stringify(source);
  mock.load.mockResolvedValue(source);
  await show(); await click('휴대폰'); await click('글자 크게');
  await act(async () => {
    const select = document.querySelector<HTMLSelectElement>('[aria-label="읽기 배경"]')!;
    select.value = 'dark'; select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  expect(document.querySelector('.reader-book')?.getAttribute('data-reader-theme')).toBe('dark');
  expect(JSON.parse(localStorage.getItem('wp.bookReader')!)).toEqual({ device: 'phone', theme: 'dark', fontSize: 20 });
  expect(JSON.stringify(source)).toBe(original);
  await click('편집으로 돌아가기'); expect(mock.close).toHaveBeenCalledOnce();
});

it('handles projects with no manuscript instead of reading private planning', async () => {
  mock.state.overview!.parts = [];
  await show();
  expect(document.body.textContent).toContain('읽을 원고가 없습니다');
  expect(mock.load).not.toHaveBeenCalled();
});
