// Package saving has its own retry/close protection, separate from text saves.
import { api } from '../api';
import { onProjectMutation } from '../api/projectChanges';
import type { Overview } from '../api/types';
import { flushEdits, registerAfterFlusher } from '../lib/flush';
import { debouncedSaver } from '../lib/useDebouncedSave';
import { registerSave } from './saves';
import { get, set } from './state';
import { saveEverything, showToast, toastError } from './ui';
import { openProject } from './project';

let current: { root: string; saver: ReturnType<typeof debouncedSaver<number>>; stop: () => void } | null = null;

export function stopProjectFile() {
  current?.stop();
  current = null;
}

export function startProjectFile(ov: Overview) {
  stopProjectFile();
  if (!ov.projectFile) return;
  const saver = debouncedSaver<number>({
    key: `wpm:${ov.root}`,
    save: async () => { await api.projectFileSave(ov.root); },
  });
  const unregister = registerSave(`wpm:${ov.root}`, { ...saver.target, rescueFile: () => api.projectFileRescue(ov.root) });
  const unflush = registerAfterFlusher(saver.flush);
  const unlisten = onProjectMutation((root) => {
    if (root === ov.root) saver.schedule(1, 1200);
  });
  current = { root: ov.root, saver, stop: () => { unlisten(); unflush(); unregister(); } };
}

/** Captures late journal/anchor writes before closing, after other saves. */
export async function finishProjectFile(): Promise<boolean> {
  const active = current;
  if (!active) return true;
  active.saver.schedule(1);
  await active.saver.flush();
  return !active.saver.target.pending();
}

export async function saveProjectFileAs() {
  const ov = get().overview;
  if (!ov) return;
  try {
    const dest = await api.pickSaveFile('작품 파일로 저장', ov.projectFile ?? `${ov.project.title}.wpm`, 'wpm');
    if (!dest || get().overview?.root !== ov.root) return;
    // Save As also offers a way out when the old file became unwritable.
    // Flush the editors; an old package failure must not block the new destination.
    await flushEdits();
    const otherFailed = Object.entries(get().saves).some(([key, value]) => !key.startsWith('wpm:') && value.state === 'error');
    if (otherFailed) {
      showToast({ text: '원고를 먼저 저장해야 작품 파일을 만들 수 있습니다.', tone: 'error' });
      return;
    }
    const file = await api.projectFileSave(ov.root, dest);
    const overview = get().overview;
    if (!overview || overview.root !== ov.root) return;
    const next = { ...overview, projectFile: file };
    set({ overview: next });
    if (!current) startProjectFile(next);
    // Any previous package failure now retries into the newly chosen file.
    if (!(await finishProjectFile())) return;
    showToast({ text: '작품 파일(.wpm)로 저장했습니다.', action: { label: '위치 열기', run: () => void api.reveal(file) } });
  } catch (e) {
    toastError('작품 파일을 저장하지 못함', e);
  }
}

export async function pickProjectFile() {
  try {
    const files = await api.pickFiles('작품 파일 열기', { name: 'WriterProgram 작품 파일 (.wpm)', extensions: ['wpm'] });
    const path = files[0];
    if (!path) return;
    if (get().overview && !(await saveEverything(true))) return;
    await openProject(path);
  } catch (e) {
    toastError('작품 파일을 열지 못함', e);
  }
}
