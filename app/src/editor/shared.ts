// A document open in both panes (분할) has one save session, and each edit is
// passed to the other editor so the two stay the same.

import type { Editor, JSONContent } from '@tiptap/core';
import type { Transaction } from '@tiptap/pm/state';
import type { SaveOutcome } from '../api/types';
import { clearConflict } from '../store';
import { noteEdit } from './journal';
import { SaveSession } from './session';

/** Marks a transaction passed on from the other editor. */
const FROM_PEER = 'fromPeer';
/** Marks a transaction that loads the text on disk: nothing to save. */
export const RELOAD = 'wpReload';

interface Shared {
  session: SaveSession;
  editors: Set<Editor>;
  onSaved: (outcome: SaveOutcome) => void;
  /** The text of the last editor to go, for its final save. */
  lastBody: JSONContent | null;
}

const open = new Map<string, Shared>();

/** Puts a whole text in the editor, without an undo step and without passing it on. */
function replaceDoc(editor: Editor, json: JSONContent) {
  const doc = editor.schema.nodeFromJSON(json);
  const tr = editor.state.tr.replaceWith(0, editor.state.doc.content.size, doc.content);
  editor.view.dispatch(tr.setMeta('addToHistory', false).setMeta(FROM_PEER, true));
}

/**
 * Shows the text on disk (another device's) in every editor of a document,
 * without an undo step and without saving it again. The cursor stays near
 * where it was.
 */
export function reloadDoc(docId: string, json: JSONContent, rev: string) {
  const entry = open.get(docId);
  if (!entry) return;
  for (const editor of entry.editors) {
    if (editor.isDestroyed) continue;
    // Replace only the part that differs, so the cursor, the scroll position
    // and everything around stay put.
    const next = editor.schema.nodeFromJSON(json);
    const current = editor.state.doc;
    const start = current.content.findDiffStart(next.content);
    if (start == null) continue;
    let { a: endA, b: endB } = current.content.findDiffEnd(next.content) ?? {
      a: current.content.size,
      b: next.content.size,
    };
    const overlap = start - Math.min(endA, endB);
    if (overlap > 0) {
      endA += overlap;
      endB += overlap;
    }
    const tr = editor.state.tr.replace(start, endA, next.slice(start, endB));
    editor.view.dispatch(tr.setMeta('addToHistory', false).setMeta(FROM_PEER, true).setMeta(RELOAD, true));
  }
  entry.session.loaded(rev);
}

/** The save session of a document open in an editor. */
export function sessionOf(docId: string): SaveSession | null {
  return open.get(docId)?.session ?? null;
}

/** Every editor showing this document now. */
export function editorsOf(docId: string): Editor[] {
  return [...(open.get(docId)?.editors ?? [])].filter((e) => !e.isDestroyed);
}

/** Another editor showing this document right now, if any. */
export function peerOf(docId: string): Editor | null {
  for (const editor of open.get(docId)?.editors ?? []) if (!editor.isDestroyed) return editor;
  return null;
}

/** `rev` is the fingerprint of the text the editor was loaded with. */
export function attach(
  root: string,
  docId: string,
  editor: Editor,
  onSaved: (outcome: SaveOutcome) => void,
  rev: string,
): { session: SaveSession; detach: () => void } {
  let shared = open.get(docId);
  if (!shared) {
    const entry: Shared = {
      session: new SaveSession(
        root,
        docId,
        () => peerOf(docId)?.getJSON() ?? entry.lastBody ?? { type: 'doc', content: [] },
        (outcome) => entry.onSaved(outcome),
        rev,
      ),
      editors: new Set(),
      onSaved,
      lastBody: null,
    };
    open.set(docId, entry);
    shared = entry;
  }
  const entry = shared;
  entry.onSaved = onSaved;

  // Start from the other editor's text: it may hold edits not saved yet.
  const peer = peerOf(docId);
  if (peer && !peer.state.doc.eq(editor.state.doc)) {
    replaceDoc(editor, peer.getJSON());
  }
  entry.editors.add(editor);

  const pass = ({ transaction }: { transaction: Transaction }) => {
    if (!transaction.docChanged || transaction.getMeta(FROM_PEER)) return;
    // The writer's own edit (passed-on and reloaded text is marked FROM_PEER).
    noteEdit(root, docId, transaction, editor.view.composing);
    for (const other of entry.editors) {
      if (other === editor || other.isDestroyed) continue;
      const tr = other.state.tr;
      try {
        for (const s of transaction.steps) tr.step(s);
      } catch {
        // Out of step somehow: copy the whole text instead.
        replaceDoc(other, editor.getJSON());
        continue;
      }
      other.view.dispatch(tr.setMeta(FROM_PEER, true).setMeta('addToHistory', false));
    }
  };
  editor.on('transaction', pass);

  return {
    session: entry.session,
    detach: () => {
      editor.off('transaction', pass);
      entry.lastBody = editor.getJSON();
      entry.editors.delete(editor);
      if (entry.editors.size === 0) {
        open.delete(docId);
        void entry.session.dispose();
        // This device's text is in the records; the banner goes with the editor.
        clearConflict(docId);
      }
    },
  };
}
