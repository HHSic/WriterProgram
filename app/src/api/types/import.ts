// 가져오기 (import.rs): reading chapters from txt, md, docx and hwpx files.

import type { HeadAlign, ManuscriptFormat, RunningFoot, RunningHead } from './format';

export type SplitRule = 'auto' | 'file' | 'heading' | 'episode' | 'chapter' | 'number' | 'regex';
export type LineMode = 'auto' | 'line' | 'blank';

export interface ImportOptions {
  rule: SplitRule;
  /** Used with the `regex` rule. */
  pattern: string;
  lineMode: LineMode;
  /** `utf-8` or `euc-kr` for txt and md; found out when null. */
  encoding: string | null;
}

export interface ImportFile {
  name: string;
  kind: string;
  /** Why the file cannot be imported. */
  error: string | null;
  encoding: string | null;
  /** The way the file was cut into chapters, in screen words. */
  rule: string;
  chapters: number;
  /** What a 한글 file says about its pages (crates/core/src/import/page.rs). */
  page: PageSetup | null;
}

export interface PageSetup {
  paper: ManuscriptFormat['paper'];
  margins: ManuscriptFormat['margins'];
  header: RunningHead;
  pageNumbers: HeadAlign | null;
  footer: RunningFoot;
  /** The same in a sentence. */
  summary: string;
}

/** What is left out: tables, pictures and footnotes are never imported. */
export interface SkipTotals {
  tables: number;
  images: number;
  footnotes: number;
}

export interface ImportChapter {
  index: number;
  file: number;
  title: string;
  chars: number;
  paragraphs: number;
  snippet: string;
  skipped: SkipTotals;
}

export interface ImportPreview {
  files: ImportFile[];
  chapters: ImportChapter[];
  skipped: SkipTotals;
}

export interface ImportSpec {
  partId: string | null;
  after: string | null;
  picks: { index: number; title: string }[];
  leaveNotes: boolean;
  /** Make the 원고 서식 follow the pages of the first 한글 file. */
  pageSetup: boolean;
}

export interface Imported {
  docs: string[];
  notes: number;
  /** The 원고 서식 now follows the file's pages. */
  format: boolean;
}
