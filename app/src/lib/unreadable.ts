// Documents whose file is there but cannot be read (Overview.unreadable):
// putting them back at their place among the rows the tree shows.

import type { UnreadableDoc } from '../api/types';

export type Row<T> = { kind: 'doc'; doc: T } | { kind: 'unreadable'; item: UnreadableDoc };

/** `docs` with the unreadable ones of the same list at their places (`index`). */
export function withUnreadable<T>(docs: T[], unreadable: UnreadableDoc[]): Row<T>[] {
  const out: Row<T>[] = [];
  let next = 0;
  for (const item of [...unreadable].sort((a, b) => a.index - b.index)) {
    while (out.length < item.index && next < docs.length) out.push({ kind: 'doc', doc: docs[next++] });
    out.push({ kind: 'unreadable', item });
  }
  while (next < docs.length) out.push({ kind: 'doc', doc: docs[next++] });
  return out;
}

/** "열 수 없는 회차 2개는 빼고 찾았습니다." (`noun`: 회차 or 장, or 문서). */
export function skippedText(count: number, noun: string): string {
  return `열 수 없는 ${noun} ${count.toLocaleString('ko-KR')}개는 빼고 찾았습니다.`;
}

/** The unreadable documents of one part, or of the planning list (`null`). */
export function unreadableIn(all: UnreadableDoc[], part: string | null): UnreadableDoc[] {
  return all.filter((u) => (part === null ? u.section === 'planning' : u.section === 'manuscript' && u.part === part));
}
