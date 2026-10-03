// A project: its info, parts (부), overview, recent list and what creating or changing it takes.

import type { CardSummary, CardType } from './cards';
import type { CopyInfo } from './devices';
import type { Counts, DocSummary, Section } from './docs';
import type { ManuscriptFormat } from './format';

export type ProjectKind = 'webnovel' | 'print';

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
  manuscriptFormat: ManuscriptFormat;
  /** 창작 과정 보관: each chapter's last record of a day is never cleared. */
  keepDaily: boolean;
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
  totalPages: number | null;
  cardTypes: CardType[];
  cards: CardSummary[];
  /** Copies left by sync programs, waiting for the writer to pick. */
  copies: CopyInfo[];
  /** Listed documents whose file is there but cannot be read (고쳐 열기). */
  unreadable: UnreadableDoc[];
}

/** A document whose file is there but cannot be read (writer_core::project::UnreadableDoc). */
export interface UnreadableDoc {
  id: string;
  section: Section;
  /** The part it is listed in; null for planning documents. */
  part: string | null;
  /** Its place among the rows shown in that part (or the planning list). */
  index: number;
  /** The title as well as it can be read; empty when unknown. */
  titleGuess: string;
  /** Why, in the writer's words. */
  reason: string;
  /** 고쳐 열기 can mend it (not when another program holds the file). */
  repairable: boolean;
}

/** One way back for a damaged project.json: a backup's day or a copy's file. */
export interface RecoveryChoice {
  name: string;
  /** When the copy was last written; null for a backup. */
  modified: string | null;
  parts: number;
  chapters: number;
}

/** What can bring a damaged project.json back (writer_core::project::Recovery). */
export interface Recovery {
  /** A copy another device left. */
  copy: RecoveryChoice | null;
  /** The newest daily backup. */
  backup: RecoveryChoice | null;
  /** Chapter files in the folder: what rebuilding would list. */
  chapterFiles: number;
}

export type RecoverWay = 'copy' | 'backup' | 'rebuild';

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

/** 이 작품 크기, in bytes (crates/core/src/project/size.rs). */
export interface ProjectSizes {
  /** project.json, chapters, planning, cards and notes. */
  writing: number;
  /** Records (.snapshots). */
  records: number;
  /** Of the records, the ones kept only as each chapter's last state of a day (창작 과정 보관). */
  daily: number;
  trash: number;
  /** The writing journal; 0 when there is none. */
  journal: number;
  /** What was sent to editors and the corrected files (교정본 주고받기). */
  exchanges: number;
  /** What tidying old automatic records would free. */
  tidyFrees: number;
  /** Records have grown much bigger than the writing: suggest tidying. */
  suggestTidy: boolean;
  /** Free room on the disk the project is on; null when unknown. */
  diskFree: number | null;
}

export interface ProjectPatch {
  title?: string;
  kind?: ProjectKind;
  penName?: string;
  goal?: Goal;
  sceneBreak?: string;
  manuscriptFormat?: ManuscriptFormat;
  keepDaily?: boolean;
}
