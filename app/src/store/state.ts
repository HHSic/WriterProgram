// App state: what the screens read, and the types around it.

import type { Editor } from '@tiptap/core';
import { create } from 'zustand';
import type { AiSettings, AiTask, CopyInfo, Counts, DriveLink, FormatCatalog, Note, Overview, Recovery, RescueFile, SnapshotInfo } from '../api/types';
import type { Pane, SplitDir, Target } from '../lib/tabs';
import { type ViewSettings, loadView } from '../lib/view';


/** How saving one target (a chapter's text, its title, a card, a note) is going (store/saves.ts). */
export interface SaveState {
  state: 'saved' | 'saving' | 'error';
  /** Why the last save failed, in the writer's words. */
  error?: string;
  /** When saves started failing (ms since 1970). */
  since?: number;
  /** When the next try runs (ms since 1970). */
  retryAt?: number;
  /** Where the unsaved text was kept when saves kept failing (비상 보관). */
  rescued?: string;
}
export type RightTab = 'outline' | 'notes' | 'cast' | 'find' | 'records' | 'ai';
export type FindScope = 'doc' | 'part' | 'all';

/** Asks the find panel to open with a search (from the sidebar or Ctrl+F). */
export interface FindRequest {
  text?: string;
  scope?: FindScope;
  focus: 'find' | 'replace';
  nonce: number;
}

/** Asks the AI tab to get a request ready (from a chapter's menu). */
export interface AiRequest {
  task: AiTask;
  docIds: string[];
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
  /** 내보내기; `toEditor`: 편집자에게 보내기 (what is sent is kept), with `docIds` picked to start. */
  | { kind: 'export'; toEditor?: boolean; docIds?: string[] }
  /** 교정본 주고받기: chapters sent to editors and corrected files taken back. */
  | { kind: 'exchanges' }
  /**
   * 가져오기: txt, md, docx, hwpx into chapters, into `partId` when given.
   * With `newProject` (시작 화면 "원고 가져오기") the chapters go into a new project.
   */
  | { kind: 'import'; partId?: string; newProject?: boolean }
  | { kind: 'view' }
  | { kind: 'symbols' }
  /** 단축키: every keyboard shortcut (lib/shortcuts.ts). */
  | { kind: 'shortcuts' }
  /**
   * 둘 다 보기: this device's text next to another device's, a document next
   * to its copy, or a chapter next to its rescue copy (비상 보관).
   */
  | { kind: 'compare'; docId: string; copy?: CopyInfo; rescue?: RescueFile }
  /** Every copy left by sync programs (다른 기기 사본). */
  | { kind: 'copies' }
  /** 기기 간 맞추기: connecting drives. */
  | { kind: 'drives' }
  /** Bringing a project from a drive to this device. */
  | { kind: 'driveImport' }
  /** Moving the project folder (저장 위치 옮기기). */
  | { kind: 'move' }
  /** AI 연결 (내 API 키): this device's AI settings. */
  | { kind: 'ai' }
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
      /** The other button's label (취소) and what it does besides closing. */
      cancel?: string;
      onCancel?: () => void;
    }
  /** 창작 과정 증명서 만들기. */
  | { kind: 'proof' }
  /** Many files removed at once on one side: remove them on the other too? (`heldRemovals`) */
  | { kind: 'removals' }
  /** 고쳐 열기: a document whose file is there but cannot be read. */
  | { kind: 'mend'; docId: string }
  /** The project's project.json is damaged: the ways to bring it back. */
  | { kind: 'recover'; path: string; recovery: Recovery };

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
  /** Save state per target: `doc:<id>`, `meta:<id>`, `card:<id>`, `note:<id>` (store/saves.ts). */
  saves: Record<string, SaveState>;
  /** The window is closing with writing not saved: the writer picks what to do. */
  closeAsk: { resolve: (close: boolean) => void } | null;
  liveCounts: Counts | null;
  selection: { withSpaces: number; withoutSpaces: number } | null;
  rightTab: RightTab;
  rightOpen: boolean;
  /** 집중 모드: the page alone, without the side columns, tabs and bars. */
  focusMode: boolean;
  view: ViewSettings;
  dialog: Dialog | null;
  toast: Toast | null;
  /** Bumped to reload the open document from disk (after going back to a record). */
  docVersion: number;
  /** Bumped when the records of the open document change. */
  recordsVersion: number;
  /** Bumped when chapters are sent, a corrected file is read or corrections are applied. */
  exchangesVersion: number;
  /** The corrected file's name for each review tab (교정본 검토), once loaded. */
  reviewFiles: Record<string, string>;
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
  /** Removals the last pass held back because too many went at once, until the writer answers. */
  heldRemovals: HeldRemovals | null;
  /** Bumped when a browser tab's page or title changes (tab labels follow). */
  webVersion: number;
  /** 소리 내어 읽기 going on in `editor` (or, `noVoice`, asked for there with
   * no Korean voice installed). */
  reading: Reading | null;
  /** This device's AI settings; null until read. */
  ai: AiSettings | null;
  aiRequest: AiRequest | null;
}

/** Paths a pass did not remove because too many went at once (crates/sync engine.rs). */
export interface HeldRemovals {
  /** Gone here; to remove on the drive. */
  there: string[];
  /** Gone on the drive; to remove here. */
  here: string[];
}

export interface Reading {
  status: 'reading' | 'paused' | 'noVoice';
  editor: Editor;
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
  saves: {},
  closeAsk: null,
  liveCounts: null,
  selection: null,
  rightTab: 'outline',
  rightOpen: true,
  focusMode: false,
  view: loadView(),
  dialog: null,
  toast: null,
  docVersion: 0,
  recordsVersion: 0,
  exchangesVersion: 0,
  reviewFiles: {},
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
  heldRemovals: null,
  webVersion: 0,
  reading: null,
  ai: null,
  aiRequest: null,
}));

export const set = useApp.setState;
export const get = useApp.getState;

/** The open project's folder; throws when none is open. */
export function root(): string {
  const ov = get().overview;
  if (!ov) throw new Error('no project open');
  return ov.root;
}
