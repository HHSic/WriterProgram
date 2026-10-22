// 작품 해석 (crates/core/src/reading.rs): what sentences of a chapter do and
// mean and how they link up, written by an assistant through writer-mcp.

export interface InterpretLink {
  /** Id of the other sentence in the reading; empty for a link into another chapter. */
  to: string;
  kind: string;
  note: string;
  /** The other chapter, for a link into it. */
  doc?: string;
  /** The sentence there, exactly as in its text. */
  quote?: string;
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
  /** When the writer last changed it in the app. */
  edited?: string;
}

/** Where a sentence sits in the chapter now: UTF-16 offsets in its paragraph. */
export interface InterpretPlace {
  id: string;
  block: number;
  start: number;
  end: number;
}

/** A sentence in some chapter, found in its text as it is now. */
export interface InterpretSpot {
  doc: string;
  title: string;
  block: number;
  start: number;
  end: number;
}

/** A link of this chapter's reading into another chapter. */
export interface InterpretRemote {
  /** The sentence here it starts from, and its place among that sentence's links. */
  from: string;
  link: number;
  doc: string;
  title: string;
  quote: string;
  kind: string;
  note: string;
  /** Where the quote is now; null when it is gone from that chapter. */
  spot: InterpretSpot | null;
}

/** A link another chapter's reading makes into this chapter. */
export interface InterpretIncoming {
  doc: string;
  title: string;
  from: string;
  fromQuote: string;
  fromSpot: InterpretSpot | null;
  kind: string;
  note: string;
  /** The sentence here and where it is now (its `id` is empty). */
  quote: string;
  place: InterpretPlace | null;
}

export interface InterpretView {
  reading: Interpretation;
  /** This chapter has a reading of its own (not only links into it). */
  read: boolean;
  places: InterpretPlace[];
  /** Sentences no longer found in the text. */
  gone: string[];
  /** The text changed since it was read. */
  changed: boolean;
  remote: InterpretRemote[];
  incoming: InterpretIncoming[];
}
