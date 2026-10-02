// 소리 내어 읽기: one reading at a time, in the editor of the open tab
// (the reading itself is editor/readAloud.ts).

import { NodeSelection } from '@tiptap/pm/state';
import { Reader, loadVoices, pickVoice, readingPlan, speechAvailable } from '../editor/readAloud';
import { get, set, useApp } from './state';
import { showToast } from './ui';

let reader: Reader | null = null;
let unwatch: (() => void) | null = null;

/** Reads the open chapter aloud from the cursor, or the selected text. */
export async function startReading() {
  const editor = get().editor;
  if (!editor || editor.isDestroyed) return;
  stopReading(false);
  if (!speechAvailable()) {
    showToast({ text: '이 기기에서는 소리 내어 읽을 수 없음', tone: 'error' });
    return;
  }
  const voice = pickVoice(await loadVoices(), get().view.readVoice);
  // The writer may have moved on while the voices were looked up.
  if (get().editor !== editor || editor.isDestroyed) return;
  if (!voice) {
    set({ reading: { status: 'noVoice', editor } });
    return;
  }

  const { selection, doc } = editor.state;
  const ranged = !selection.empty && !(selection instanceof NodeSelection);
  let steps = readingPlan(doc, selection.from, ranged ? selection.to : selection.from);
  // From the very end: the whole chapter from the top.
  if (!steps.length && !ranged) steps = readingPlan(doc, 0, 0);
  if (!steps.length) {
    showToast({ text: '읽을 글이 없음' });
    return;
  }

  const r = new Reader(editor, steps, {
    voice,
    rate: () => get().view.readRate,
    onState: (status) => {
      if (reader !== r) return;
      if (status === 'done') finish();
      else set({ reading: { status, editor } });
    },
    onError: () => showToast({ text: '읽다가 멈춤 · 목소리를 낼 수 없음', tone: 'error' }),
  });
  reader = r;
  // Another tab (or the other pane) taken up: stop reading this one.
  unwatch = useApp.subscribe((s) => {
    if (reader === r && s.editor !== r.editor) stopReading(false);
  });
  set({ reading: { status: 'reading', editor } });
  r.start();
}

function finish() {
  reader = null;
  unwatch?.();
  unwatch = null;
  if (get().reading) set({ reading: null });
}

export function pauseReading() {
  reader?.pause();
}

export function resumeReading() {
  reader?.resume();
}

/** Stops reading. With `placeCursor` (the 멈춤 button, Esc) the cursor goes
 * to the sentence being read. */
export function stopReading(placeCursor = true) {
  const r = reader;
  if (r) r.stop(placeCursor);
  finish();
}

/** Ctrl+Shift+R and the header button: start, or stop when reading. */
export function toggleReading() {
  if (reader) stopReading();
  else void startReading();
}

/** Closes the "no Korean voice" message. */
export function closeReadingNotice() {
  if (get().reading?.status === 'noVoice') set({ reading: null });
}
