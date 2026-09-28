// Data exchanged with the Rust side (app/src-tauri/src/commands.rs and
// crates/core). Field names follow the serde camelCase renames there.

import type { JSONContent } from '@tiptap/core';

export type ProjectKind = 'webnovel' | 'print';
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
export type SnapshotKind = 'auto' | 'manual' | 'before-replace' | 'before-restore' | 'before-revise';

export interface Counts {
  withSpaces: number;
  withoutSpaces: number;
  manuscriptLines: number;
  manuscriptPages: number;
}

export interface Goal {
  perDoc: number | null;
  countSpaces: boolean;
  daily: number | null;
}

/** 원고 서식 (crates/core/src/format.rs). mm, pt, %. */
export interface ManuscriptFormat {
  preset: string;
  paper: { kind: string; widthMm: number; heightMm: number };
  margins: { top: number; bottom: number; inside: number; outside: number; header: number; footer: number };
  font: string;
  sizePt: number;
  lineSpacing: number;
  letterSpacing: number;
  indent: number;
  blankLineBetween: boolean;
  chapterNewPage: boolean;
  pageNumbers: boolean;
  /** 머리말 (crates/core/src/format.rs RunningHead). */
  header: RunningHead;
}

export type HeadContent = 'none' | 'title' | 'chapter' | 'titleChapter' | 'author' | 'custom';
export type HeadAlign = 'left' | 'center' | 'right' | 'outside';

export interface RunningHead {
  content: HeadContent;
  /** For content 'custom'. */
  text: string;
  align: HeadAlign;
  /** Left off each chapter's first page (when chapters start on a new page). */
  skipChapterFirst: boolean;
}

export interface UserPreset {
  name: string;
  format: ManuscriptFormat;
}

export interface FormatCatalog {
  builtin: { id: string; name: string; format: ManuscriptFormat }[];
  user: UserPreset[];
  fonts: { key: string; label: string }[];
  papers: { key: string; label: string; widthMm: number; heightMm: number }[];
}

