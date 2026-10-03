import { describe, expect, it } from 'vitest';
import type { JSONContent } from '@tiptap/core';
import type { CountRule, Counts } from '../api/types';
import cases from '../../../crates/core/tests/fixtures/counts.json';
import platformCases from '../../../crates/core/tests/fixtures/platform-counts.json';
import { blocksFromJSON, countBlocks, countByRule, countChars } from './counts';

describe('counts match crates/core', () => {
  for (const c of cases as { name: string; body: JSONContent; counts: Counts }[]) {
    it(c.name, () => {
      expect(countBlocks(blocksFromJSON(c.body))).toEqual(c.counts);
    });
  }
});

describe('platform counts match crates/core', () => {
  for (const c of platformCases as { name: string; body: JSONContent; rule: CountRule; chars: number }[]) {
    it(c.name, () => {
      expect(countByRule(countBlocks(blocksFromJSON(c.body)), c.rule)).toBe(c.chars);
    });
  }
});

describe('selection counts', () => {
  it('skip line breaks and spaces', () => {
    expect(countChars('가 나\n다')).toEqual({ withSpaces: 4, withoutSpaces: 3 });
  });
});
