import { describe, expect, it } from 'vitest';
import type { UnreadableDoc } from '../api/types';
import { skippedText, unreadableIn, withUnreadable } from './unreadable';

const u = (id: string, index: number, part: string | null = 'p1'): UnreadableDoc => ({
  id,
  section: part ? 'manuscript' : 'planning',
  part,
  index,
  titleGuess: '',
  reason: '내용이 없는 빈 파일',
  repairable: true,
});

const ids = (rows: ReturnType<typeof withUnreadable<string>>) => rows.map((r) => (r.kind === 'doc' ? r.doc : `!${r.item.id}`));

describe('unreadable documents in the tree', () => {
  it('go back to their places', () => {
    expect(ids(withUnreadable(['a', 'b', 'c'], [u('x', 1)]))).toEqual(['a', '!x', 'b', 'c']);
    expect(ids(withUnreadable(['a', 'b'], [u('y', 3), u('x', 0)]))).toEqual(['!x', 'a', 'b', '!y']);
    expect(ids(withUnreadable(['a'], [u('x', 1), u('y', 2)]))).toEqual(['a', '!x', '!y']);
    expect(ids(withUnreadable([], [u('x', 0)]))).toEqual(['!x']);
    expect(ids(withUnreadable(['a'], []))).toEqual(['a']);
  });

  it('are named when a search leaves them out', () => {
    expect(skippedText(2, '회차')).toBe('열 수 없는 회차 2개는 빼고 찾았습니다.');
  });

  it('belong to their own part or the planning list', () => {
    const all = [u('x', 0, 'p1'), u('y', 0, 'p2'), u('z', 0, null)];
    expect(unreadableIn(all, 'p2').map((d) => d.id)).toEqual(['y']);
    expect(unreadableIn(all, null).map((d) => d.id)).toEqual(['z']);
  });
});
