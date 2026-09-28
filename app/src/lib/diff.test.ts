import { describe, expect, it } from 'vitest';
import { diffLetters, diffParagraphs, placesChanged } from './diff';

describe('diffParagraphs', () => {
  it('lines up same, changed, added and removed paragraphs', () => {
    const left = ['첫 문단.', '비가 왔다.', '지운 문단.', '끝.'];
    const right = ['첫 문단.', '비가 많이 왔다.', '끝.', '새 문단.'];
    const rows = diffParagraphs(left, right);
    expect(rows.map((r) => r.kind)).toEqual(['same', 'changed', 'removed', 'same', 'added']);
    expect(placesChanged(rows)).toBe(2);
    const changed = rows[1];
    if (changed.kind !== 'changed') throw new Error('expected a changed row');
    expect(changed.left).toEqual([{ text: '비가 왔다.', changed: false }]);
    expect(changed.right).toEqual([
      { text: '비가 ', changed: false },
      { text: '많이 ', changed: true },
      { text: '왔다.', changed: false },
    ]);
  });

  it('finds nothing to mark in the same text', () => {
    const rows = diffParagraphs(['가', '나'], ['가', '나']);
    expect(rows.every((r) => r.kind === 'same')).toBe(true);
    expect(placesChanged(rows)).toBe(0);
  });

  it('handles an empty side', () => {
    expect(diffParagraphs([], ['새 글']).map((r) => r.kind)).toEqual(['added']);
    expect(diffParagraphs(['옛 글'], []).map((r) => r.kind)).toEqual(['removed']);
  });
});

describe('diffLetters', () => {
  it('marks letters, not bytes, of Korean text and emoji', () => {
    const { left, right } = diffLetters('서하는 웃었다 😀', '서하가 웃었다 😀');
    expect(left.filter((p) => p.changed).map((p) => p.text)).toEqual(['는']);
    expect(right.filter((p) => p.changed).map((p) => p.text)).toEqual(['가']);
  });

  it('marks very long paragraphs as a whole', () => {
    const a = '가'.repeat(1000);
    const b = '나'.repeat(1000);
    const { left, right } = diffLetters(a, b);
    expect(left).toEqual([{ text: a, changed: true }]);
    expect(right).toEqual([{ text: b, changed: true }]);
  });
});
