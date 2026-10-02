// Creation journal (창작 일지): this device's on/off switch, the first-use
// notice (docs/creation-proof.md §7), and the daily time stamp (시각 고정,
// §4.2), asked for once before the first one. The editor's part is
// editor/journal.ts.

import { api } from '../api';
import type { AnchorResult, JournalSettings } from '../api/types';
import { setJournalOn as setEditorJournal } from '../editor/journal';
import { get } from './state';
import { openDialog, showToast, toastError } from './ui';

/** While a project is open, how often to see whether a stamp is due. */
const ANCHOR_EVERY = 60 * 60 * 1000;

let anchorTimer: ReturnType<typeof setInterval> | undefined;

/**
 * Reads this device's setting when a project opens. The first time, tells
 * the writer the journal is kept, with a way to turn it off. `quiet` holds
 * the notice back (another toast is showing); it comes next time. Then
 * stamps the project once a day while it is open, when allowed; the first
 * time after the notice it asks.
 */
export async function loadJournal(quiet: boolean) {
  let settings: JournalSettings;
  try {
    settings = await api.journalSettings();
  } catch {
    return;
  }
  setEditorJournal(settings.enabled);
  const root = get().overview?.root;
  if (root) startAnchoring(root);
  if (!settings.enabled || quiet) return;
  if (!settings.noticed) {
    showToast({
      text: '쓰는 과정을 이 기기에 기록합니다(내용은 기록하지 않음)',
      action: { label: '끄기…', run: () => openDialog({ kind: 'project', tab: 'basic' }) },
    });
    try {
      await api.journalSet({ noticed: true });
    } catch {
      // Shown again next time; no harm.
    }
    return;
  }
  if (settings.anchor === null && !get().dialog) askAnchor();
}

/** Asks once whether a fingerprint may go out for the daily time stamp. */
function askAnchor() {
  openDialog({
    kind: 'confirm',
    title: '날짜 증명 받기',
    message:
      '하루 한 번, 그날까지 쓴 기록의 지문(32바이트)만 공개 시각 인증 기관에 보내, 그날 이 원고가 있었다는 증명을 받을까요? 원고 내용은 보내지 않고, 지문으로 원고를 되살릴 수 없습니다. 나중에 작품 설정에서 바꿀 수 있습니다.',
    confirm: '증명 받기',
    cancel: '받지 않기',
    onConfirm: async () => {
      await setAnchorAllowed(true);
    },
    onCancel: () => void setAnchorAllowed(false),
  });
}

/** Allows or stops daily time stamps on this device. */
export async function setAnchorAllowed(allowed: boolean): Promise<JournalSettings | null> {
  try {
    const settings = await api.journalSet({ anchor: allowed });
    const root = get().overview?.root;
    if (allowed && root) void anchorNow(root, false);
    return settings;
  } catch (e) {
    toastError('날짜 증명 설정을 바꾸지 못함', e);
    return null;
  }
}

/** Stamps `root` if due (or now, `force`). Never in the writer's way: failures wait for the next try. */
export async function anchorNow(root: string, force: boolean): Promise<AnchorResult | null> {
  try {
    return await api.journalAnchor(root, force);
  } catch {
    return null;
  }
}

/** Tries now and every hour while `root` stays open. */
function startAnchoring(root: string) {
  if (anchorTimer) clearInterval(anchorTimer);
  void anchorNow(root, false);
  anchorTimer = setInterval(() => {
    if (get().overview?.root !== root) {
      clearInterval(anchorTimer);
      anchorTimer = undefined;
      return;
    }
    void anchorNow(root, false);
  }, ANCHOR_EVERY);
}

/** Turns the journal on or off on this device (all its projects). */
export async function setJournalEnabled(enabled: boolean): Promise<JournalSettings | null> {
  try {
    const settings = await api.journalSet({ enabled, noticed: true });
    setEditorJournal(settings.enabled);
    return settings;
  } catch (e) {
    toastError('창작 일지 설정을 바꾸지 못함', e);
    return null;
  }
}
