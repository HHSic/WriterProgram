// Rescue copies (비상 보관) found when a project opens: a chapter's text that
// could not be saved last time and is newer than the chapter on disk. The
// writer compares the two (workspace/Compare.tsx) and keeps one.

import { api } from '../api';
import type { Overview, RescueFile } from '../api/types';
import { peerOf, reloadDoc } from '../editor/shared';
import { UNTITLED } from '../lib/labels';
import { get, set } from './state';
import { findDoc, openDialog, patchSummary, saveEverything, showToast, toastError } from './ui';

/** Chapters' rescue copies newer than the chapter, newest first. */
export function newerRescues(ov: Overview, files: RescueFile[]): RescueFile[] {
  return files.filter((f) => {
    const place = findDoc(ov, f.item);
    if (!place) return false;
    const saved = Date.parse(f.saved);
    const modified = place.doc.modified ? Date.parse(place.doc.modified) : 0;
    return Number.isFinite(saved) && saved > modified;
  });
}

/** Offers to compare when the open project has a rescue copy newer than its chapter. */
export async function offerRescues() {
  const ov = get().overview;
  if (!ov) return;
  let files: RescueFile[];
  try {
    files = newerRescues(ov, await api.rescueList(ov.project.id));
  } catch {
    // Nothing to offer when the app's folder cannot be read.
    return;
  }
  if (files.length === 0) return;
  const first = files[0];
  showToast({
    text: files.length > 1 ? `비상 보관된 글이 ${files.length}개 있습니다` : '비상 보관된 글이 있습니다',
    action: { label: '비교하기', run: () => openDialog({ kind: 'compare', docId: first.item, rescue: first }) },
  });
}

/** 지금 글 두기: takes the rescue copy off the list (the file is kept in `old`). */
export async function setRescueAside(file: RescueFile) {
  try {
    await api.rescueSetAside(file.path);
  } catch (e) {
    toastError('비상 보관 글을 치우지 못함', e);
    return;
  }
  void offerRescues();
}

/** 비상 보관 글로 바꾸기: the chapter's text is kept as a record first. */
export async function takeRescue(file: RescueFile) {
  const ov = get().overview;
  if (!ov) return;
  const docId = file.item;
  if (!(await saveEverything())) return;
  try {
    const body = await api.rescueLoad(file.path);
    const current = peerOf(docId)?.getJSON() ?? (await api.docLoad(ov.root, docId)).body;
    await api.docKeep(ov.root, docId, current, 'before-restore');
    const outcome = await api.docSave(ov.root, docId, body, null, true);
    reloadDoc(docId, body, outcome.rev);
    patchSummary(docId, { counts: outcome.counts, pages: outcome.pages, modified: new Date().toISOString() });
    set({ recordsVersion: get().recordsVersion + 1 });
    await api.rescueSetAside(file.path);
    showToast({ text: `${findDoc(ov, docId)?.doc.title || UNTITLED}: 비상 보관된 글로 바꿈 · 바꾸기 전 글은 기록에 남음` });
  } catch (e) {
    toastError('비상 보관 글로 바꾸지 못함', e);
    return;
  }
  void offerRescues();
}
