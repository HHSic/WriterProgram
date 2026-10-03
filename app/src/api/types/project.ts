// A project: its info, parts (부), overview, recent list and what creating or changing it takes.

import type { CardSummary, CardType } from './cards';
import type { CopyInfo } from './devices';
import type { CountRule, Counts, DocSummary } from './docs';
import type { ManuscriptFormat } from './format';

export type ProjectKind = 'webnovel' | 'print';

export interface Goal {
  perDoc: number | null;
  countSpaces: boolean;
  daily: number | null;
}

/** 연재 플랫폼 and how it counts (crates/core/src/project/mod.rs `Platform`). */
export interface Platform {
  /** An id from lib/platforms.ts, or `custom`. */
  id: string;
  rule: CountRule;
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
  /** 연재 플랫폼 whose counting chapter counts and goals follow (web novels). */
  platform: Platform | null;
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
  platform?: Platform | null;
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
  /** null takes the platform away. */
  platform?: Platform | null;
}
