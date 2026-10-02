// Creation journal (창작 일지): crates/core/src/journal.rs, app commands/journal.rs.

/** This device's journal settings (kept in the app's settings folder, not the project). */
export interface JournalSettings {
  /** This device's id; its journal file is `.journal/<device>.jsonl`. */
  device: string;
  enabled: boolean;
  /** The writer has seen the first-use notice. */
  noticed: boolean;
  /** Daily time stamps (시각 고정) allowed on this device; null until asked. */
  anchor: boolean | null;
}

/** What the editor reports. Counts only, never text. */
export type JournalEvent =
  | { kind: 'session'; doc: string; start: string; end: string; inserted: number; deleted: number }
  | { kind: 'paste'; doc: string; chars: number; outside: boolean };

export interface JournalSummary {
  /** Time of the earliest line on any device. */
  since: string | null;
  saves: number;
  sessions: number;
  pastes: number;
  imports: number;
  /** 교정 주고받기 lines: chapters sent, corrected files taken back, corrections applied. */
  exchanges: number;
  /** Time stamps (one per authority) on all devices. */
  anchors: number;
  /** When the newest time stamp was signed. */
  lastAnchor: string | null;
  /** Devices with a journal in this project. */
  devices: number;
  /** Lines written by this device. */
  thisDevice: number;
}

export type JournalProblem = 'unreadable' | 'broken';

export interface JournalFileCheck {
  device: string;
  lines: number;
  /** First wrong line (from 1) and what is wrong with it. */
  firstBad: [number, JournalProblem] | null;
}

export interface JournalReport {
  files: JournalFileCheck[];
  ok: boolean;
}

/** What a time stamp request came to (시각 고정, commands/anchor.rs). */
export interface AnchorResult {
  state: 'signed' | 'notAllowed' | 'journalOff' | 'noJournal' | 'unchanged' | 'doneToday' | 'offline';
  /** Authorities that signed. */
  signed: string[];
}

/** What the writer picks for a creation proof certificate (writer_core::proof::Options). */
export interface ProofOptions {
  /** Chapters covered; null for the whole work. */
  docs: string[] | null;
  /** First and last day covered, `YYYY-MM-DD`, both included; null for no limit. */
  from: string | null;
  to: string | null;
  /** Chapters whose draft and final text are compared on the page. */
  excerpts: string[];
  /** Times on the page as dates only. */
  datesOnly: boolean;
}

/** Where a certificate was written. */
export interface ProofWritten {
  folder: string;
  html: string;
  bundle: string;
  summary: string;
  /** Its own check found nothing wrong. */
  ok: boolean;
}
