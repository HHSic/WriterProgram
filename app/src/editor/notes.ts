// Notes (메모) on stretches of text: the `memo` mark in the editor
// (extensions.ts) carries the note id. Adding or taking away a note's mark is
// not an undo step, so undo never leaves a mark without its note.

import type { Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { TextSelection } from '@tiptap/pm/state';

export interface Range {
  from: number;
  to: number;
}

/** Stretches of text marked for a note, in order, touching ones joined. */
export function markRanges(doc: PmNode, noteId: string): Range[] {
  const ranges: Range[] = [];
  doc.descendants((node, pos) => {
    if (!node.isText) return true;
    if (!node.marks.some((m) => m.type.name === 'memo' && m.attrs.id === noteId)) return false;
    const last = ranges[ranges.length - 1];
    if (last && last.to === pos) last.to = pos + node.nodeSize;
    else ranges.push({ from: pos, to: pos + node.nodeSize });
    return false;
  });
  return ranges;
}

/** Where each note's marked text starts, for listing notes in text order. */
export function markStarts(doc: PmNode): Map<string, number> {
  const starts = new Map<string, number>();
  doc.descendants((node, pos) => {
    if (!node.isText) return true;
    for (const m of node.marks) {
      if (m.type.name === 'memo' && typeof m.attrs.id === 'string' && !starts.has(m.attrs.id)) {
        starts.set(m.attrs.id, pos);
      }
    }
    return false;
  });
  return starts;
}

/** The note's marked text as it reads now (first stretch), or null when it is gone. */
export function markedText(doc: PmNode, noteId: string): string | null {
  const first = markRanges(doc, noteId)[0];
  return first ? doc.textBetween(first.from, first.to, ' ') : null;
}

export function addMark(editor: Editor, noteId: string, range: Range) {
  const type = editor.schema.marks.memo;
  const tr = editor.state.tr.addMark(range.from, range.to, type.create({ id: noteId }));
  editor.view.dispatch(tr.setMeta('addToHistory', false));
}

export function removeMarks(editor: Editor, noteId: string) {
  if (editor.isDestroyed) return;
  const type = editor.schema.marks.memo;
  const ranges = markRanges(editor.state.doc, noteId);
  if (!ranges.length) return;
  const tr = editor.state.tr;
  for (const r of ranges) tr.removeMark(r.from, r.to, type.create({ id: noteId }));
  editor.view.dispatch(tr.setMeta('addToHistory', false));
}

/** Selects the note's text and brings it to the middle of the view. */
export function selectNote(editor: Editor, noteId: string): boolean {
  if (editor.isDestroyed) return false;
  const first = markRanges(editor.state.doc, noteId)[0];
  if (!first) return false;
  editor.view.dispatch(editor.state.tr.setSelection(TextSelection.create(editor.state.doc, first.from, first.to)));
  const dom = editor.view.domAtPos(first.from).node;
  const el = dom instanceof HTMLElement ? dom : dom.parentElement;
  el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  return true;
}
