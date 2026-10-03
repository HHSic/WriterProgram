// Rescue copies (비상 보관, crates/core/src/rescue.rs): writing that kept
// failing to save into the project folder, kept in the app's data folder.

import type { JSONContent } from '@tiptap/core';

export interface RescueFile {
  /** The document id, or `meta-<id>`, `card-<id>`, `note-<id>`. */
  item: string;
  path: string;
  /** When the file was last written (RFC 3339). */
  saved: string;
}

/** What a rescue copy holds: a chapter's text, or plain text for the rest. */
export type RescueContent = { body: JSONContent } | { text: string };
