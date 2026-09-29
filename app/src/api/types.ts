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
  /** Where the first-line indent is left out (crates/core/src/indent.rs). */
  indentRules: IndentRules;
}

export interface IndentRules {
  /** The first paragraph of a chapter. */
  chapterFirst: boolean;
  /** The first paragraph after a scene break. */
  afterScene: boolean;
  /** Paragraphs set in with margins (letters, quotations). */
  margined: boolean;
  /** Dialogue (opening with a quotation mark). */
  dialogue: boolean;
  /** Dialogue as on 원고지: every line one cell in. */
  dialogueHang: boolean;
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
  /** Copies left by sync programs, waiting for the writer to pick. */
  copies: CopyInfo[];
}

/** A copy of a file left by a sync program (crates/core/src/copies.rs). */
export interface CopyInfo {
  /** File name in its folder, e.g. `k7q2m9x4t1ab-DESKTOP-1AB2C3D.md`. */
  file: string;
  section: Section;
  /** Id of the document, card or note it is a copy of. */
  of: string;
  title: string;
  modified: string | null;
  chars: number;
  /** The device it came from, when the sync program named it. */
  device: string | null;
}

/** take: the copy replaces the original; discard: it goes to the trash; keepBoth: it becomes its own. */
export type CopyAction = 'take' | 'discard' | 'keepBoth';

/** Where projects are kept (crates/core/src/places.rs). */
export type Service = 'onedrive' | 'googledrive' | 'dropbox' | 'icloud' | 'local';

export interface Place {
  service: Service;
  label: string;
  /** The folder the program keeps in step. */
  root: string;
  /** Where new projects go in this place. */
  suggested: string;
}

/** Drives the app can keep projects in step with directly (crates/sync). */
export type DriveProvider = 'google' | 'onedrive' | 'dropbox';

export interface DriveAccount {
  name: string;
  email: string;
}

export interface DriveInfo {
  provider: DriveProvider;
  label: string;
  /** The app is registered with this drive, so it can sign in. */
  registered: boolean;
  account: DriveAccount | null;
  connectedAt: string | null;
}

export interface DriveStatus {
  drives: DriveInfo[];
  /** Where the app's registrations with the drives are read from. */
  appsFile: string;
}

/** A project kept in step with a folder on a drive. */
export interface DriveLink {
  provider: DriveProvider;
  folder: string;
  linkedAt: string;
  syncedAt: string | null;
  /** Why the last pass stopped; null when it went through. */
  error: string | null;
}

export interface SyncReport {
  uploaded: string[];
  downloaded: string[];
  removedHere: string[];
  removedThere: string[];
  copies: string[];
  merged: boolean;
  later: string[];
}

export interface SyncOutcome {
  link: DriveLink;
  report: SyncReport | null;
}

/** A page in the in-app browser, for 자료로 보관. */
export interface PageClip {
  url: string;
  title: string;
  /** Selected text, if any. */
  text: string;
}

/** Word from the in-app browser: a page's address, title, or loading. */
export interface PageEvent {
  label: string;
  url?: string;
  title?: string;
  loading?: boolean;
}

/** Where a browser tab sits in the window, in CSS pixels. */
export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface DriveProject {
  folder: string;
  title: string;
  id: string;
}

/** A change in the project folder the app did not make (crates/core/src/changes.rs). */
export type Change =
  | { kind: 'project' }
  | { kind: 'projectCopy' }
  | { kind: 'doc'; id: string; rev: string | null }
  | { kind: 'card'; id: string }
  | { kind: 'note'; id: string }
  | { kind: 'records'; docId: string }
  | { kind: 'trash' };

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

// 가져오기 (import.rs)

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
}

export interface Imported {
  docs: string[];
  notes: number;
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
  /**
   * `base` is the fingerprint of the text the editor started from; when the
   * text on disk changed since, nothing is saved unless `force`.
   */
  docSave(root: string, docId: string, body: JSONContent, base?: string | null, force?: boolean): Promise<SaveOutcome>;
  /** Keeps a record of `body` as the document's text (before loading another device's). */
  docKeep(root: string, docId: string, body: JSONContent, kind: SnapshotKind): Promise<SnapshotInfo | null>;
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
  /** A copy of a chapter or planning document, to compare with the original. */
  copyLoad(root: string, section: Section, file: string): Promise<DocData>;
  copyResolve(root: string, section: Section, file: string, action: CopyAction): Promise<void>;
  /** Sync folders on this computer, then this computer only. */
  storagePlaces(): Promise<Place[]>;
  /** The sync folder a path is in, if any. */
  storageOf(path: string): Promise<Place | null>;
  /** Moves the project folder into `dest` and opens it there. */
  projectMove(root: string, dest: string): Promise<{ overview: Overview; leftBehind: boolean }>;
  /**
   * Calls `onChange` with changes in the project folder that the app did not
   * make (another device, or by hand). Returns a function that stops it.
   */
  watchProject(root: string, onChange: (changes: Change[]) => void): () => void;
  driveStatus(): Promise<DriveStatus>;
  /** What the files would become; nothing is made. */
  importPreview(paths: string[], opts: ImportOptions): Promise<ImportPreview>;
  /** Makes the chapters picked in the preview. */
  importCommit(root: string, paths: string[], opts: ImportOptions, spec: ImportSpec): Promise<Imported>;
  /** Opens the drive's sign-in page in the browser and waits for the writer. */
  driveConnect(provider: DriveProvider): Promise<DriveAccount>;
  driveCancel(): Promise<void>;
  driveDisconnect(provider: DriveProvider): Promise<void>;
  driveProjects(provider: DriveProvider): Promise<DriveProject[]>;
  /** Brings a project from a drive into a new folder in `dest` and opens it. */
  driveFetch(provider: DriveProvider, folder: string, dest: string): Promise<Overview>;
  projectLinkGet(projectId: string): Promise<DriveLink | null>;
  projectLink(projectId: string, title: string, provider: DriveProvider): Promise<DriveLink>;
  projectUnlink(projectId: string): Promise<void>;
  projectSync(root: string, projectId: string): Promise<SyncOutcome>;
  /** In-app browser (prototype): desktop only. */
  browserOpen(label: string, url: string, at: Bounds): Promise<void>;
  browserBounds(label: string, at: Bounds, visible: boolean): Promise<void>;
  browserNavigate(label: string, url: string): Promise<string>;
  browserStep(label: string, step: 'back' | 'forward' | 'reload'): Promise<void>;
  browserClose(label: string): Promise<void>;
  browserClip(label: string): Promise<PageClip>;
  onBrowserPage(handler: (event: PageEvent) => void): () => void;
  reveal(path: string): Promise<void>;
  pickFolder(title: string, defaultPath?: string): Promise<string | null>;
  /** Files to import (txt, md, docx); empty when the writer cancels. */
  pickFiles(title: string): Promise<string[]>;
  /** `extension` without the dot: txt, docx or hwpx. */
  pickSaveFile(title: string, defaultName: string, extension: string): Promise<string | null>;
  /**
   * Runs `handler` before the window closes (used to finish saving). The
   * window stays open when it resolves to false.
   */
  onCloseRequested(handler: () => Promise<boolean>): void;
  setWindowTitle(title: string): void;
}
