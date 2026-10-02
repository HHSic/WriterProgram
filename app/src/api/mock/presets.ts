// The format presets and catalog the browser stand-in offers, and its rough page count.

import type { FormatCatalog, ManuscriptFormat } from '../types';

// Same presets as crates/core/src/format.rs.
export const SUBMISSION: ManuscriptFormat = {
  preset: 'submission-a4',
  paper: { kind: 'a4', widthMm: 210, heightMm: 297 },
  margins: { top: 20, bottom: 15, inside: 30, outside: 30, header: 15, footer: 15 },
  font: 'batang',
  sizePt: 10,
  lineSpacing: 160,
  letterSpacing: 0,
  indent: 1,
  blankLineBetween: false,
  chapterNewPage: true,
  pageNumbers: true,
  pageNumberAlign: 'center',
  header: { content: 'none', text: '', align: 'center', skipChapterFirst: true },
  footer: { text: '', align: 'left' },
  indentRules: { chapterFirst: false, afterScene: false, margined: true, dialogue: false, dialogueHang: false },
};
export const WEBNOVEL: ManuscriptFormat = {
  ...SUBMISSION,
  preset: 'webnovel',
  paper: { kind: 'none', widthMm: 210, heightMm: 297 },
  font: 'dotum',
  indent: 0,
  blankLineBetween: true,
  chapterNewPage: false,
  pageNumbers: false,
};
const SHINGUK: ManuscriptFormat = {
  ...SUBMISSION,
  preset: 'book-shinguk',
  paper: { kind: 'shinguk', widthMm: 152, heightMm: 225 },
  margins: { top: 22, bottom: 20, inside: 22, outside: 20, header: 8, footer: 8 },
  font: 'noto-serif',
  lineSpacing: 180,
  letterSpacing: -3,
};
const BOOK46: ManuscriptFormat = {
  ...SHINGUK,
  preset: 'book-46',
  paper: { kind: '46', widthMm: 128, heightMm: 188 },
  margins: { top: 18, bottom: 18, inside: 18, outside: 15, header: 7, footer: 7 },
  sizePt: 9.5,
  lineSpacing: 175,
};
export const CATALOG: Omit<FormatCatalog, 'user'> = {
  builtin: [
    { id: 'submission-a4', name: '투고 원고 (A4)', format: SUBMISSION },
    { id: 'webnovel', name: '웹소설 플랫폼', format: WEBNOVEL },
    { id: 'book-shinguk', name: '책 (신국판)', format: SHINGUK },
    { id: 'book-46', name: '책 (46판)', format: BOOK46 },
  ],
  fonts: [
    { key: 'batang', label: '바탕 계열' },
    { key: 'dotum', label: '돋움 계열' },
    { key: 'nanum-myeongjo', label: '나눔명조' },
    { key: 'noto-serif', label: '본명조' },
    { key: 'gowun-batang', label: '고운바탕' },
  ],
  papers: [
    { key: 'a4', label: 'A4', widthMm: 210, heightMm: 297 },
    { key: 'b5', label: 'B5', widthMm: 182, heightMm: 257 },
    { key: 'a5', label: 'A5', widthMm: 148, heightMm: 210 },
    { key: 'shinguk', label: '신국판', widthMm: 152, heightMm: 225 },
    { key: '46', label: '46판', widthMm: 128, heightMm: 188 },
  ],
};

/** Rough page estimate for the preview; the real one is in crates/core/src/layout.rs. */
export function mockPages(chars: number, f: ManuscriptFormat): number | null {
  if (f.paper.kind === 'none') return null;
  const size = (f.sizePt * 25.4) / 72;
  const perLine = (f.paper.widthMm - f.margins.inside - f.margins.outside) / (size * (1 + f.letterSpacing / 100));
  const lines = Math.floor(
    (f.paper.heightMm - f.margins.top - f.margins.bottom - f.margins.header - f.margins.footer) / ((size * f.lineSpacing) / 100),
  );
  return Math.max(1, Math.ceil(chars / (perLine * lines * 0.85)));
}
