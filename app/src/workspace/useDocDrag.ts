// Reordering documents in the left column: by mouse drag, or by a long press
// and drag with a finger. Chapters move within and between parts; planning
// documents move within 기획.

import { useState, type DragEvent } from 'react';
import type { DocSummary, Overview, PartView } from '../api/types';
import type { TouchDrag } from '../lib/press';
import { moveDoc } from '../store';

const DRAG_TYPE = 'application/x-writer-doc';

export type DropTarget = { docId: string; after: boolean } | { partId: string } | null;

/** Where a finger dragging a document is over: a row, or a part heading. */
function dropAt(x: number, y: number, dragged: string): DropTarget {
  const el = document.elementFromPoint(x, y);
  const row = el?.closest<HTMLElement>('[data-doc]');
  if (row && row.dataset.doc !== dragged) {
    const rect = row.getBoundingClientRect();
    return { docId: row.dataset.doc!, after: y > rect.top + rect.height / 2 };
  }
  const part = el?.closest<HTMLElement>('[data-part]');
  return part ? { partId: part.dataset.part! } : null;
}

/**
 * The drop target being shown (`drop`), the label following a finger
 * (`ghost`), and the handlers the rows and part headings take.
 */
export function useDocDrag(ov: Overview) {
  const [drop, setDrop] = useState<DropTarget>(null);
  const [ghost, setGhost] = useState<{ label: string; x: number; y: number } | null>(null);

  /** Moves `dragged` to a drop target: before or after a row, or to the end of a part. */
  const dropOn = (dragged: string, target: DropTarget) => {
    if (!target) return;
    const planning = ov.planning.some((d) => d.id === dragged);
    if ('partId' in target) {
      const part = ov.parts.find((p) => p.id === target.partId);
      if (part && !planning) void moveDoc(dragged, part.id, part.docs.filter((d) => d.id !== dragged).length);
      return;
    }
    if (target.docId === dragged) return;
    const part = ov.parts.find((p) => p.docs.some((d) => d.id === target.docId)) ?? null;
    // Chapters stay in the manuscript and planning documents in 기획.
    if ((part === null) !== planning) return;
    const list = part ? part.docs : ov.planning;
    const rest = list.map((d) => d.id).filter((id) => id !== dragged);
    void moveDoc(dragged, part?.id ?? null, rest.indexOf(target.docId) + (target.after ? 1 : 0));
  };

  const onDragStart = (e: DragEvent, docId: string) => {
    e.dataTransfer.setData(DRAG_TYPE, docId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const onDragOverDoc = (e: DragEvent, docId: string) => {
    if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
    e.preventDefault();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setDrop({ docId, after: e.clientY > rect.top + rect.height / 2 });
  };

  const onDropDoc = (e: DragEvent) => {
    e.preventDefault();
    const dragged = e.dataTransfer.getData(DRAG_TYPE);
    const target = drop;
    setDrop(null);
    if (dragged) dropOn(dragged, target);
  };

  const onDragOverPart = (e: DragEvent, part: PartView) => {
    if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
    e.preventDefault();
    setDrop({ partId: part.id });
  };

  const onDropPart = (e: DragEvent, part: PartView) => {
    e.preventDefault();
    setDrop(null);
    const dragged = e.dataTransfer.getData(DRAG_TYPE);
    if (dragged) dropOn(dragged, { partId: part.id });
  };

  /** A drag left a heading or ended without a drop. */
  const clearDrop = () => setDrop(null);

  /** A long press and drag with a finger does what a mouse drag does. */
  const touchDrag = (doc: DocSummary, label: string): TouchDrag => ({
    start: (x, y) => setGhost({ label, x, y }),
    move: (x, y) => {
      setGhost((g) => (g ? { ...g, x, y } : g));
      setDrop(dropAt(x, y, doc.id));
    },
    end: (x, y, cancelled) => {
      const target = cancelled ? null : dropAt(x, y, doc.id);
      setGhost(null);
      setDrop(null);
      dropOn(doc.id, target);
    },
  });

  return { drop, ghost, onDragStart, onDragOverDoc, onDropDoc, onDragOverPart, onDropPart, clearDrop, touchDrag };
}
