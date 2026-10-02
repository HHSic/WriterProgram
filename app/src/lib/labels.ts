// Screen words for kinds and statuses (docs/terminology.md, docs/layout-data.md).

import type { DocStatus, FormatCatalog, ManuscriptFormat, PartView, ProjectKind, SnapshotKind } from '../api/types';

export const KIND_LABEL: Record<ProjectKind, string> = {
  webnovel: '웹소설',
  print: '출판 장편',
};

export const STATUS_LABEL: Record<DocStatus, string> = {
  draft: '초안',
  revise: '퇴고',
  done: '완료',
  stock: '비축',
  scheduled: '예약',
  published: '연재됨',
  final: '탈고',
  proof1: '1교',
  proof2: '2교',
  proof3: '3교',
};

const COMMON: DocStatus[] = ['draft', 'revise', 'done'];

export function statusesFor(kind: ProjectKind): DocStatus[] {
  return kind === 'webnovel'
    ? [...COMMON, 'stock', 'scheduled', 'published']
    : [...COMMON, 'final', 'proof1', 'proof2', 'proof3'];
}

export const RECORD_KIND_LABEL: Record<SnapshotKind, string> = {
  auto: '자동 기록',
  manual: '직접 보관',
  'before-replace': '바꾸기 전',
  'before-restore': '되돌리기 전',
  'before-revise': '퇴고 전',
  'this-device': '이 기기에서 쓴 글',
  'other-device': '다른 기기에서 쓴 글',
  'before-reload': '다른 기기 것 불러오기 전',
  'before-copy': '사본으로 바꾸기 전',
  'before-corrections': '교정 반영 전',
};

/** "12화" for web novels, "12장" for print. */
export function docNumber(kind: ProjectKind, n: number): string {
  return kind === 'webnovel' ? `${n}화` : `${n}장`;
}

/** What one manuscript document is called: 회차 or 장. */
export function docNoun(kind: ProjectKind): string {
  return kind === 'webnovel' ? '회차' : '장';
}

export const UNTITLED = '제목 없음';

/** 비축 N화: chapters finished but not yet published (비축 + 예약). */
export function stockCount(parts: PartView[]): number {
  return parts.reduce(
    (n, part) => n + part.docs.filter((d) => d.status === 'stock' || d.status === 'scheduled').length,
    0,
  );
}

function hasFinal(word: string): boolean {
  const code = word.charCodeAt(word.length - 1) - 0xac00;
  return code >= 0 && code <= 11171 && code % 28 !== 0;
}

/** Adds the object particle: "회차를", "장을". */
export function withObject(word: string): string {
  return `${word}${hasFinal(word) ? '을' : '를'}`;
}

/** Adds the subject particle: "회차가", "장이". */
export function withSubject(word: string): string {
  return `${word}${hasFinal(word) ? '이' : '가'}`;
}

/** Screen name of a manuscript format: its preset's name, marked when changed. */
export function formatName(format: ManuscriptFormat, catalog: FormatCatalog | null): string {
  const builtin = catalog?.builtin.find((b) => b.id === format.preset);
  const user = catalog?.user.find((u) => u.name === format.preset);
  const base = builtin?.format ?? user?.format;
  const name = builtin?.name ?? user?.name ?? '직접 정한 서식';
  if (!base) return name;
  const same = JSON.stringify({ ...base, preset: '' }) === JSON.stringify({ ...format, preset: '' });
  return same ? name : `${name} (바꿈)`;
}

/** Paper name for "A4 예상 12쪽". */
export function paperName(format: ManuscriptFormat, catalog: FormatCatalog | null): string {
  if (format.paper.kind === 'custom') return `${format.paper.widthMm}×${format.paper.heightMm}mm`;
  return catalog?.papers.find((p) => p.key === format.paper.kind)?.label ?? '';
}
