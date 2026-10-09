// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import type { Overview, ShareLink } from '../api/types';
import { ShareDialog } from './ShareDialog';

const mock = vi.hoisted(() => ({
  state: {overview: null as Overview | null, activeDocId: 'second'},
  api: {isDesktop: true, driveStatus: vi.fn(), shareList: vi.fn(), shareCreate: vi.fn(), shareRevoke: vi.fn()},
  save: vi.fn(), close: vi.fn(), open: vi.fn(), copy: vi.fn(), dialog: vi.fn(),
}));
vi.mock('../api', () => ({api: mock.api}));
vi.mock('../store', () => ({useApp: (select: (state: typeof mock.state) => unknown) => select(mock.state),
  allManuscript: (ov: Overview) => ov.parts.flatMap((p) => p.docs), closeDialog: mock.close, openDialog: mock.dialog, openWeb: mock.open, saveEverything: mock.save}));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root;
const link: ShareLink = {id: 'a'.repeat(32), url: 'https://drive.google.com/file/d/test-document/view',
  title: '공유 작품', chapterCount: 1, createdAt: '2026-10-06T00:00:00Z', provider: 'google', account: 'writer@example.com'};
beforeEach(() => {
  vi.clearAllMocks();
  mock.state.overview = {root: '/work/book', project: {title: '공유 작품'},
    parts: [{docs: [{id: 'first', title: '첫 장'}, {id: 'second', title: '둘째 장'}]}],
    planning: [{id: 'private', title: '비밀 기획'}]} as unknown as Overview;
  mock.api.driveStatus.mockResolvedValue({drives: [{provider: 'google', label: 'Google Drive', registered: true, account: {email: 'writer@example.com', name: '작가'}}]});
  mock.api.shareList.mockResolvedValue([]);
  mock.api.shareCreate.mockResolvedValue(link);
  mock.api.shareRevoke.mockResolvedValue(undefined);
  mock.save.mockResolvedValue(true);
  mock.copy.mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {configurable: true, value: {writeText: mock.copy}});
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); document.body.replaceChildren(); });
async function show() { await act(async () => { root.render(<ShareDialog />); }); }
function button(label: string) {
  const button = [...document.querySelectorAll('button')].find((b) => b.textContent === label);
  if (!button) throw new Error(`Missing ${label}`);
  return button;
}
async function click(label: string) { await act(async () => { button(label).click(); }); }

it('waits for pending saves and publishes only the selected manuscript', async () => {
  let saved!: (ok: boolean) => void;
  mock.save.mockReturnValue(new Promise<boolean>((resolve) => { saved = resolve; }));
  await show();
  expect(document.body.textContent).not.toContain('비밀 기획');
  await click('링크 만들기');
  expect(mock.api.shareCreate).not.toHaveBeenCalled();
  await act(async () => { saved(true); });
  expect(mock.api.shareCreate).toHaveBeenCalledExactlyOnceWith('/work/book', ['second'], 'google');
  expect(document.querySelector<HTMLInputElement>('[aria-label="공유 링크 주소"]')?.value).toBe(link.url);
  await click('링크 복사');
  expect(mock.copy).toHaveBeenCalledExactlyOnceWith(link.url);
});

it('does not publish when saving the manuscript fails', async () => {
  mock.save.mockResolvedValue(false);
  await show();
  await click('링크 만들기');
  expect(mock.api.shareCreate).not.toHaveBeenCalled();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('저장하지 못했습니다');
});

it('does not claim a link was created when uploading fails', async () => {
  mock.api.shareCreate.mockRejectedValue(new Error('서버에 연결할 수 없음'));
  await show();
  await click('모두 선택');
  await click('링크 만들기');
  expect(mock.api.shareCreate).toHaveBeenCalledWith('/work/book', ['first', 'second'], 'google');
  expect(document.querySelector('[aria-label="새 공유 링크"]')).toBeNull();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('서버에 연결할 수 없음');
});

it('keeps a link listed when revocation fails and removes it on success', async () => {
  mock.api.shareList.mockResolvedValue([link]);
  mock.api.shareRevoke.mockRejectedValueOnce('서버에 연결할 수 없음');
  await show();
  await click('공유 종료');
  expect(button('공유 종료')).toBeDefined();
  await click('공유 종료');
  expect(mock.api.shareRevoke).toHaveBeenCalledWith('/work/book', link.id);
  expect(document.querySelector('.share-links')?.children.length).toBe(0);
});

it('requires a connected drive account', async () => {
  mock.api.driveStatus.mockResolvedValue({drives: []});
  await show();
  expect(button('링크 만들기').disabled).toBe(true);
  expect(document.body.textContent).toContain('드라이브 계정을 먼저 연결');
  await click('드라이브 연결…');
  expect(mock.dialog).toHaveBeenCalledWith({kind: 'drives'});
});

it('allows only connected registered accounts and uses the chosen provider', async () => {
  mock.api.driveStatus.mockResolvedValue({drives: [
    {provider: 'google', label: 'Google Drive', registered: true, account: null},
    {provider: 'onedrive', label: 'OneDrive', registered: true, account: {email:'writer@example.com'}},
    {provider: 'dropbox', label: 'Dropbox', registered: false, account: {email:'old@example.com'}},
  ]});
  await show();
  const options = [...document.querySelectorAll('option')];
  expect(options.map((o) => o.value)).toEqual(['onedrive']);
  await click('링크 만들기');
  expect(mock.api.shareCreate).toHaveBeenCalledWith('/work/book', ['second'], 'onedrive');
});

it('requires the original drive account to end a share', async () => {
  mock.api.shareList.mockResolvedValue([{...link, account: 'another@example.com'}]);
  await show();
  expect(button('공유 종료').disabled).toBe(true);
});
