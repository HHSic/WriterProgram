// Creation journal (창작 일지): this device's on/off switch, the first-use
// notice (docs/creation-proof.md §7), and the time stamps (시각 고정, 날짜 증명,
// §4.2). The first time, a small note in the corner asks whether stamps may
// be taken; then they come by themselves (or each one is asked, 물어보고 받기)
// when the writing calls for one: a regular look every ten minutes (the
// first change of a day, or 3,000 characters since the last stamp; the rule
// is writer_core::anchor), and moments of their own — a chapter marked
// finished or published, a manuscript sent or exported, the project or app
// closing. At most four a day besides the ones the writer asks for. The
// editor's part is editor/journal.ts.

import { api } from '../api';
import type { AnchorOccasion, AnchorResult, DocStatus, JournalSettings } from '../api/types';
import { setJournalOn as setEditorJournal } from '../editor/journal';
import { get, set } from './state';
import { openDialog, showToast, toastError } from './ui';

/** While a project is open, how often to see whether a stamp is due. */
const CHECK_EVERY = 10 * 60 * 1000;

/** With 물어보고 받기, the shortest time between two questions. */
const ASK_AGAIN_AFTER = 30 * 60 * 1000;

/** Statuses that mark a chapter as done: a moment worth its own stamp. */
const FINISHING: DocStatus[] = ['done', 'published'];

let checkTimer: ReturnType<typeof setInterval> | undefined;
let lastAsked = 0;
let consentAsked = false;

/**
 * Reads this device's setting when a project opens. The first time, tells
 * the writer the journal is kept, with a way to turn it off. `quiet` holds
 * the notice back (another toast is showing); it comes next time. After the
 * notice, the corner asks once whether date proofs may be taken.
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
  if (root) startChecking(root);
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
  if (settings.anchor === null) askConsent();
}

/** Asks once, in the corner, whether a fingerprint may go out for date proofs. */
function askConsent() {
  if (consentAsked) return;
  consentAsked = true;
  set({
    cornerNote: {
      text: '쓴 날짜를 증명받을까요? 원고 내용은 보내지 않고 지문(32바이트)만 공개 시각 인증 기관에 보냅니다.',
      stay: true,
      actions: [
        { label: '받기', run: () => void setAnchorAllowed(true) },
        { label: '안 받기', run: () => void setAnchorAllowed(false) },
        // 나중에: asked again the next time a project opens.
        { label: '나중에', run: () => (consentAsked = false) },
      ],
      onClose: () => (consentAsked = false),
    },
  });
}

/** Allows or stops time stamps on this device. */
export async function setAnchorAllowed(allowed: boolean): Promise<JournalSettings | null> {
  try {
    const settings = await api.journalSet({ anchor: allowed });
    const root = get().overview?.root;
    if (allowed && root) void stampIfDue(root, 'check');
    return settings;
  } catch (e) {
    toastError('날짜 증명 설정을 바꾸지 못함', e);
    return null;
  }
}

/** 자동 / 물어보고 받기 / 받지 않기, and whether to say when one came. */
export async function setAnchorWay(patch: { anchor?: boolean; anchorAsk?: boolean; anchorNotify?: boolean }) {
  try {
    return await api.journalSet(patch);
  } catch (e) {
    toastError('날짜 증명 설정을 바꾸지 못함', e);
    return null;
  }
}

/** Asks the app for a stamp; never in the writer's way (failures wait for the next try). */
export async function anchorNow(root: string, occasion: AnchorOccasion): Promise<AnchorResult | null> {
  try {
    return await api.journalAnchor(root, occasion);
  } catch {
    return null;
  }
}

/**
 * Takes a stamp if one is due for `occasion`, then says so in the corner
 * (when the writer wants that). With 물어보고 받기 the corner asks first.
 */
export async function stampIfDue(root: string, occasion: AnchorOccasion): Promise<AnchorResult | null> {
  const result = await anchorNow(root, occasion);
  if (!result) return null;
  if (result.state === 'ask') askToStamp(root);
  else if (result.state === 'signed' && occasion !== 'closing') void tellStamped();
  return result;
}

/** 물어보고 받기: a stamp is due; the corner asks, not more often than every half hour. */
function askToStamp(root: string) {
  const now = Date.now();
  if (now - lastAsked < ASK_AGAIN_AFTER || get().cornerNote) return;
  lastAsked = now;
  set({
    cornerNote: {
      text: '지금까지 쓴 글의 날짜 증명을 받을까요?',
      stay: true,
      actions: [
        {
          label: '받기',
          run: () => {
            void (async () => {
              const result = await anchorNow(root, 'now');
              if (result?.state === 'signed') void tellStamped();
            })();
          },
        },
        { label: '나중에', run: () => {} },
      ],
    },
  });
}

/** A quiet word in the corner that a date proof came, unless turned off. */
async function tellStamped() {
  try {
    const settings = await api.journalSettings();
    if (!settings.anchorNotify) return;
  } catch {
    return;
  }
  if (get().cornerNote) return;
  set({ cornerNote: { text: '오늘 쓴 글의 날짜 증명을 받았습니다.', ms: 2500 } });
}

/** A chapter's status changed: finishing it is a moment for a stamp. */
export function noteStatusChange(status: DocStatus) {
  const root = get().overview?.root;
  if (root && FINISHING.includes(status)) void stampIfDue(root, 'moment');
}

/** A manuscript went out (sent to an editor or exported): a moment for a stamp. */
export function noteManuscriptOut() {
  const root = get().overview?.root;
  if (root) void stampIfDue(root, 'moment');
}

/**
 * The project or app is closing: takes a stamp if one is due, giving the
 * authorities a few seconds at most. "날짜 증명을 받는 중" shows only when it
 * takes a moment, so nothing flickers when none is due.
 */
export async function stampOnClosing(root: string) {
  const busy = setTimeout(() => set({ busy: '날짜 증명을 받는 중' }), 400);
  try {
    await anchorNow(root, 'closing');
  } finally {
    clearTimeout(busy);
    if (get().busy === '날짜 증명을 받는 중') set({ busy: null });
  }
}

/** Looks now and every ten minutes while `root` stays open. */
function startChecking(root: string) {
  if (checkTimer) clearInterval(checkTimer);
  void stampIfDue(root, 'check');
  checkTimer = setInterval(() => {
    if (get().overview?.root !== root) {
      clearInterval(checkTimer);
      checkTimer = undefined;
      return;
    }
    void stampIfDue(root, 'check');
  }, CHECK_EVERY);
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
