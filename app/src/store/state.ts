// App state: what the screens read, and the types around it.

import type { Editor } from '@tiptap/core';
import { create } from 'zustand';
import type { CopyInfo, Counts, DriveLink, FormatCatalog, Note, Overview, SnapshotInfo } from '../api/types';
import type { Pane, SplitDir, Target } from '../lib/tabs';
import { type ViewSettings, loadView } from '../lib/view';


export type SaveState = { state: 'saved' | 'saving' | 'error'; error?: string };
export type RightTab = 'outline' | 'notes' | 'cast' | 'find' | 'records';
export type FindScope = 'doc' | 'part' | 'all';

/** Asks the find panel to open with a search (from the sidebar or Ctrl+F). */
export interface FindRequest {
  text?: string;
  scope?: FindScope;
  focus: 'find' | 'replace';
  nonce: number;
}

/** A place to move to once its document has opened. */
export interface Jump {
  docId: string;
  block: number;
  start: number;
  end: number;
}

export type SettingsTab = 'basic' | 'goal' | 'format';

export type Dialog =
  | { kind: 'newProject' }
  | { kind: 'project'; tab?: SettingsTab }
  | { kind: 'trash' }
  | { kind: 'export' }
  /** 가져오기: txt, md, docx, hwpx into chapters, into `partId` when given. */
  | { kind: 'import'; partId?: string }
  | { kind: 'view' }
  | { kind: 'symbols' }
  /** 둘 다 보기: this device's text next to another device's, or a document next to its copy. */
  | { kind: 'compare'; docId: string; copy?: CopyInfo }
  /** Every copy left by sync programs (다른 기기 사본). */
  | { kind: 'copies' }
  /** 기기 간 맞추기: connecting drives. */
  | { kind: 'drives' }
  /** Bringing a project from a drive to this device. */
  | { kind: 'driveImport' }
  /** Moving the project folder (저장 위치 옮기기). */
  | { kind: 'move' }
  | {
      kind: 'prompt';
      title: string;
      label: string;
      value: string;
      confirm: string;
      inputMode?: 'text' | 'numeric';
      onSubmit: (value: string) => Promise<void> | void;
    }
  | {
      kind: 'confirm';
      title: string;
      message: string;
      confirm: string;
      danger?: boolean;
      onConfirm: () => Promise<void> | void;
    };

/** A document whose text another device changed while it was being edited here. */
export interface DocConflict {
  /** This device's text, kept as a record when its save was refused. */
  record: SnapshotInfo | null;
  at: string;
}

export interface Toast {
  text: string;
  tone?: 'error';
  action?: { label: string; run: () => void };
}

interface AppState {
  overview: Overview | null;
  /** The middle column: one pane of tabs, or two side by side (분할). */
  panes: Pane[];
  /** The pane being worked in; the right column and status bar follow it. */
  focus: number;
  split: SplitDir;
  /** What the focused pane's open tab shows, and the same split by kind. */
  activeTarget: Target | null;
  activeDocId: string | null;
  activeCardId: string | null;
  /** Editor of the focused pane's open tab, when that is a document. */
  editor: Editor | null;
  save: SaveState;
  liveCounts: Counts | null;
  selection: { withSpaces: number; withoutSpaces: number } | null;
  rightTab: RightTab;
  rightOpen: boolean;
  view: ViewSettings;
  dialog: Dialog | null;
  toast: Toast | null;
  /** Bumped to reload the open document from disk (after going back to a record). */
  docVersion: number;
  /** Bumped when the records of the open document change. */
  recordsVersion: number;
  /** Manuscript formats, fonts and paper sizes offered in settings. */
  catalog: FormatCatalog | null;
  findRequest: FindRequest | null;
  pendingJump: Jump | null;
  /** Card shown over the right panel (미리보기). */
  previewCardId: string | null;
  /** 부 picked in the tree: where 새 회차 goes, until another chapter opens. */
  selectedPartId: string | null;
  /** For each card, in how many chapters it appears. */
  cardCounts: Record<string, number>;
  /** Every note (메모) in the project. */
  notes: Note[];
  /** The note being looked at: open in the 메모 tab, its text marked. */
  focusNoteId: string | null;
  /** A note whose text to select once its document is open. */
  pendingNote: string | null;
  /** Documents another device changed while they were being edited here. */
  conflicts: Record<string, DocConflict>;
  /** Bumped per card when another device changed it (open card editors reload). */
  cardReloads: Record<string, number>;
  /** Shown over everything while the app is busy with the whole project. */
  busy: string | null;
  /** The drive folder the open project is kept in step with, if any. */
  link: DriveLink | null;
  /** A pass with the drive is running. */
  syncing: boolean;
  /** Bumped when a browser tab's page or title changes (tab labels follow). */
  webVersion: number;
}

export const useApp = create<AppState>(() => ({
  overview: null,
  panes: [],
  focus: 0,
  split: 'row',
  activeTarget: null,
  activeDocId: null,
  activeCardId: null,
  editor: null,
  save: { state: 'saved' },
  liveCounts: null,
  selection: null,
  rightTab: 'outline',
  rightOpen: true,
  view: loadView(),
  dialog: null,
  toast: null,
  docVersion: 0,
  recordsVersion: 0,
  catalog: null,
  findRequest: null,
  pendingJump: null,
  previewCardId: null,
  selectedPartId: null,
  cardCounts: {},
  notes: [],
  focusNoteId: null,
  pendingNote: null,
  conflicts: {},
  cardReloads: {},
  busy: null,
  link: null,
  syncing: false,
  webVersion: 0,
}));

export const set = useApp.setState;
export const get = useApp.getState;

/** The open project's folder; throws when none is open. */
export function root(): string {
  const ov = get().overview;
  if (!ov) throw new Error('no project open');
  return ov.root;
}
