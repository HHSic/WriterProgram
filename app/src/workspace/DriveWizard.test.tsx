// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { DriveInfo } from '../api/types';
import { DriveWizard } from './DriveWizard';
import { DrivesDialog } from './Drives';

const mock = vi.hoisted(() => ({
  api: {isDesktop: true, driveStatus: vi.fn(), driveRegistrationGet: vi.fn(), driveRegistrationSave: vi.fn(), driveRegistrationOpen: vi.fn(), driveConnect: vi.fn(), driveCancel: vi.fn()},
  close: vi.fn(), toast: vi.fn(),
}));
vi.mock('../api', () => ({api: mock.api}));
vi.mock('../store', () => ({closeDialog: mock.close, toastError: mock.toast, enterProject: vi.fn(), linkProject: vi.fn(), openDialog: vi.fn(), syncNow: vi.fn(), unlinkProject: vi.fn(), useApp: vi.fn()}));
vi.mock('../components/PlacePicker', () => ({placeNote: vi.fn(), usePlaceOf: vi.fn()}));
(globalThis as {IS_REACT_ACT_ENVIRONMENT?: boolean}).IS_REACT_ACT_ENVIRONMENT = true;

const drives: DriveInfo[] = [
  {provider: 'google', label: 'Google Drive', registered: false, account: null, connectedAt: null},
  {provider: 'onedrive', label: 'OneDrive', registered: true, account: null, connectedAt: null},
  {provider: 'dropbox', label: 'Dropbox', registered: false, account: null, connectedAt: null},
];
let root: Root;
beforeEach(() => {
  vi.clearAllMocks();
  mock.api.driveStatus.mockResolvedValue({drives, appsFile: 'private-path/drive-apps.json'});
  mock.api.driveRegistrationGet.mockResolvedValue(null);
  mock.api.driveRegistrationSave.mockResolvedValue(undefined);
  mock.api.driveRegistrationOpen.mockResolvedValue(undefined);
  mock.api.driveConnect.mockResolvedValue({name: '작가', email: 'writer@example.com'});
  mock.api.driveCancel.mockResolvedValue(undefined);
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => {act(() => root.unmount()); document.body.replaceChildren();});
async function show(initial: DriveInfo | null = null) {
  await act(async () => root.render(<DriveWizard drives={drives} initial={initial} onClose={mock.close} />));
}
function button(text: string) {
  const found = [...document.querySelectorAll('button')].find((node) => node.textContent === text);
  if (!found) throw new Error(`Missing button ${text}`);
  return found;
}
async function click(text: string) {await act(async () => button(text).click());}
async function input(label: string, value: string) {
  const field = [...document.querySelectorAll('label')].find((node) => node.querySelector('.field-label')?.textContent === label)?.querySelector('input');
  if (!field) throw new Error(`Missing input ${label}`);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(field, value);
    field.dispatchEvent(new Event('input', {bubbles: true}));
  });
}

it('offers setup for unregistered drives instead of asking for a JSON file', async () => {
  await act(async () => root.render(<DrivesDialog />));
  expect(document.body.textContent).not.toContain('private-path');
  expect(button('연결 설정').disabled).toBe(false);
  await click('연결 설정');
  expect(document.querySelector('[role="dialog"]')?.getAttribute('aria-label')).toBe('드라이브 연결 마법사');
  expect(document.body.textContent).toContain('데스크톱 앱');
});

it('saves registration before allowing login and confirms the actual account', async () => {
  let finish!: () => void;
  mock.api.driveRegistrationSave.mockReturnValue(new Promise<void>((resolve) => {finish = resolve;}));
  await show(drives[2]);
  expect(document.body.textContent).toContain('sharing.write');
  expect(document.querySelector<HTMLInputElement>('input[readonly]')?.value).toBe('http://localhost:53682/');
  await click('등록 페이지 열기');
  expect(mock.api.driveRegistrationOpen).toHaveBeenCalledExactlyOnceWith('dropbox');
  await click('등록 정보 입력');
  expect(button('저장하고 다음').disabled).toBe(true);
  await input('App key', ' dropbox-key ');
  await click('저장하고 다음');
  expect(mock.api.driveRegistrationSave).toHaveBeenCalledExactlyOnceWith('dropbox', {clientId: 'dropbox-key', clientSecret: null});
  expect(mock.api.driveConnect).not.toHaveBeenCalled();
  expect(document.body.textContent).not.toContain('브라우저에서 로그인');
  await act(async () => finish());
  await click('브라우저에서 로그인');
  expect(mock.api.driveConnect).toHaveBeenCalledExactlyOnceWith('dropbox');
  expect(document.body.textContent).toContain('Dropbox 연결 완료');
  expect(document.body.textContent).toContain('writer@example.com');
  await click('완료');
  expect(mock.close).toHaveBeenCalledOnce();
});