export interface ProjectInfo {
  id: string;
  title: string;
  kind: ProjectKind;
  penName: string;
  created: string;
  goal: Goal;
  sceneBreak: string;
  manuscriptFormat: ManuscriptFormat;
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

export interface PartView {
  id: string;
  title: string;
  docs: DocSummary[];
}

/** 설정집 분류 (crates/core/src/cards.rs). */
export interface CardType {
  id: string;
  name: string;
  fields: string[];
}

export interface CardSummary {
  id: string;
  cardType: string;
  name: string;
  aliases: string[];
  highlight: boolean;
  summary: string;
}

export interface Card {
  id: string;
  cardType: string;
  name: string;
  aliases: string[];
  /** [name, value] pairs, in order. */
  fields: [string, string][];
  highlight: boolean;
  created: string;
  description: string;
}

/** What a note (메모) hangs on: a stretch of text, a document, a card or the project. */
export type NoteAnchor = 'text' | 'doc' | 'card' | 'project';

export interface NoteReply {
  at: string;
  text: string;
}

export interface Note {
  id: string;
  anchor: NoteAnchor;
  /** Document (text, doc) or card id; empty for the project. */
  target: string;
  /** The marked text when last saved. */
  quote: string;
  text: string;
  replies: NoteReply[];
  tags: string[];
  done: boolean;
  created: string;
  updated: string;
}

export interface NewNote {
  id?: string;
  anchor: NoteAnchor;
  target?: string;
  quote?: string;
  text?: string;
}

export interface Appearance {
  docId: string;
  count: number;
  samples: SearchMatch[];
}

export interface Overview {
  root: string;
  project: ProjectInfo;
  parts: PartView[];
  planning: DocSummary[];
  trashCount: number;
  total: Counts;
  totalPages: number | null;
  cardTypes: CardType[];
  cards: CardSummary[];
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
}

export interface RecentItem {
  path: string;
  title: string;
  kind: ProjectKind;
  chars: number;
  openedAt: string;
  exists: boolean;
}

export interface NewProject {
  parent: string;
  title: string;
  kind: ProjectKind;
  perDocGoal: number | null;
  countSpaces: boolean;
  firstChapter: boolean;
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

export interface ProjectPatch {
  title?: string;
  kind?: ProjectKind;
  penName?: string;
  goal?: Goal;
  sceneBreak?: string;
  manuscriptFormat?: ManuscriptFormat;
}

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
}

export interface ReplaceOutcome {
  replaced: number;
  docs: { docId: string; count: number; snapshot: SnapshotInfo }[];
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

export interface Backend {
  /** True in the desktop app, false in the browser preview. */
  isDesktop: boolean;
  recentList(): Promise<RecentItem[]>;
  recentRemove(path: string): Promise<void>;
  defaultLocation(): Promise<string>;
  projectCreate(opts: NewProject): Promise<Overview>;
  projectOpen(path: string): Promise<Overview>;
  projectOverview(root: string): Promise<Overview>;
  projectUpdate(root: string, patch: ProjectPatch): Promise<ProjectInfo>;
  partAdd(root: string, title: string): Promise<string>;
  partRename(root: string, partId: string, title: string): Promise<void>;
  partRemove(root: string, partId: string): Promise<void>;
  docAdd(root: string, spec: NewDoc): Promise<string>;
  docMove(root: string, docId: string, partId: string | null, index: number): Promise<void>;
  docTrash(root: string, docId: string): Promise<TrashItem>;
  docLoad(root: string, docId: string): Promise<DocData>;
  docSave(root: string, docId: string, body: JSONContent): Promise<SaveOutcome>;
  docUpdateMeta(root: string, docId: string, patch: MetaPatch): Promise<DocMeta>;
  snapshotList(root: string, docId: string): Promise<SnapshotInfo[]>;
  snapshotCreate(root: string, docId: string, name: string): Promise<SnapshotInfo>;
  snapshotLoad(root: string, docId: string, snapshotId: string): Promise<DocData>;
  snapshotRestore(root: string, docId: string, snapshotId: string): Promise<SnapshotInfo>;
  trashList(root: string): Promise<TrashItem[]>;
  trashRestore(root: string, trashId: string): Promise<void>;
  trashDelete(root: string, trashId: string): Promise<void>;
  exportText(root: string, items: ExportItem[], opts: TextOptions): Promise<string>;
  exportTxt(root: string, items: ExportItem[], opts: TextOptions, dest: string, perDoc: boolean): Promise<string[]>;
  exportFile(
    root: string,
    items: ExportItem[],
    opts: DocOptions,
    format: ManuscriptFormat,
    kind: FileKind,
    dest: string,
    perDoc: boolean,
  ): Promise<string[]>;
  search(root: string, query: SearchQuery): Promise<SearchResult>;
  replaceAll(root: string, query: SearchQuery, replacement: string): Promise<ReplaceOutcome>;
  cardLoad(root: string, cardId: string): Promise<Card>;
  cardCreate(root: string, typeId: string, name: string): Promise<Card>;
  cardSave(root: string, card: Card): Promise<CardSummary>;
  cardTrash(root: string, cardId: string): Promise<TrashItem>;
  cardAppearances(root: string, cardId: string): Promise<Appearance[]>;
  cardCounts(root: string): Promise<[string, number][]>;
  cardTypeAdd(root: string, name: string): Promise<CardType>;
  cardTypeUpdate(root: string, kind: CardType): Promise<void>;
  cardTypeRemove(root: string, typeId: string): Promise<void>;
  noteList(root: string): Promise<Note[]>;
  noteCreate(root: string, spec: NewNote): Promise<Note>;
  noteSave(root: string, note: Note): Promise<Note>;
  noteTrash(root: string, noteId: string): Promise<TrashItem>;
  formatCatalog(): Promise<FormatCatalog>;
  formatSavePreset(name: string, format: ManuscriptFormat): Promise<UserPreset[]>;
  formatDeletePreset(name: string): Promise<UserPreset[]>;
  formatEstimate(root: string, format: ManuscriptFormat): Promise<number | null>;
  reveal(path: string): Promise<void>;
  pickFolder(title: string, defaultPath?: string): Promise<string | null>;
  /** `extension` without the dot: txt, docx or hwpx. */
  pickSaveFile(title: string, defaultName: string, extension: string): Promise<string | null>;
  /**
   * Runs `handler` before the window closes (used to finish saving). The
   * window stays open when it resolves to false.
   */
  onCloseRequested(handler: () => Promise<boolean>): void;
  setWindowTitle(title: string): void;
}
