// Data exchanged with the Rust side (app/src-tauri/src/commands.rs and
// crates/core). Field names follow the serde camelCase renames there.

import type { JSONContent } from '@tiptap/core';

export type ProjectKind = 'webnovel' | 'print';
export type Section = 'manuscript' | 'planning';
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

export interface ProjectInfo {
  id: string;
  title: string;
  kind: ProjectKind;
  penName: string;
  created: string;
  goal: Goal;
  sceneBreak: string;
  preset: string;
}

export interface DocSummary {
  id: string;
  title: string;
  synopsis: string;
  status: DocStatus;
  target: number | null;
  counts: Counts;
}

export interface PartView {
  id: string;
  title: string;
  docs: DocSummary[];
}

export interface Overview {
  root: string;
  project: ProjectInfo;
  parts: PartView[];
  planning: DocSummary[];
  trashCount: number;
  total: Counts;
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
}

export interface TextOptions {
  includeTitles: boolean;
  blankLineBetween: boolean;
  sceneBreak: string;
}

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
  reveal(path: string): Promise<void>;
  pickFolder(title: string, defaultPath?: string): Promise<string | null>;
  pickSaveFile(title: string, defaultName: string): Promise<string | null>;
  /**
   * Runs `handler` before the window closes (used to finish saving). The
   * window stays open when it resolves to false.
   */
  onCloseRequested(handler: () => Promise<boolean>): void;
  setWindowTitle(title: string): void;
}