it('keeps entries and blocks login when saving fails, then allows a retry', async () => {
  mock.api.driveRegistrationSave.mockRejectedValueOnce('파일 접근 권한 없음');
  await show(drives[2]);
  await click('등록 정보 입력');
  await input('App key', 'my-key');
  await click('저장하고 다음');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('파일 접근 권한 없음');
  expect(document.querySelector<HTMLInputElement>('input')?.value).toBe('my-key');
  expect(mock.api.driveConnect).not.toHaveBeenCalled();
  await click('저장하고 다음');
  expect(button('브라우저에서 로그인')).toBeDefined();
});

it('requires the Google app secret and keeps drafts when returning to the guide', async () => {
  await show(drives[0]);
  await click('등록 정보 입력');
  await input('클라이언트 ID', 'google.apps.googleusercontent.com');
  expect(button('저장하고 다음').disabled).toBe(true);
  await input('클라이언트 보안 비밀', 'google-secret');
  expect(document.querySelector('input[type="password"]')).not.toBeNull();
  await click('이전');
  await click('등록 정보 입력');
  expect(mock.api.driveRegistrationGet).toHaveBeenCalledTimes(1);
  await click('저장하고 다음');
  expect(mock.api.driveRegistrationSave).toHaveBeenCalledWith('google', {clientId: 'google.apps.googleusercontent.com', clientSecret: 'google-secret'});
});

it('skips registration for existing apps and keeps login errors available for retry', async () => {
  mock.api.driveConnect.mockRejectedValueOnce('조직 관리자 동의 필요');
  await show(drives[1]);
  expect(mock.api.driveRegistrationGet).not.toHaveBeenCalled();
  await click('브라우저에서 로그인');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('조직 관리자 동의 필요');
  expect(document.body.textContent).not.toContain('연결 완료');
  await click('브라우저에서 로그인');
  expect(document.body.textContent).toContain('OneDrive 연결 완료');
  expect(mock.api.driveRegistrationSave).not.toHaveBeenCalled();
});

it('loads existing settings for editing and preserves a custom redirect port', async () => {
  mock.api.driveRegistrationGet.mockResolvedValue({clientId: 'old-key', redirectPort: 45678});
  await show({...drives[2], registered: true});
  await click('등록 정보 수정');
  await input('App key', 'new-key');
  await click('이전');
  expect(document.querySelector<HTMLInputElement>('input[readonly]')?.value).toBe('http://localhost:45678/');
  await click('등록 정보 입력');
  await click('저장하고 다음');
  expect(mock.api.driveRegistrationSave).toHaveBeenCalledWith('dropbox', {clientId: 'new-key', clientSecret: null, redirectPort: 45678});
});

it('cancels a pending browser login when the wizard is dismissed', async () => {
  let reject!: (error: string) => void;
  mock.api.driveConnect.mockReturnValue(new Promise((_resolve, fail) => {reject = fail;}));
  await show(drives[1]);
  await click('브라우저에서 로그인');
  await click('로그인 취소');
  expect(mock.api.driveCancel).toHaveBeenCalledOnce();
  await act(async () => root.render(<div />));
  expect(mock.api.driveCancel).toHaveBeenCalledTimes(2);
  await act(async () => reject('연결을 그만둠'));
  expect(document.body.textContent).not.toContain('연결 완료');
});

it('shows a read error instead of replacing existing settings with a blank form', async () => {
  mock.api.driveRegistrationGet.mockRejectedValue('설정을 읽지 못함');
  await show(drives[2]);
  await click('등록 정보 입력');
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('설정을 읽지 못함');
  expect(document.querySelector('input:not([readonly])')).toBeNull();
  expect(mock.api.driveRegistrationSave).not.toHaveBeenCalled();
});
