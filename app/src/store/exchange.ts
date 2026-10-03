// 교정본 주고받기 (docs/corrections.md): taking back a corrected file, the
// review tab, and applying what the writer picked there.

import { api } from '../api';
import type { Applied, Decisions } from '../api/types';
import { reloadDoc } from '../editor/shared';
import { docNoun } from '../lib/labels';
import { type Choices, NO_CHOICES, appliedText } from '../lib/review';
import { get, root, set } from './state';
import { closeDialog, patchSummary, saveEverything, showToast, toastError } from './ui';
import { openTarget } from './tabs';
import { selectDoc } from './docs';
import { loadNotes } from './notes';

/** Choices made in a review tab and not applied yet, by exchange (kept while the app is open). */
const drafts = new Map<string, Choices>();

export function reviewDraft(exchangeId: string): Choices {
  return drafts.get(exchangeId) ?? NO_CHOICES;
}

export function keepReviewDraft(exchangeId: string, choices: Choices) {
  drafts.set(exchangeId, choices);
}

function exchangesChanged() {
  set({ exchangesVersion: get().exchangesVersion + 1 });
}

/** Tells the list and review tabs that chapters were sent. */
export function noteSent() {
  exchangesChanged();
}

/** Opens the review of an exchange's corrected file in a middle tab. */
export async function openReview(exchangeId: string) {
  closeDialog();
  await openTarget({ kind: 'review', id: exchangeId });
}

/**
 * 교정본 가져오기: the writer picks the file the editor sent back, and it is
 * compared with what was sent. The review opens in a tab.
 */
export async function takeBackCorrected(exchangeId: string): Promise<boolean> {
  const picked = await api.pickFiles('받은 교정본 고르기', { name: '교정본 (hwpx, docx)', extensions: ['hwpx', 'docx'] });
  if (!picked.length) return false;
  if (!(await saveEverything())) return false;
  set({ busy: '교정본을 읽는 중' });
  try {
    await api.exchangeRead(root(), exchangeId, picked[0]);
    // A new file: what was picked for the last one no longer applies.
    drafts.delete(exchangeId);
    exchangesChanged();
  } catch (e) {
    toastError('교정본을 읽지 못함', e);
    return false;
  } finally {
    set({ busy: null });
  }
  await openReview(exchangeId);
  return true;
}

/**
 * 반영하기: applies the writer's choices. Chapters open in an editor take
 * the new text in place (cursor and undo of the rest stay); each changed
 * chapter was kept as a record ("교정 반영 전") first.
 */
export async function applyReview(exchangeId: string, decisions: Decisions): Promise<Applied | null> {
  if (!(await saveEverything())) return null;
  let applied: Applied;
  try {
    applied = await api.exchangeApply(root(), exchangeId, decisions);
  } catch (e) {
    toastError('반영하지 못함', e);
    return null;
  }
  drafts.delete(exchangeId);
  for (const docId of applied.docs) {
    try {
      const data = await api.docLoad(root(), docId);
      reloadDoc(docId, data.body, data.rev);
      patchSummary(docId, { counts: data.counts });
    } catch (e) {
      toastError('반영한 원고를 다시 불러오지 못함', e);
    }
  }
  if (applied.docs.length) set({ recordsVersion: get().recordsVersion + 1 });
  if (applied.memos.length) await loadNotes();
  exchangesChanged();
  const first = applied.docs[0];
  showToast({
    text: appliedText(applied, docNoun(get().overview!.project.kind)),
    action: first ? { label: '기록 보기', run: () => void showRecords(first) } : undefined,
  });
  return applied;
}

/** Opens a chapter with its records (기록) beside it, where "교정 반영 전" can be gone back to. */
async function showRecords(docId: string) {
  await selectDoc(docId);
  set({ rightOpen: true, rightTab: 'records' });
}
