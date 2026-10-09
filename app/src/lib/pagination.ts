import type { ManuscriptFormat } from '../api/types';

export const PAGE_GAP = 48;
/** Scale paper and physical margins together to the available editor width. */
export function pageGeometry(width: number, format: ManuscriptFormat) {
  const finite = (n: number, fallback: number) => Number.isFinite(n) && n > 0 ? n : fallback;
  const paperWidth = finite(format.paper.widthMm, 210);
  const paperHeight = finite(format.paper.heightMm, 297);
  const scale = Math.max(1, width) / paperWidth;
  const margin = (n: number) => Math.max(0, Number.isFinite(n) ? n : 0) * scale;
  const margins = format.paper.kind === 'none' ? { inside: 30, outside: 30, top: 20, bottom: 15, header: 15, footer: 15 } : format.margins;
  const left = margin(margins.inside);
  const right = margin(margins.outside);
  const top = margin(margins.top + margins.header);
  const bottom = margin(margins.bottom + margins.footer);
  return { width: Math.max(1, width - left - right), height: Math.max(80, paperHeight * scale - top - bottom), left, right, top, bottom };
}
export function pageCount(width: number, extent: number) {
  return Math.max(1, Math.ceil((extent + PAGE_GAP - 0.5) / (Math.max(1, width) + PAGE_GAP)));
}
