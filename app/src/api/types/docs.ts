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
  | 'before-copy'
  // Before an editor's corrections were accepted (crates/core/src/corrections).
  | 'before-corrections'
  // Before a save that took a lot of the text away (crates/core/src/doc.rs save_body).
  | 'before-shrink';

export interface Counts {
  withSpaces: number;
  withoutSpaces: number;
  manuscriptLines: number;
  manuscriptPages: number;
  /** Straight marks `. , ! ? ' "` (노벨피아 leaves them out). */
  plainMarks: number;
  /** Characters outside the Basic Multilingual Plane (most emoji). */
  wide: number;
  /** What `<`, `>` and `&` add when counted as HTML (`&lt;`, `&gt;`, `&amp;`). */
  htmlExtra: number;
}

/** How a serial platform counts characters (crates/core/src/count.rs `CountRule`). */
export interface CountRule {
  /** Spaces count (공백 포함). */
  spaces: boolean;
  /** `. , ! ? ' "` do not count. */
  skipMarks: boolean;
  /** Characters outside the Basic Multilingual Plane count as two. */
  wideTwice: boolean;
  /** `<` and `>` count as four, `&` as five. */
  htmlEscapes: boolean;
}

export interface DocSummary {
  id: string;
  title: string;
  synopsis: string;
  status: DocStatus;
  target: number | null;
  /** 완료 회차 잠금: kept as it is (read-only, passed by replace-all and corrections). */
  locked: boolean;
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
  locked: boolean;
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

/** What 고쳐 열기 would make of a document that cannot be read (writer_core::mend). */
export interface MendPreview {
  id: string;
  section: Section;
  title: string;
  /** Why it could not be read. */
  reason: string;
  /** How the text was read: `utf-8`, `utf-16` or `euc-kr`. */
  encoding: string;
  /** The title and other chapter details are made anew. */
  newFront: boolean;
  /** The start of the text as it will read. */
  text: string;
  chars: number;
}

/** What 고쳐 열기 did: the original kept under this name in the same folder. */
export interface Mended {
  id: string;
  kept: string;
}

export interface MetaPatch {
  title?: string;
  synopsis?: string;
  status?: DocStatus;
  /** `null` clears the target; leave out to keep it. */
  target?: number | null;
  /** 잠금 / 잠금 풀기. */
  locked?: boolean;
}
