// The Backend interface: every call the screens make to the Rust side (or the browser stand-in).

import type { JSONContent } from '@tiptap/core';
import type { Bounds, PageClip, PageEvent } from './browser';
import type { Appearance, Card, CardSummary, CardType } from './cards';
import type {
  Change,
  CopyAction,
  DriveAccount,
  DriveLink,
  DriveProject,
  DriveProvider,
  DriveStatus,
  Place,
  SyncOutcome,
} from './devices';
import type { DocData, DocMeta, MetaPatch, NewDoc, SaveOutcome, Section, SnapshotInfo, SnapshotKind, TrashItem } from './docs';
import type { Applied, Decisions, Exchange, ExchangeInfo, Review } from './exchange';
import type { FormatCatalog, ManuscriptFormat, UserPreset } from './format';
import type { ImportOptions, ImportPreview, ImportSpec, Imported } from './import';
import type { JournalEvent, JournalReport, JournalSettings, JournalSummary } from './journal';
import type { NewNote, Note } from './notes';
import type { NewProject, Overview, ProjectInfo, ProjectPatch, ProjectSizes, RecentItem } from './project';
import type { DocOptions, ExportItem, FileKind, ReplaceOutcome, SearchQuery, SearchResult, TextOptions } from './search-export';

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
  /** 오래된 자동 기록 정리: removes automatic records older than two weeks; returns the bytes freed. */
  recordsTidy(root: string): Promise<number>;
  /** 이 작품 크기, and the free room on the project's disk. */
  projectSize(root: string): Promise<ProjectSizes>;
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
  /** "편집자에게 보내기": exports like `exportFile` and keeps what was sent. */
  exchangeSend(
    root: string,
    items: ExportItem[],
    opts: DocOptions,
    format: ManuscriptFormat,
    kind: FileKind,
    dest: string,
    perDoc: boolean,
  ): Promise<Exchange>;
  /** Chapters sent to editors, newest first. */
  exchangeList(root: string): Promise<ExchangeInfo[]>;
  /** Reads a corrected file (hwpx, docx) and compares it with what was sent. */
  exchangeRead(root: string, exchangeId: string, path: string): Promise<Review>;
  /** The last corrected file read, with the decisions so far. */
  exchangeReview(root: string, exchangeId: string): Promise<Review | null>;
  /** Accepts or rejects corrections; keeps a record of each chapter first. */
  exchangeApply(root: string, exchangeId: string, decisions: Decisions): Promise<Applied>;
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
  /** This device's creation journal settings (its id is made the first time). */
  journalSettings(): Promise<JournalSettings>;
  /** Turns the journal on or off on this device, or notes the first-use notice as seen. */
  journalSet(patch: { enabled?: boolean; noticed?: boolean }): Promise<JournalSettings>;
  /** A writing session or paste for the journal; ignored while it is off. */
  journalEvent(root: string, event: JournalEvent): Promise<void>;
  journalSummary(root: string): Promise<JournalSummary>;
  /** Checks that no journal line was changed or removed. */
  journalVerify(root: string): Promise<JournalReport>;
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
  /** Files to import (txt, md, docx, hwpx); empty when the writer cancels. */
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
