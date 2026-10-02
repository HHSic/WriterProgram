// Where new chapters go; boards in the middle column.

import { get, set } from './state';
import { openTarget } from './tabs';

/** Picks a part in the tree as the place for 새 회차. */
export function selectPart(id: string | null) {
  set({ selectedPartId: id });
}

/** Where 새 회차 goes: the part picked in the tree, or else the open chapter's part. */
export function newDocPartId(): string | undefined {
  const { overview: ov, selectedPartId, activeDocId } = get();
  if (!ov) return undefined;
  if (selectedPartId && ov.parts.some((p) => p.id === selectedPartId)) return selectedPartId;
  return ov.parts.find((p) => p.docs.some((d) => d.id === activeDocId))?.id;
}

export function openNotesBoard() {
  return openTarget({ kind: 'notes', id: 'all' });
}

export function openTable(partId: string) {
  return openTarget({ kind: 'table', id: partId });
}
