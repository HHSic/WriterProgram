// Find and replace, and exporting documents to txt, docx and hwpx.

import type { SnapshotInfo } from './docs';

export interface TextOptions {
  includeTitles: boolean;
  blankLineBetween: boolean;
  sceneBreak: string;
}

export interface SearchQuery {
  text: string;
  regex: boolean;
  wholeWord: boolean;
  /** Documents to search; leave out for the whole project. */
  docIds?: string[];
}

export interface SearchMatch {
  block: number;
  /** UTF-16 offsets within the paragraph text (line breaks count as one). */
  start: number;
  end: number;
  before: string;
  text: string;
  after: string;
}

export interface SearchResult {
  docs: { docId: string; matches: SearchMatch[] }[];
  total: number;
  truncated: boolean;
  /** Documents in scope that cannot be read, left out (고쳐 열기). */
  skipped: number;
}

export interface ReplaceOutcome {
  replaced: number;
  docs: { docId: string; count: number; snapshot: SnapshotInfo }[];
  /** Documents in scope that cannot be read, left as they are. */
  skipped: number;
}

export interface DocOptions {
  includeTitles: boolean;
  sceneBreak: string;
}

export type FileKind = 'docx' | 'hwpx';

export interface ExportItem {
  docId: string;
  heading: string;
  fileName: string;
}
