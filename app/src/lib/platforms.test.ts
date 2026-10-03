import { describe, expect, it } from 'vitest';
import type { Goal, ProjectInfo } from '../api/types';
import { blocksFromJSON, countBlocks } from '../editor/counts';
import { pasteHtml, previewLines } from './paste';
import { PLATFORMS, defaultPaste, goalChars, platformOf, presetOf, ruleText } from './platforms';

const goal: Goal = { perDoc: 5000, countSpaces: true, daily: null };
const project = (kind: ProjectInfo['kind'], platformId: string | null): Pick<ProjectInfo, 'kind' | 'platform' | 'goal'> => {
  const preset = presetOf(platformId);
  return { kind, goal, platform: preset ? { id: preset.id, rule: preset.rule } : null };
};

const counts = countBlocks(
  blocksFromJSON({ type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: '"가요?" 나는 물었다.' }] }] }),
);

describe('platform counting', () => {
  it('goals follow the platform, or 공백 포함/제외 without one', () => {
    expect(goalChars(project('webnovel', null), counts)).toBe(13);
    expect(goalChars(project('webnovel', 'munpia'), counts)).toBe(13);
    // 노벨피아: no spaces, no straight quotes, question marks or periods.
    expect(goalChars(project('webnovel', 'novelpia'), counts)).toBe(7);
  });

  it('only web novels have a platform', () => {
    expect(platformOf(project('print', 'novelpia'))).toBeNull();
    expect(goalChars(project('print', 'novelpia'), counts)).toBe(13);
  });

  it('every platform has a minimum and words for its rule', () => {
    for (const p of PLATFORMS) {
      expect(p.minimum).toBeGreaterThan(0);
      expect(ruleText(p.rule)).not.toBe('');
    }
  });

  it('paste follows the platform, else empty lines for web novels', () => {
    expect(defaultPaste(project('webnovel', 'novelpia'))).toEqual({ blankLine: true, html: 'none' });
    expect(defaultPaste(project('print', null))).toEqual({ blankLine: false, html: 'none' });
  });
});

describe('paste', () => {
  it('writes a paragraph a line and an empty paragraph for an empty line', () => {
    expect(pasteHtml('가\n\n<나> & 다')).toBe('<p>가</p><p><br></p><p>&lt;나&gt; &amp; 다</p>');
  });

  it('previews the first lines', () => {
    expect(previewLines('가\n\n나', 2)).toEqual([
      { text: '가', blank: false },
      { text: '', blank: true },
    ]);
  });
});
