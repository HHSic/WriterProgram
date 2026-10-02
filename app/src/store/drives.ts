// Keeping a project in step with a drive.

import { api } from '../api';
import type { DriveProvider, Overview } from '../api/types';
import { localDate, sizeText } from '../lib/format';
import { get, set } from './state';
import { saveEverything, showToast, toastError } from './ui';
import { refreshOverview } from './project';
import { refreshCardCounts } from './cards';
import { loadNotes } from './notes';

// ---------------------------------------------------------------------------
// Keeping a project in step with a drive (crates/sync, app drives.rs)
//
// A linked project is brought in step when it opens, every minute while it
// is open, and when the window comes back to the front. Changes that arrive
// reach open documents through the folder watcher, like a sync program's.

const SYNC_EVERY_MS = 60_000;
let syncTimer: ReturnType<typeof setInterval> | undefined;
const onFocus = () => void syncNow({ quiet: true });

export async function loadLink(ov: Overview) {
  const link = await api.projectLinkGet(ov.project.id).catch(() => null);
  if (get().overview?.root !== ov.root) return;
  set({ link });
  if (link) startAutoSync();
}

function startAutoSync() {
  stopAutoSync();
  void syncNow({ quiet: true });
  syncTimer = setInterval(() => void syncNow({ quiet: true }), SYNC_EVERY_MS);
  window.addEventListener('focus', onFocus);
}

export function stopAutoSync() {
  clearInterval(syncTimer);
  syncTimer = undefined;
  window.removeEventListener('focus', onFocus);
}

const SPACE_WARNED_KEY = 'writer.driveSpaceWarned';
let spaceWarnedOn = '';

/**
 * The words for a drive running low on room, once a day (also across
 * restarts when the browser storage works); null when already said today.
 */
function lowSpaceWarning(left: number): string | null {
  const today = localDate();
  if (spaceWarnedOn === today) return null;
  try {
    if (localStorage.getItem(SPACE_WARNED_KEY) === today) {
      spaceWarnedOn = today;
      return null;
    }
    localStorage.setItem(SPACE_WARNED_KEY, today);
  } catch {
    // Without storage, once a day while the app stays open.
  }
  spaceWarnedOn = today;
  return `드라이브 남은 공간이 ${sizeText(left)}뿐입니다. 공간이 차면 맞추기가 멈춥니다.`;
}

/** One pass with the drive now. Quiet passes only speak up about problems in the status bar. */
export async function syncNow(opts: { quiet?: boolean } = {}) {
  const ov = get().overview;
  if (!ov || !get().link || get().syncing) return;
  // Edits waiting to be saved go first, so they travel in this pass.
  if (!(await saveEverything())) return;
  set({ syncing: true });
  try {
    const out = await api.projectSync(ov.root, ov.project.id);
    if (get().overview?.root !== ov.root) return;
    set({ link: out.link });
    const r = out.report;
    if (r && (r.downloaded.length || r.removedHere.length || r.copies.length || r.merged)) {
      await refreshOverview();
      void loadNotes();
      void refreshCardCounts();
    }
    const lowSpace = r?.spaceLeft != null ? lowSpaceWarning(r.spaceLeft) : null;
    if (!opts.quiet && out.link.error) {
      showToast({ text: `드라이브와 맞추지 못함 · ${out.link.error}`, tone: 'error' });
    } else if (lowSpace) {
      showToast({ text: lowSpace });
    } else if (!opts.quiet) {
      showToast({ text: '드라이브와 맞춤' });
    }
  } catch (e) {
    if (!opts.quiet) toastError('드라이브와 맞추지 못함', e);
  } finally {
    set({ syncing: false });
  }
}

/** Starts keeping the open project in step with a drive. */
export async function linkProject(provider: DriveProvider): Promise<boolean> {
  const ov = get().overview;
  if (!ov) return false;
  try {
    const link = await api.projectLink(ov.project.id, ov.project.title, provider);
    set({ link });
    startAutoSync();
    return true;
  } catch (e) {
    toastError('드라이브와 맞추기를 시작하지 못함', e);
    return false;
  }
}

/** Stops keeping the open project in step (nothing is removed anywhere). */
export async function unlinkProject() {
  const ov = get().overview;
  if (!ov) return;
  try {
    await api.projectUnlink(ov.project.id);
    stopAutoSync();
    set({ link: null });
  } catch (e) {
    toastError('그만두지 못함', e);
  }
}
