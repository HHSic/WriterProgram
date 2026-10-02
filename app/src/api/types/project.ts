// A project: its info, parts (부), overview, recent list and what creating or changing it takes.

import type { CardSummary, CardType } from './cards';
import type { CopyInfo } from './devices';
import type { Counts, DocSummary } from './docs';
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
}

export interface ProjectPatch {
  title?: string;
  kind?: ProjectKind;
  penName?: string;
  goal?: Goal;
  sceneBreak?: string;
  manuscriptFormat?: ManuscriptFormat;
}
