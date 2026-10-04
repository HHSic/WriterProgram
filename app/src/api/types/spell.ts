// 맞춤법 검사 (crates/core/src/spell, app/src-tauri/src/commands/spell.rs).

import type { SearchMatch } from './search-export';

export interface SpellDict {
  version: string;
  /** Where the dictionary and its license files are on this device. */
  folder: string;
  source: string;
  /** Source and license, as shown in 보기 설정. */
  notice: string;
}

export interface SpellStatus {
  /** The dictionary on this device; null until the writer fetches it. */
  installed: SpellDict | null;
  version: string;
  /** Size of the download, bytes. */
  downloadBytes: number;
  /** Size once unpacked, bytes. */
  installedBytes: number;
  source: string;
  notice: string;
}

/** A word the dictionary does not know, in UTF-16 units of the paragraph. */
export interface SpellIssue {
  start: number;
  end: number;
  word: string;
}

export interface SpellWorkCheck {
  docs: { docId: string; locked: boolean; issues: SearchMatch[] }[];
  total: number;
  truncated: boolean;
  skipped: number;
}

/** 사전에 넣은 낱말: this work's list (in the project folder) and this device's. */
export interface SpellWords {
  project: string[];
  device: string[];
}

export type SpellPlace = 'project' | 'device';
