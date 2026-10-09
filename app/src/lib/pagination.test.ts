import { expect, it } from 'vitest';
import { PAGE_GAP, pageCount, pageGeometry } from './pagination';
import type { ManuscriptFormat } from '../api/types';
it('scales the paper and margins together, including header and footer areas', () => {
  const format = { paper: { widthMm: 210, heightMm: 297 }, margins: { inside: 30, outside: 20, top: 20, bottom: 20, header: 10, footer: 10 } } as ManuscriptFormat;
  expect(pageGeometry(630, format)).toEqual({ width: 480, height: 711, left: 90, right: 60, top: 90, bottom: 90 });
  expect(pageCount(480, 480)).toBe(1);
  expect(pageCount(480, 3 * (480 + PAGE_GAP) - PAGE_GAP)).toBe(3);
});
