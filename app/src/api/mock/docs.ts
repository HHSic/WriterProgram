// Chapters and planning documents: adding, moving, loading, saving, records (기록) and the trash.

import type { Backend, Card, Note, TrashItem } from '../types';
import { noteMockSave } from './journal';
import { mockPages } from './presets';
import { clone, counts, doc, newDoc, now, project, record, revOf, wait, type MockDoc } from './state';

/** Bytes of a value as the files would hold it, roughly. */
export function bytesOf(value: unknown): number {
  return new TextEncoder().encode(JSON.stringify(value)).length;
}

/** Automatic records tidying removes: older than two weeks, each document keeping its newest. */
export function oldAutoRecords(p: ReturnType<typeof project>) {
  const out: { docId: string; id: string; size: number }[] = [];
  for (const [docId, list] of p.records) {
    let newest = true;
    for (const r of list) {
      if (r.info.kind !== 'auto') continue;
      if (!newest && Date.now() - Date.parse(r.info.at) > 14 * 86_400_000) {
        out.push({ docId, id: r.info.id, size: bytesOf(r.body) });
      }
      newest = false;
    }
  }
  return out;
}

export const docMethods = {
  async recordsTidy(root) {
    const p = project(root);
    let freed = 0;
    for (const old of oldAutoRecords(p)) {
      p.records.set(
        old.docId,
        (p.records.get(old.docId) ?? []).filter((r) => r.info.id !== old.id),
      );
      freed += old.size;
    }
    return freed;
  },
  async docAdd(root, spec) {
    const p = project(root);
    const section = spec.section ?? 'manuscript';
    const d = newDoc(p, section, spec.title?.trim() ?? '');
    if (section === 'planning') {
      const at = spec.after ? p.planning.indexOf(spec.after) : -1;
      p.planning.splice(at >= 0 ? at + 1 : p.planning.length, 0, d.meta.id);
    } else {
      const withAfter = spec.after ? p.parts.find((x) => x.docs.includes(spec.after!)) : undefined;
      if (withAfter) withAfter.docs.splice(withAfter.docs.indexOf(spec.after!) + 1, 0, d.meta.id);
      else (p.parts.find((x) => x.id === spec.partId) ?? p.parts[p.parts.length - 1]).docs.push(d.meta.id);
    }
    return d.meta.id;
  },
  async docMove(root, docId, partId, index) {
    const p = project(root);
    const fromPart = p.parts.find((x) => x.docs.includes(docId));
    if (fromPart && partId) {
      const to = p.parts.find((x) => x.id === partId);
      if (!to) throw '부를 찾을 수 없음';
      fromPart.docs = fromPart.docs.filter((x) => x !== docId);
      to.docs.splice(Math.min(index, to.docs.length), 0, docId);
    } else if (!fromPart && !partId) {
      p.planning = p.planning.filter((x) => x !== docId);
      p.planning.splice(Math.min(index, p.planning.length), 0, docId);
    } else {
      throw '원고와 기획 문서 사이에서는 옮길 수 없음';
    }
  },
  async docTrash(root, docId) {
    const p = project(root);
    const d = doc(p, docId);
    const part = p.parts.find((x) => x.docs.includes(docId));
    const index = part ? part.docs.indexOf(docId) : p.planning.indexOf(docId);
    if (part) part.docs = part.docs.filter((x) => x !== docId);
    else p.planning = p.planning.filter((x) => x !== docId);
    p.docs.delete(docId);
    const item: TrashItem = {
      id: `${Date.now()}-${docId}`,
      docId,
      section: d.section,
      title: d.meta.title,
      deletedAt: now(),
      partId: part?.id ?? null,
      index,
      chars: counts(d.body).withSpaces,
    };
    p.trash.unshift({ item, doc: d });
    return clone(item);
  },
  async docMendPreview(root, docId) {
    await wait();
    const d = doc(project(root), docId);
    if (!d.broken) throw '이미 열 수 있는 회차입니다';
    const text = (d.body.content ?? [])
      .map((n) => (n.content ?? []).map((i) => i.text ?? '').join(''))
      .join('\n');
    return {
      id: docId,
      section: d.section,
      title: d.meta.title,
      reason: d.broken,
      encoding: 'euc-kr',
      newFront: false,
      text: text.slice(0, 1500),
      chars: counts(d.body).withSpaces,
    };
  },
  async docMend(root, docId) {
    await wait();
    const d = doc(project(root), docId);
    if (!d.broken) throw '이미 열 수 있는 회차입니다';
    delete d.broken;
    d.modified = now();
    const stamp = new Date().toISOString().replace(/[-:]/g, '').replace('T', '-').replace('.', '-').slice(0, 19);
    return { id: docId, kept: `${docId}.md.broken-${stamp}` };
  },
  async docLoad(root, docId) {
    await wait();
    const d = doc(project(root), docId);
    if (d.broken) throw `파일을 읽을 수 없음 · ${docId}.md (${d.broken})`;
    return { meta: clone(d.meta), body: clone(d.body), counts: counts(d.body), rev: revOf(d.body) };
  },
  async docSave(root, docId, b, base, force) {
    await wait();
    const p = project(root);
    const d = doc(p, docId);
    const c = counts(b);
    const pages = d.section === 'manuscript' ? mockPages(c.withSpaces, p.info.manuscriptFormat) : null;
    const disk = revOf(d.body);
    if (revOf(b) === disk) return { counts: c, pages, snapshot: null, rev: disk, conflict: false };
    const elsewhere = !!base && base !== disk;
    if (elsewhere && !force) {
      const snapshot = record(p, { ...d, body: clone(b) }, 'this-device', '');
      return { counts: c, pages, snapshot, rev: disk, conflict: true };
    }
    // A big deletion keeps the text before it, like doc.rs save_body.
    const before = counts(d.body).withSpaces;
    const removed = before - c.withSpaces;
    const shrinks = removed >= 500 || (removed >= 200 && removed * 100 >= before * 20);
    const newest = p.records.get(docId)?.[0];
    const snapshot = elsewhere
      ? record(p, d, 'other-device', '')
      : shrinks && !(newest && revOf(newest.body) === disk)
        ? record(p, d, 'before-shrink', '')
        : null;
    d.body = clone(b);
    d.modified = now();
    noteMockSave(root);
    return { counts: c, pages, snapshot, rev: revOf(b), conflict: false };
  },
  async docKeep(root, docId, b, kind) {
    const p = project(root);
    const d = doc(p, docId);
    const newest = p.records.get(docId)?.[0];
    if (newest && revOf(newest.body) === revOf(b)) return null;
    return record(p, { ...d, body: clone(b) }, kind, '');
  },
  async docUpdateMeta(root, docId, patch) {
    const d = doc(project(root), docId);
    if (patch.title !== undefined) d.meta.title = patch.title.trim();
    if (patch.synopsis !== undefined) d.meta.synopsis = patch.synopsis.trim();
    if (patch.status !== undefined) d.meta.status = patch.status;
    if (patch.target !== undefined) d.meta.target = patch.target && patch.target > 0 ? patch.target : null;
    d.modified = now();
    return clone(d.meta);
  },
  async snapshotList(root, docId) {
    await wait();
    return (project(root).records.get(docId) ?? []).map((r) => clone(r.info));
  },
  async snapshotCreate(root, docId, name) {
    const p = project(root);
    return record(p, doc(p, docId), 'manual', name);
  },
  async snapshotLoad(root, docId, snapshotId) {
    const p = project(root);
    const r = (p.records.get(docId) ?? []).find((x) => x.info.id === snapshotId);
    if (!r) throw '기록을 찾을 수 없음';
    return { meta: clone(doc(p, docId).meta), body: clone(r.body), counts: r.info.counts, rev: revOf(r.body) };
  },
  async snapshotRestore(root, docId, snapshotId) {
    const p = project(root);
    const d = doc(p, docId);
    const r = (p.records.get(docId) ?? []).find((x) => x.info.id === snapshotId);
    if (!r) throw '기록을 찾을 수 없음';
    const before = record(p, d, 'before-restore', '');
    d.body = clone(r.body);
    return before;
  },
  async trashList(root) {
    return project(root).trash.map((t) => clone(t.item));
  },
  async trashRestore(root, trashId) {
    const p = project(root);
    const t = p.trash.find((x) => x.item.id === trashId);
    if (!t) throw '휴지통 항목을 찾을 수 없음';
    if (t.item.section === 'cards') {
      const card = (t.doc as MockDoc & { card?: Card }).card;
      if (card) p.cards.set(card.id, card);
      p.trash = p.trash.filter((x) => x !== t);
      return;
    }
    if (t.item.section === 'notes') {
      const note = (t.doc as MockDoc & { note?: Note }).note;
      if (note) p.notes.set(note.id, note);
      p.trash = p.trash.filter((x) => x !== t);
      return;
    }
    p.docs.set(t.item.docId, t.doc);
    if (t.item.section === 'planning') p.planning.splice(t.item.index, 0, t.item.docId);
    else (p.parts.find((x) => x.id === t.item.partId) ?? p.parts[p.parts.length - 1]).docs.splice(t.item.index, 0, t.item.docId);
    p.trash = p.trash.filter((x) => x !== t);
  },
  async trashDelete(root, trashId) {
    const p = project(root);
    p.trash = p.trash.filter((x) => x.item.id !== trashId);
  },
} satisfies Partial<Backend>;
