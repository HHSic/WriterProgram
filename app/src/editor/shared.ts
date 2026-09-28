// A document open in both panes (분할) has one save session, and each edit is
// passed to the other editor so the two stay the same.

import type { Editor, JSONContent } from '@tiptap/core';
import type { Transaction } from '@tiptap/pm/state';
import type { SaveOutcome } from '../api/types';
import { SaveSession } from './session';

/** Marks a transaction passed on from the other editor. */
const FROM_PEER = 'fromPeer';

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

/** Every editor showing this document now. */
export function editorsOf(docId: string): Editor[] {
  return [...(open.get(docId)?.editors ?? [])].filter((e) => !e.isDestroyed);
}

/** Another editor showing this document right now, if any. */
export function peerOf(docId: string): Editor | null {
  for (const editor of open.get(docId)?.editors ?? []) if (!editor.isDestroyed) return editor;
  return null;
}

export function attach(
  root: string,
  docId: string,
  editor: Editor,
  onSaved: (outcome: SaveOutcome) => void,
): { session: SaveSession; detach: () => void } {
  let shared = open.get(docId);
  if (!shared) {
    const entry: Shared = {
      session: new SaveSession(
        root,
        docId,
        () => peerOf(docId)?.getJSON() ?? entry.lastBody ?? { type: 'doc', content: [] },
        (outcome) => entry.onSaved(outcome),
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
      }
    },
  };
}
