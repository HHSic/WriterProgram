// Chapters and planning documents: their info, text, counts, records (기록) and the trash.

import type { JSONContent } from '@tiptap/core';

export type Section = 'manuscript' | 'planning' | 'cards' | 'notes';
export type DocStatus =
  | 'draft'
  | 'revise'
  | 'done'
  | 'stock'
  | 'scheduled'
  | 'published'
  | 'final'
  | 'proof1'
  | 'proof2'
  | 'proof3';
export type SnapshotKind =
  | 'auto'
  | 'manual'
  | 'before-replace'
  | 'before-restore'
  | 'before-revise'
  // Writing on two devices (crates/core/src/copies.rs, doc.rs save_body).
  | 'this-device'
  | 'other-device'
  | 'before-reload'
  | 'before-copy';

export interface Counts {
  withSpaces: number;
  withoutSpaces: number;
  manuscriptLines: number;
  manuscriptPages: number;
}

export interface DocSummary {
  id: string;
  title: string;
  synopsis: string;
  status: DocStatus;
  target: number | null;
  counts: Counts;
  /** Estimated pages in the manuscript format; null without paper. */
  pages: number | null;
  /** When the file was last written (ISO time). */
  modified: string | null;
}

export interface DocMeta {
  id: string;
  title: string;
  synopsis: string;
  status: DocStatus;
  target: number | null;
  created: string;
}

export interface DocData {
  meta: DocMeta;
  body: JSONContent;
  counts: Counts;
  /** Fingerprint of the text, sent back with saves. */
  rev: string;
}

export interface SnapshotInfo {
  id: string;
  kind: SnapshotKind;
  name: string;
  at: string;
  counts: Counts;
}

export interface SaveOutcome {
  counts: Counts;
  pages: number | null;
  snapshot: SnapshotInfo | null;
  /** Fingerprint of the text now on disk. */
  rev: string;
  /**
   * The text on disk changed since the editor loaded it (another device), so
   * nothing was saved; the editor's text is kept as a record (`snapshot`).
   */
  conflict: boolean;
}

export interface TrashItem {
  id: string;
  docId: string;
  section: Section;
  title: string;
  deletedAt: string;
  partId: string | null;
  index: number;
  chars: number;
  /** For a copy left by a sync program: the name it comes back under. */
  file?: string | null;
}

export interface NewDoc {
  section?: Section;
  partId?: string;
  after?: string;
  title?: string;
}

export interface MetaPatch {
  title?: string;
  synopsis?: string;
  status?: DocStatus;
  /** `null` clears the target; leave out to keep it. */
  target?: number | null;
}
