// Notes (메모) and what they hang on.

/** What a note (메모) hangs on: a stretch of text, a document, a card or the project. */
export type NoteAnchor = 'text' | 'doc' | 'card' | 'project';

export interface NoteReply {
  at: string;
  text: string;
}

export interface Note {
  id: string;
  anchor: NoteAnchor;
  /** Document (text, doc) or card id; empty for the project. */
  target: string;
  /** The marked text when last saved. */
  quote: string;
  text: string;
  replies: NoteReply[];
  tags: string[];
  done: boolean;
  created: string;
  updated: string;
}

export interface NewNote {
  id?: string;
  anchor: NoteAnchor;
  target?: string;
  quote?: string;
  text?: string;
}
