// 메모.

import { api } from '../api';
import type { NewNote, Note } from '../api/types';
import { addMark, removeMarks } from '../editor/notes';
import { editorsOf } from '../editor/shared';
import { flushAll } from '../lib/flush';
import { newId } from '../lib/ids';
import { get, root, set } from './state';
import { showToast, toastError } from './ui';
import { refreshOverview } from './project';
import { selectDoc } from './docs';
import { openCard } from './cards';

// ---------------------------------------------------------------------------
// Notes (메모)

export async function loadNotes() {
  try {
    set({ notes: await api.noteList(root()) });
  } catch (e) {
    toastError('메모를 읽지 못함', e);
  }
}

/** Shows a note in the 메모 tab (and marks its text). */
export function focusNote(id: string | null) {
  if (!id) {
    set({ focusNoteId: null });
    return;
  }
  set({ focusNoteId: id, rightOpen: true, rightTab: 'notes', previewCardId: null });
}

/** Replaces one note after it was saved. */
export function patchNote(note: Note) {
  const notes = get().notes;
  set({ notes: notes.some((n) => n.id === note.id) ? notes.map((n) => (n.id === note.id ? note : n)) : [...notes, note] });
}

export async function addNote(spec: NewNote): Promise<Note | null> {
  try {
    const note = await api.noteCreate(root(), spec);
    set({ notes: [...get().notes, note] });
    focusNote(note.id);
    return note;
  } catch (e) {
    toastError('메모를 만들지 못함', e);
    return null;
  }
}

/** A note on the text selected in the open document (Ctrl+Alt+M, 메모 button). */
export async function addTextNote() {
  const { editor, activeDocId } = get();
  if (!editor || !activeDocId || !editor.isEditable) return;
  const { from, to, empty } = editor.state.selection;
  if (empty) {
    showToast({ text: '메모를 붙일 글을 먼저 고르세요' });
    return;
  }
  // Mark first, so the note follows the text even while it is being made.
  const id = newId();
  const quote = editor.state.doc.textBetween(from, to, ' ');
  addMark(editor, id, { from, to });
  const note = await addNote({ id, anchor: 'text', target: activeDocId, quote });
  if (!note) for (const e of editorsOf(activeDocId)) removeMarks(e, id);
}

/** Opens what a note is on: its document with the text selected, or its card. */
export async function showNote(note: Note) {
  if (note.anchor === 'text' || note.anchor === 'doc') {
    if (note.anchor === 'text') set({ pendingNote: note.id });
    await selectDoc(note.target);
  } else if (note.anchor === 'card') {
    await openCard(note.target);
  }
  focusNote(note.id);
}

export async function trashNote(id: string) {
  const ov = get().overview;
  const note = get().notes.find((n) => n.id === id);
  if (!ov || !note) return;
  try {
    if (note.anchor === 'text') {
      // The open document saves without the mark before the note goes.
      for (const e of editorsOf(note.target)) removeMarks(e, id);
      await flushAll();
    }
    const item = await api.noteTrash(ov.root, id);
    set({
      notes: get().notes.filter((n) => n.id !== id),
      focusNoteId: get().focusNoteId === id ? null : get().focusNoteId,
    });
    void refreshOverview();
    showToast({
      text: '메모를 휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void api
            .trashRestore(ov.root, item.id)
            .then(() => Promise.all([loadNotes(), refreshOverview()]))
            .catch((e: unknown) => toastError('되돌리지 못함', e));
        },
      },
    });
  } catch (e) {
    toastError('메모를 휴지통으로 옮기지 못함', e);
  }
}

/** Open notes on each document, for the tree. */
export function openNoteCounts(notes: Note[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const n of notes) {
    if (n.done || (n.anchor !== 'text' && n.anchor !== 'doc')) continue;
    counts.set(n.target, (counts.get(n.target) ?? 0) + 1);
  }
  return counts;
}
