// New versions of the app: a quiet check once a day, a toast when one is out,
// and the install after everything is saved (the installer closes the app and
// opens the new version). 보기 설정 shows the version and checks on demand.

import { api } from '../api';
import type { UpdateInfo } from '../api/types';
import { set } from './state';
import { closeDialog, openDialog, saveEverything, showToast, toastError } from './ui';

const CHECKED_KEY = 'wp.update.checked';
const AUTO_KEY = 'wp.update.auto';

/** Whether the app looks for new versions by itself (once a day). */
export function autoUpdateCheck(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setAutoUpdateCheck(on: boolean) {
  try {
    localStorage.setItem(AUTO_KEY, on ? 'on' : 'off');
  } catch {
    // Stays on for this session; no harm.
  }
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Looks for a new version once a day, saying nothing unless one is out. */
export async function checkForUpdateDaily() {
  if (!api.isDesktop || !autoUpdateCheck()) return;
  try {
    if (localStorage.getItem(CHECKED_KEY) === today()) return;
    localStorage.setItem(CHECKED_KEY, today());
  } catch {
    // Without storage, check every start.
  }
  try {
    const found = await api.updateCheck();
    if (found) offerUpdate(found);
  } catch {
    // Offline or GitHub unreachable: try again tomorrow, quietly.
  }
}

/** Looks now (보기 설정): says what it found either way. */
export async function checkForUpdateNow(): Promise<UpdateInfo | null> {
  try {
    const found = await api.updateCheck();
    if (found) offerUpdate(found);
    else showToast({ text: '지금 쓰는 버전이 가장 새 버전입니다.' });
    return found;
  } catch (e) {
    toastError('새 버전을 확인하지 못함', e);
    return null;
  }
}

function offerUpdate(found: UpdateInfo) {
  showToast({
    text: `새 버전 ${found.version}이 나왔습니다.`,
    action: { label: '바꾸기…', run: () => confirmUpdate(found) },
  });
}

function confirmUpdate(found: UpdateInfo) {
  const notes = found.notes ? `\n\n바뀐 것:\n${found.notes}` : '';
  openDialog({
    kind: 'confirm',
    title: `새 버전 ${found.version}`,
    message: `지금 쓰는 ${found.current}을 ${found.version}으로 바꿉니다. 원고를 모두 저장한 뒤 앱이 잠시 닫혔다가 새 버전으로 다시 열립니다.${notes}`,
    confirm: '저장하고 바꾸기',
    onConfirm: async () => {
      closeDialog();
      await installUpdate();
    },
  });
}

async function installUpdate() {
  if (!(await saveEverything())) return;
  const stop = api.onUpdateProgress((p) => {
    const share = p.total ? ` ${Math.min(99, Math.round((p.received / p.total) * 100))}%` : '';
    set({ busy: `새 버전을 받는 중${share}` });
  });
  set({ busy: '새 버전을 받는 중' });
  try {
    await api.updateInstall();
    // Only reached where the installer does not close the app, or in a dry run.
    set({ busy: null });
  } catch (e) {
    set({ busy: null });
    toastError('새 버전으로 바꾸지 못함', e);
  } finally {
    stop();
  }
}
