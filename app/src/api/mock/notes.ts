// Notes (메모) on a line, a document, a card or the whole project.

import type { JSONContent } from '@tiptap/core';
import type { Backend, Note, TrashItem } from '../types';
import { clone, doc, id, now, project, type MockDoc } from './state';

/** A body without the marks of one note. */
function unmarkMemo(body: JSONContent, noteId: string): JSONContent {
  const strip = (node: JSONContent): JSONContent => ({
    ...node,
    marks: node.marks?.filter((m) => !(m.type === 'memo' && m.attrs?.id === noteId)),
    content: node.content?.map(strip),
  });
  return strip(clone(body));
}

export const noteMethods = {
  async noteList(root) {
    const p = project(root);
    return clone([...p.notes.values()].sort((a, b) => a.created.localeCompare(b.created)));
  },
  async noteCreate(root, spec) {
    const p = project(root);
    const target = spec.anchor === 'project' ? '' : (spec.target ?? '').trim();
    if ((spec.anchor === 'text' || spec.anchor === 'doc') && !p.docs.has(target)) throw '문서를 찾을 수 없음';
    if (spec.anchor === 'card' && !p.cards.has(target)) throw '카드를 찾을 수 없음';
    if (spec.id && p.notes.has(spec.id)) throw '같은 메모가 이미 있음';
    const at = now();
    const note: Note = {
      id: spec.id ?? id(),
      anchor: spec.anchor,
      target,
      quote: [...(spec.quote ?? '')].slice(0, 200).join(''),
      text: spec.text ?? '',
      replies: [],
      tags: [],
      done: false,
      created: at,
      updated: at,
    };
    p.notes.set(note.id, note);
    return clone(note);
  },
  async noteSave(root, note) {
    const p = project(root);
    const current = p.notes.get(note.id);
    if (!current) throw '메모를 찾을 수 없음';
    const tags: string[] = [];
    for (const raw of note.tags) {
      const tag = raw.trim().replace(/^#+/, '').trim();
      if (tag && !tags.includes(tag)) tags.push(tag);
    }
    const saved: Note = {
      ...clone(note),
      anchor: current.anchor,
      target: current.target,
      created: current.created,
      replies: note.replies.filter((r) => r.text.trim()),
      tags,
      updated: now(),
    };
    p.notes.set(saved.id, saved);
    return clone(saved);
  },
  async noteTrash(root, noteId) {
    const p = project(root);
    const n = p.notes.get(noteId);
    if (!n) throw '메모를 찾을 수 없음';
    p.notes.delete(noteId);
    if (n.anchor === 'text' && p.docs.has(n.target)) {
      const d = doc(p, n.target);
      d.body = unmarkMemo(d.body, noteId);
    }
    const first = n.text.split('\n').find((l) => l.trim()) ?? n.quote;
    const item: TrashItem = {
      id: `${Date.now()}-${noteId}`,
      docId: noteId,
      section: 'notes',
      title: [...first].slice(0, 40).join(''),
      deletedAt: now(),
      partId: null,
      index: 0,
      chars: n.text.length,
    };
    const holder: MockDoc & { note?: Note } = {
      meta: { id: noteId, title: item.title, synopsis: '', status: 'draft', target: null, created: n.created },
      body: { type: 'doc' },
      section: 'notes',
      note: n,
    };
    p.trash.unshift({ item, doc: holder });
    return clone(item);
  },
} satisfies Partial<Backend>;
