// New versions of the app: a quiet check at every start, a toast when one is
// out, and the install after everything is saved (the installer closes the app
// and opens the new version). 보기 설정 shows the version and checks on demand.

import { api } from '../api';
import type { UpdateInfo } from '../api/types';
import { set } from './state';
import { closeDialog, openDialog, saveEverything, showToast, toastError } from './ui';
import { finishProjectFile } from './wpm';

const AUTO_KEY = 'wp.update.auto';
let installing = false;

/** Whether the app looks for new versions by itself (at every start). */
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

/** Looks for a new version when the app starts, saying nothing unless one is out. */
export async function checkForUpdateOnStart() {
  if (!api.isDesktop || !autoUpdateCheck()) return;
  try {
    const found = await api.updateCheck();
    if (found) offerUpdate(found);
  } catch {
    // Offline or GitHub unreachable: the next start asks again.
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
    message: `지금 쓰는 ${found.current}을 ${found.version}으로 바꿉니다. 원고와 작품 파일을 모두 저장한 뒤 앱이 잠시 닫혔다가 새 버전으로 다시 열립니다.${notes}`,
    confirm: '저장하고 바꾸기',
    onConfirm: async () => {
      closeDialog();
      await installUpdate();
    },
  });
}

async function installUpdate() {
  if (installing) return;
  installing = true;
  let stop = () => {};
  // Prevent edits while the final saves and package are being captured.
  set({ busy: '업데이트 전에 원고를 저장하는 중' });
  try {
    if (!(await saveEverything(true))) return;
    await api.journalFlush(null).catch(() => {});
    if (!(await finishProjectFile())) {
      showToast({ text: '작품 파일을 저장하지 못해 업데이트를 중단했습니다. 저장 상태를 확인하세요.', tone: 'error' });
      return;
    }
    stop = api.onUpdateProgress((p) => {
      const share = p.total ? ` ${Math.min(99, Math.round((p.received / p.total) * 100))}%` : '';
      set({ busy: `새 버전을 받는 중${share}` });
    });
    set({ busy: '새 버전을 받는 중' });
    await api.updateInstall();
  } catch (e) {
    toastError('새 버전으로 바꾸지 못함', e);
  } finally {
    stop();
    installing = false;
    set({ busy: null });
  }
}
