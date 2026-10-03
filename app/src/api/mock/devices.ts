// Copies left by sync programs, storage places, moving a project, and watching it for changes.

import type { Backend, Change, Place, TrashItem } from '../types';
import { body, clone, counts, doc, forgetRecent, id, newDoc, now, overview, project, projects, record, revOf, touchRecent, wait, type MockDoc } from './state';

/** The screen's handler for changes "from another device", while a project is watched. */
let watcher: { root: string; onChange: (changes: Change[]) => void } | null = null;

const PLACES: Place[] = [
  { service: 'onedrive', label: 'OneDrive', root: 'C:\\Users\\작가\\OneDrive', suggested: 'C:\\Users\\작가\\OneDrive\\WriterProgram' },
  { service: 'googledrive', label: 'Google Drive', root: 'G:\\내 드라이브', suggested: 'G:\\내 드라이브\\WriterProgram' },
  { service: 'local', label: '이 PC', root: 'C:\\Users\\작가\\Documents', suggested: 'C:\\Users\\작가\\Documents\\WriterProgram' },
];

export const deviceMethods = {
  async copyLoad(root, _section, file) {
    const p = project(root);
    const c = p.copies.find((x) => x.info.file === file);
    if (!c?.doc) throw '사본을 찾을 수 없음';
    return { meta: clone(c.doc.meta), body: clone(c.doc.body), counts: counts(c.doc.body), rev: revOf(c.doc.body) };
  },
  async copyResolve(root, _section, file, action) {
    const p = project(root);
    const c = p.copies.find((x) => x.info.file === file);
    if (!c) throw '사본을 찾을 수 없음';
    p.copies = p.copies.filter((x) => x !== c);
    const of = c.info.of;
    if (action === 'discard') {
      const item: TrashItem = {
        id: `${Date.now()}-${of}-copy`,
        docId: of,
        section: c.info.section,
        title: c.info.title,
        deletedAt: now(),
        partId: null,
        index: 0,
        chars: c.info.chars,
        file,
      };
      p.trash.unshift({ item, doc: c.doc ?? ({ card: c.card } as unknown as MockDoc) });
      return;
    }
    if (c.doc) {
      const d = doc(p, of);
      if (action === 'take') {
        record(p, d, 'before-copy', '');
        d.body = clone(c.doc.body);
        d.meta.title = c.doc.meta.title;
        d.modified = now();
      } else {
        const copy = newDoc(p, d.section, `${c.doc.meta.title} (사본)`, clone(c.doc.body));
        if (d.section === 'planning') p.planning.splice(p.planning.indexOf(of) + 1, 0, copy.meta.id);
        else {
          const part = p.parts.find((x) => x.docs.includes(of))!;
          part.docs.splice(part.docs.indexOf(of) + 1, 0, copy.meta.id);
        }
      }
    } else if (c.card) {
      if (action === 'take') p.cards.set(of, { ...c.card, id: of });
      else {
        const card = { ...c.card, id: id(), name: `${c.card.name} (사본)` };
        p.cards.set(card.id, card);
      }
    }
  },
  async storagePlaces() {
    return clone(PLACES);
  },
  async storageOf(path) {
    const lower = path.toLowerCase();
    const found = PLACES.filter((pl) => pl.service !== 'local' && lower.startsWith(pl.root.toLowerCase()));
    return found.length ? clone(found[0]) : null;
  },
  async projectMove(root, dest) {
    await wait();
    const p = project(root);
    const folder = root.split('\\').pop() ?? p.info.title;
    const target = `${dest}\\${folder}`;
    if (target.toLowerCase() === root.toLowerCase()) throw '이미 그 위치에 있음';
    projects.delete(root);
    projects.set(target, p);
    forgetRecent(root);
    touchRecent(target);
    return { overview: overview(target), leftBehind: false };
  },
  watchProject(root, onChange) {
    watcher = { root, onChange };
    return () => {
      if (watcher?.root === root) watcher = null;
    };
  },
} satisfies Partial<Backend>;

/**
 * Stand-in for another device, to try the screens in a browser:
 * `__otherDevice.edit(docId, ['문단', ...])` changes a chapter as a sync
 * program would, `__otherDevice.copy(docId, [...])` leaves a copy of it,
 * `__otherDevice.break(docId)` makes its file one that cannot be read (as if
 * saved as ANSI in Notepad) for 고쳐 열기; open the project again to see it.
 */
export const otherDevice = {
  edit(docId: string, paragraphs: string[]) {
    if (!watcher) return;
    const d = doc(project(watcher.root), docId);
    d.body = body(...paragraphs);
    d.modified = now();
    watcher.onChange([{ kind: 'doc', id: docId, rev: revOf(d.body) }]);
  },
  copy(docId: string, paragraphs: string[], device = 'DESKTOP-1AB2C3D') {
    if (!watcher) return;
    const p = project(watcher.root);
    const d = doc(p, docId);
    const copyDoc: MockDoc = { ...clone(d), body: body(...paragraphs), modified: now() };
    const file = `${docId}-${device}.md`;
    p.copies.push({
      info: { file, section: d.section, of: docId, title: d.meta.title, modified: now(), chars: counts(copyDoc.body).withSpaces, device },
      doc: copyDoc,
    });
    watcher.onChange([{ kind: 'doc', id: `${docId}-${device}`, rev: revOf(copyDoc.body) }]);
  },
  break(docId: string, reason = '다른 글자 방식(EUC-KR 등)으로 저장된 파일') {
    if (!watcher) return;
    doc(project(watcher.root), docId).broken = reason;
  },
};
