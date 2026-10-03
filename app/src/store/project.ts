// Opening, changing and leaving a project.

import { api } from '../api';
import type { ManuscriptFormat, Overview, ProjectPatch, RecoverWay } from '../api/types';
import { get, root, set } from './state';
import { closeDialog, openDialog, saveEverything, showToast, toastError } from './ui';
import { commit, initialLayout, resetTabs } from './tabs';
import { refreshCardCounts } from './cards';
import { loadNotes } from './notes';
import { startWatching, stopWatching } from './devices';
import { loadLink, stopAutoSync } from './drives';
import { loadJournal } from './journal';
import { loadAi } from './ai';
import { offerRescues } from './rescue';

// ---------------------------------------------------------------------------
// Projects

export function enterProject(ov: Overview) {
  stopAutoSync();
  resetTabs();
  set({
    overview: ov,
    previewCardId: null,
    cardCounts: {},
    notes: [],
    focusNoteId: null,
    pendingNote: null,
    selectedPartId: null,
    conflicts: {},
    cardReloads: {},
    link: null,
    heldRemovals: null,
    liveCounts: null,
    selection: null,
  });
  const layout = initialLayout(ov);
  commit(layout.panes, layout.focus, layout.split);
  api.setWindowTitle(`${ov.project.title} · WriterProgram`);
  startWatching(ov.root);
  void loadLink(ov);
  void loadCatalog();
  void refreshCardCounts();
  void loadNotes();
  void loadJournal(ov.copies.length > 0);
  void loadAi();
  // Writing kept in the rescue folder last time, newer than its chapter.
  void offerRescues();
  if (ov.copies.length) {
    showToast({
      text: `다른 기기에서 생긴 사본 ${ov.copies.length}개가 있음`,
      action: { label: '살펴보기', run: () => openDialog({ kind: 'copies' }) },
    });
  }
}

export async function loadCatalog() {
  try {
    set({ catalog: await api.formatCatalog() });
  } catch (e) {
    toastError('원고 서식 목록을 읽지 못함', e);
  }
}

/** Saves project settings and reloads the overview (page estimates depend on the format). */
export async function updateProject(patch: ProjectPatch): Promise<boolean> {
  try {
    const project = await api.projectUpdate(root(), patch);
    const ov = get().overview;
    if (ov) set({ overview: { ...ov, project } });
    await refreshOverview();
    api.setWindowTitle(`${project.title} · WriterProgram`);
    return true;
  } catch (e) {
    toastError('작품 설정을 저장하지 못함', e);
    return false;
  }
}

export async function applyFormat(format: ManuscriptFormat) {
  return updateProject({ manuscriptFormat: format });
}

export async function openProject(path: string): Promise<boolean> {
  try {
    enterProject(await api.projectOpen(path));
    return true;
  } catch (e) {
    // A damaged project.json gets the recovery screen instead of an error line.
    const recovery = await api.projectRecovery(path).catch(() => null);
    if (recovery) openDialog({ kind: 'recover', path, recovery });
    else toastError('작품을 열지 못함', e);
    return false;
  }
}

/** Brings a damaged project.json back the chosen way and opens the project. */
export async function recoverProject(path: string, way: RecoverWay): Promise<boolean> {
  try {
    const ov = await api.projectRecover(path, way);
    closeDialog();
    enterProject(ov);
    showToast({ text: '작품 구조를 되살렸습니다. 손상된 파일은 ‘project.json.damaged-…’ 이름으로 남겨 두었습니다.' });
    return true;
  } catch (e) {
    toastError('작품 구조를 되살리지 못함', e);
    return false;
  }
}

export async function leaveProject() {
  if (!(await saveEverything(true))) return;
  // The creation journal's saves still gathering go in as the project closes.
  const root = get().overview?.root;
  if (root) await api.journalFlush(root).catch(() => {});
  stopWatching();
  stopAutoSync();
  set({
    link: null,
    heldRemovals: null,
    overview: null,
    panes: [],
    focus: 0,
    activeTarget: null,
    activeDocId: null,
    activeCardId: null,
    editor: null,
    liveCounts: null,
    selection: null,
    dialog: null,
    previewCardId: null,
  });
  api.setWindowTitle('WriterProgram');
}

export async function refreshOverview() {
  try {
    set({ overview: await api.projectOverview(root()) });
  } catch (e) {
    toastError('작품 정보를 불러오지 못함', e);
  }
}
