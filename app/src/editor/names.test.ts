import { describe, expect, it } from 'vitest';
import type { CardSummary } from '../api/types';
import fixture from '../../../crates/core/tests/fixtures/names.json';
import { nameIndex } from './cards';
import { NAME_ENDINGS, findNames } from './names';

function card(id: string, name: string, aliases: string[] = []): CardSummary {
  return { id, cardType: 'person', name, aliases, highlight: true, summary: '' };
}

describe('names stand as words, as in crates/core', () => {
  it('uses the same endings', () => {
    expect(NAME_ENDINGS).toBe(fixture.endings);
  });

  for (const c of fixture.cases as { name: string; names: string[]; text: string; found: [number, string][] }[]) {
    it(c.name, () => {
      // The Rust side takes the names in the order given.
      const regex = new RegExp(c.names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'gu');
      const found = findNames(regex, c.text).map((m): [number, string] => [[...c.text.slice(0, m.index)].length, m.name]);
      expect(found).toEqual(c.found);
    });
  }
});

describe('card name highlighting', () => {
  it('skips 서하 inside 서하늘 and keeps it before particles', () => {
    const index = nameIndex([card('a', '서하')])!;
    const found = (text: string) => findNames(index.regex, text).map((m) => m.index);
    expect(found('서하늘')).toEqual([]);
    expect(found('서하가')).toEqual([0]);
    expect(found('서하는')).toEqual([0]);
    expect(found('서하의')).toEqual([0]);
    expect(found('서하에게')).toEqual([0]);
    expect(found('“서하.”')).toEqual([1]);
    expect(found('「서하」')).toEqual([1]);
    expect(found("'서하'")).toEqual([1]);
  });

  it('leaves the pattern ready for the next text', () => {
    const index = nameIndex([card('a', '서하', ['윤서하'])])!;
    expect(findNames(index.regex, '윤서하가 서하늘을 본다')).toEqual([{ index: 0, name: '윤서하' }]);
    expect(index.regex.lastIndex).toBe(0);
    expect(findNames(index.regex, '서하')).toEqual([{ index: 0, name: '서하' }]);
  });
});
