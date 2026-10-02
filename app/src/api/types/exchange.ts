// 교정본 주고받기 (crates/core/src/corrections): chapters sent to an editor,
// the corrected file read against them, and the writer's decisions.
// docs/corrections.md has the flow and the reading rules.

import type { FileKind } from './search-export';

export interface SentChapter {
  docId: string;
  title: string;
  /** The heading line it was sent with. */
  heading: string;
  /** The file it went in. */
  file: string;
  /** Fingerprint of the text sent. */
  fingerprint: string;
}

export interface Received {
  at: string;
  name: string;
  /** Copy kept with the record (relative to its folder). */
  stored: string;
  fingerprint: string;
}

export interface Exchange {
  id: string;
  created: string;
  kind: FileKind;
  files: string[];
  sceneBreak: string;
  chapters: SentChapter[];
  received: Received[];
}

export interface ExchangeInfo extends Exchange {
  /** Changes still waiting for a decision; null before a corrected file is read. */
  pending: number | null;
}

/** 띄어쓰기 · 문장부호 · 문장 고침. */
export type ChangeClass = 'spacing' | 'punctuation' | 'wording';
/** merge: two paragraphs became one; split: one became two. */
export type ChangeKind = 'insert' | 'delete' | 'replace' | 'merge' | 'split';
/** tracked: 변경 내용 추적; strike: struck through; color: new coloured text; compared: plain comparison. */
export type Found = 'tracked' | 'strike' | 'color' | 'compared';
export type DecisionState = 'pending' | 'accepted' | 'rejected';

/** Paragraph (block) index and UTF-16 offset in it. A scene break is one character. */
export interface ReviewPos {
  block: number;
  offset: number;
}

export interface ReviewSpan {
  from: ReviewPos;
  to: ReviewPos;
}

/**
 * One correction. In `before`/`after`, U+2029 is a paragraph break, `\n` a
 * line break and U+E000 a scene break.
 */
export interface Correction {
  id: string;
  kind: ChangeKind;
  class: ChangeClass;
  before: string;
  after: string;
  /** A few characters before and after, for a list. */
  lead: string;
  trail: string;
  /** In the text as sent. */
  at: ReviewSpan;
  /** In the chapter now; null when it cannot be placed. */
  now: ReviewSpan | null;
  /** 겹침: the writer changed this paragraph after sending too. */
  overlap: boolean;
  how: Found;
  author: string | null;
  date: string | null;
  state: DecisionState;
}

/** 살펴볼 곳: unchanged text the editor underlined, coloured or highlighted. */
export interface ReviewLook {
  id: string;
  at: ReviewSpan;
  now: ReviewSpan | null;
  quote: string;
  underline: boolean;
  color: boolean;
  highlight: boolean;
}

/** A note the editor left (한글 메모, Word comment). */
export interface EditorNote {
  id: string;
  text: string;
  author: string;
  date: string;
  at: ReviewSpan | null;
  now: ReviewSpan | null;
  quote: string;
  /** accepted: made into a 메모 (`memo`); rejected: left out. */
  state: DecisionState;
  memo: string | null;
}

export interface ChapterReview {
  docId: string;
  title: string;
  /** The chapter's text is in the corrected file. */
  found: boolean;
  /** Deleted since it was sent. */
  gone: boolean;
  /** Changed by the writer since it was sent. */
  edited: boolean;
  changes: Correction[];
  looks: ReviewLook[];
  notes: EditorNote[];
}

export interface Review {
  exchange: string;
  file: string;
  stored: string;
  at: string;
  chapters: ChapterReview[];
}

export interface Decisions {
  accept?: string[];
  reject?: string[];
  /** Accept every pending change of these classes that can be applied. */
  acceptClasses?: ChangeClass[];
  /** Editor notes to keep as 메모, and to leave out. */
  keepNotes?: string[];
  dropNotes?: string[];
}

export interface Applied {
  /** Chapters whose text changed. */
  docs: string[];
  accepted: string[];
  rejected: string[];
  /** Not applied (겹침, deleted chapter), with the reason in screen words. */
  skipped: { id: string; reason: string }[];
  /** 메모 made from the editor's notes. */
  memos: string[];
  review: Review;
}
