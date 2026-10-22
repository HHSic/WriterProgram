// 작품 해석 (crates/core/src/reading.rs): what sentences of a chapter do and
// mean and how they link up, written by an assistant through writer-mcp.

export interface InterpretLink {
  /** Id of the other sentence in the reading. */
  to: string;
  kind: string;
  note: string;
}

export interface InterpretSentence {
  id: string;
  block: number;
  quote: string;
  roles: string[];
  meaning: string;
  intent: string;
  links: InterpretLink[];
  /** Why the sentence seems to do little, for the writer to judge. */
  unclear?: string;
}

export interface InterpretScene {
  /** First paragraph of the scene. */
  block: number;
  title: string;
  purpose: string;
}

export interface Interpretation {
  doc: string;
  rev: string;
  made: string;
  by: string;
  summary: string;
  scenes: InterpretScene[];
  sentences: InterpretSentence[];
}

/** Where a sentence sits in the chapter now: UTF-16 offsets in its paragraph. */
export interface InterpretPlace {
  id: string;
  block: number;
  start: number;
  end: number;
}

export interface InterpretView {
  reading: Interpretation;
  places: InterpretPlace[];
  /** Sentences no longer found in the text. */
  gone: string[];
  /** The text changed since it was read. */
  changed: boolean;
}
