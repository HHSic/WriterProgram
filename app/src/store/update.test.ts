// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { UpdateInfo } from '../api/types';

const mock = vi.hoisted(() => ({
  api: { isDesktop: true, updateCheck: vi.fn(), updateInstall: vi.fn(), onUpdateProgress: vi.fn(), journalFlush: vi.fn() },
  save: vi.fn(), finish: vi.fn(), set: vi.fn(), toast: vi.fn(), error: vi.fn(), open: vi.fn(), close: vi.fn(), stop: vi.fn(),
}));
vi.mock('../api', () => ({ api: mock.api }));
vi.mock('./state', () => ({ set: mock.set }));
vi.mock('./ui', () => ({ saveEverything: mock.save, showToast: mock.toast, toastError: mock.error, openDialog: mock.open, closeDialog: mock.close }));
vi.mock('./wpm', () => ({ finishProjectFile: mock.finish }));

import { checkForUpdateNow, checkForUpdateOnStart, setAutoUpdateCheck } from './update';
const found: UpdateInfo = { version: '0.1.2', current: '0.1.1', notes: '작품 파일과 책 미리보기', date: null };
beforeEach(() => {
  vi.clearAllMocks(); localStorage.clear();
  mock.api.isDesktop = true;
  mock.api.updateCheck.mockResolvedValue(found);
  mock.api.updateInstall.mockResolvedValue(undefined);
  mock.api.journalFlush.mockResolvedValue(undefined);
  mock.api.onUpdateProgress.mockReturnValue(mock.stop);
  mock.save.mockResolvedValue(true); mock.finish.mockResolvedValue(true);
});
afterEach(() => { localStorage.clear(); });
async function accept() {
  await checkForUpdateNow();
  mock.toast.mock.calls.at(-1)![0].action.run();
  return mock.open.mock.calls.at(-1)![0].onConfirm as () => Promise<void>;
}

it('asks at every start, even after a check earlier the same day, and stays quiet offline', async () => {
  mock.api.updateCheck.mockRejectedValueOnce('offline');
  await checkForUpdateOnStart();
  expect(mock.toast).not.toHaveBeenCalled(); expect(mock.error).not.toHaveBeenCalled();
  await checkForUpdateOnStart(); await checkForUpdateOnStart();
  expect(mock.api.updateCheck).toHaveBeenCalledTimes(3);
  expect(mock.toast).toHaveBeenCalledTimes(2);
});

it('respects disabled automatic checks and browser-only previews', async () => {
  setAutoUpdateCheck(false); await checkForUpdateOnStart();
  setAutoUpdateCheck(true); mock.api.isDesktop = false; await checkForUpdateOnStart();
  expect(mock.api.updateCheck).not.toHaveBeenCalled();
});

it('saves manuscript, journal and the final wpm before starting installation', async () => {
  const order: string[] = [];
  let saved!: (ok: boolean) => void;
  mock.save.mockImplementation(() => new Promise<boolean>((resolve) => { saved = resolve; }));
  mock.api.journalFlush.mockImplementation(async () => { order.push('journal'); });
  mock.finish.mockImplementation(async () => { order.push('wpm'); return true; });
  mock.api.updateInstall.mockImplementation(async () => { order.push('install'); });
  const confirm = await accept(); const pending = confirm();
  expect(mock.save).toHaveBeenCalledExactlyOnceWith(true);
  expect(mock.set).toHaveBeenCalledWith({ busy: '업데이트 전에 원고를 저장하는 중' });
  expect(mock.api.updateInstall).not.toHaveBeenCalled();
  saved(true); await pending;
  expect(order).toEqual(['journal', 'wpm', 'install']);
  expect(mock.stop).toHaveBeenCalledOnce(); expect(mock.set).toHaveBeenLastCalledWith({ busy: null });
});

it('does not install after failed manuscript or project-file saving', async () => {
  mock.save.mockResolvedValueOnce(false);
  await (await accept())();
  expect(mock.api.updateInstall).not.toHaveBeenCalled();
  mock.finish.mockResolvedValueOnce(false);
  await (await accept())();
  expect(mock.api.updateInstall).not.toHaveBeenCalled();
  expect(mock.toast.mock.calls.at(-1)![0].text).toContain('작품 파일을 저장하지 못해');
  expect(mock.set).toHaveBeenLastCalledWith({ busy: null });
});

it('cleans up progress after installation failure and allows another confirmed attempt', async () => {
  mock.api.updateInstall.mockRejectedValueOnce('signature mismatch');
  const confirm = await accept(); await confirm();
  expect(mock.error).toHaveBeenCalledWith('새 버전으로 바꾸지 못함', 'signature mismatch');
  expect(mock.stop).toHaveBeenCalledOnce();
  await confirm(); expect(mock.api.updateInstall).toHaveBeenCalledTimes(2);
});

it('prevents duplicate install requests while saving', async () => {
  let saved!: (ok: boolean) => void;
  mock.save.mockReturnValueOnce(new Promise<boolean>(resolve => { saved = resolve; }));
  const confirm = await accept(); const pending = confirm(); await confirm();
  expect(mock.save).toHaveBeenCalledOnce(); saved(true); await pending;
  expect(mock.api.updateInstall).toHaveBeenCalledOnce();
});
