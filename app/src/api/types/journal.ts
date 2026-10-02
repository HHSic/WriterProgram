// Creation journal (창작 일지): crates/core/src/journal.rs, app commands/journal.rs.

/** This device's journal settings (kept in the app's settings folder, not the project). */
export interface JournalSettings {
  /** This device's id; its journal file is `.journal/<device>.jsonl`. */
  device: string;
  enabled: boolean;
  /** The writer has seen the first-use notice. */
  noticed: boolean;
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
