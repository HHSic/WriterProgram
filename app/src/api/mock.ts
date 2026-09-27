// In-memory stand-in for the Rust side, used when the screens run in a plain
// browser (`npm run dev` without Tauri). It keeps everything in memory and
// starts with one sample project, so layouts can be checked quickly.

import type { JSONContent } from '@tiptap/core';
import { blocksFromJSON, countBlocks } from '../editor/counts';
import type {
  Backend,
  DocMeta,
  DocStatus,
  DocSummary,
  Overview,
  ProjectInfo,
  RecentItem,
  Section,
  SnapshotInfo,
  SnapshotKind,
  TrashItem,
} from './types';

interface MockDoc {
  meta: DocMeta;
  body: JSONContent;
  section: Section;
}

interface MockProject {
  info: ProjectInfo;
  parts: { id: string; title: string; docs: string[] }[];
  planning: string[];
  docs: Map<string, MockDoc>;
  records: Map<string, { info: SnapshotInfo; body: JSONContent }[]>;
  trash: { item: TrashItem; doc: MockDoc }[];
}

const projects = new Map<string, MockProject>();
let recent: RecentItem[] = [];
let seq = 0;

const id = () => `m${(++seq).toString(36)}${Math.random().toString(36).slice(2, 6)}`;
const now = () => new Date().toISOString();
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
const wait = () => new Promise((r) => setTimeout(r, 40));

function body(...paragraphs: string[]): JSONContent {
  return {
    type: 'doc',
    content: paragraphs.map((p) =>
      p === '***' ? { type: 'sceneBreak' } : p ? { type: 'paragraph', content: [{ type: 'text', text: p }] } : { type: 'paragraph' },
    ),
  };
}

function newDoc(p: MockProject, section: Section, title: string, text: JSONContent = body(''), status: DocStatus = 'draft') {
  const doc: MockDoc = {
    meta: { id: id(), title, synopsis: '', status, target: null, created: now() },
    body: text,
    section,
  };
  p.docs.set(doc.meta.id, doc);
  return doc;
}

function project(root: string): MockProject {
  const p = projects.get(root);
  if (!p) throw '작품 폴더가 아님 (project.json이 없음)';
  return p;
}

function doc(p: MockProject, docId: string): MockDoc {
  const d = p.docs.get(docId);
  if (!d) throw '문서를 찾을 수 없음';
  return d;
}

const counts = (b: JSONContent) => countBlocks(blocksFromJSON(b));

function summary(d: MockDoc): DocSummary {
  return { ...clone(d.meta), counts: counts(d.body) };
}

function overview(root: string): Overview {
  const p = project(root);
  const parts = p.parts.map((part) => ({ id: part.id, title: part.title, docs: part.docs.map((i) => summary(doc(p, i))) }));
  const total = parts
    .flatMap((part) => part.docs)
    .reduce(
      (t, d) => ({
        withSpaces: t.withSpaces + d.counts.withSpaces,
        withoutSpaces: t.withoutSpaces + d.counts.withoutSpaces,
        manuscriptLines: t.manuscriptLines + d.counts.manuscriptLines,
        manuscriptPages: t.manuscriptPages + d.counts.manuscriptPages,
      }),
      { withSpaces: 0, withoutSpaces: 0, manuscriptLines: 0, manuscriptPages: 0 },
    );
  return {
    root,
    project: clone(p.info),
    parts,
    planning: p.planning.map((i) => summary(doc(p, i))),
    trashCount: p.trash.length,
    total,
  };
}

function touchRecent(root: string) {
  const ov = overview(root);
  recent = [
    { path: root, title: ov.project.title, kind: ov.project.kind, chars: ov.total.withSpaces, openedAt: now(), exists: true },
    ...recent.filter((r) => r.path !== root),
  ];
}

function record(p: MockProject, d: MockDoc, kind: SnapshotKind, name: string): SnapshotInfo {
  const info: SnapshotInfo = { id: `${Date.now()}.${kind}`, kind, name: name.trim(), at: now(), counts: counts(d.body) };
  const list = p.records.get(d.meta.id) ?? [];
  list.unshift({ info, body: clone(d.body) });
  p.records.set(d.meta.id, list);
  return info;
}

function createProject(parent: string, title: string, kind: ProjectInfo['kind'], perDoc: number | null, countSpaces: boolean): string {
  const root = `${parent}\\${title}`;
  const p: MockProject = {
    info: {
      id: id(),
      title,
      kind,
      penName: '',
      created: now(),
      goal: { perDoc, countSpaces, daily: null },
      sceneBreak: kind === 'webnovel' ? '◆' : '*',
      preset: kind === 'webnovel' ? 'webnovel' : 'manuscript-a4',
    },
    parts: [{ id: id(), title: '1부', docs: [] }],
    planning: [],
    docs: new Map(),
    records: new Map(),
    trash: [],
  };
  projects.set(root, p);
  p.planning.push(newDoc(p, 'planning', '시놉시스').meta.id, newDoc(p, 'planning', '작품 소개').meta.id);
  return root;
}

function seed() {
  const root = createProject('C:\\Users\\작가\\Documents\\WriterProgram', '달빛 서점의 마지막 손님', 'webnovel', 5000, true);
  const p = project(root);
  const chapters: [string, DocStatus, string[]][] = [
    ['문 닫는 시간', 'stock', [
      '서하는 매일 밤 열한 시에 서점 문을 닫았다. 할머니가 그랬고, 할머니의 어머니도 그랬다고 했다.',
      '“오늘도 손님은 없었네.”',
      '혼잣말은 오래된 서가 사이로 흩어졌다.',
    ]],
    ['빗소리가 들리는 밤', 'stock', [
      '비는 저녁부터 내렸다. 유리문에 맺힌 빗방울 너머로 골목의 가로등이 번졌다.',
      '서하는 장부를 덮고 창밖을 오래 보았다.',
    ]],
    ['비에 젖은 손님', 'draft', [
      '셔터를 반쯤 내렸을 때 종이 울렸다. 이 시간에 문을 여는 사람은 없었다. 적어도 지난 삼 년 동안은 그랬다.',
      '윤서하는 계산대 아래에서 우산을 꺼내 들었다. 문턱에 선 남자는 우산도 없이 젖어 있었고, 품에 안은 종이봉투만은 이상하리만치 말라 있었다.',
      '“영업, 끝났나요?”',
      '“끝났어요. 그런데 들어오세요. 그 봉투가 젖으면 곤란할 것 같으니까.”',
      '***',
      '남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다. 누렇게 바랜 칸마다 같은 이름이 적혀 있었다.',
    ]],
  ];
  for (const [title, status, text] of chapters) {
    const d = newDoc(p, 'manuscript', title, body(...text), status);
    p.parts[0].docs.push(d.meta.id);
  }
  p.parts[0].title = '1부 · 서점의 문';
  p.parts.push({ id: id(), title: '2부 · 비밀 서가', docs: [] });
  doc(p, p.parts[0].docs[2]).meta.synopsis = '폐점 직전 찾아온 손님이 서하에게 할머니 시절의 대여 카드를 내민다.';
  touchRecent(root);
}

seed();

export const mockBackend: Backend = {
  isDesktop: false,
  async recentList() {
    await wait();
    return clone(recent);
  },
  async recentRemove(path) {
    recent = recent.filter((r) => r.path !== path);
  },
  async defaultLocation() {
    return 'C:\\Users\\작가\\Documents\\WriterProgram';
  },
  async projectCreate(opts) {
    await wait();
    const title = opts.title.trim();
    if (!title) throw '작품 제목을 적어 주세요';
    const root = createProject(opts.parent, title, opts.kind, opts.perDocGoal, opts.countSpaces);
    const p = project(root);
    if (opts.firstChapter) p.parts[0].docs.push(newDoc(p, 'manuscript', '').meta.id);
    touchRecent(root);
    return overview(root);
  },
  async projectOpen(path) {
    await wait();
    touchRecent(path);
    return overview(path);
  },
  async projectOverview(root) {
    return overview(root);
  },
  async projectUpdate(root, patch) {
    const p = project(root);
    Object.assign(p.info, patch);
    return clone(p.info);
  },
  async partAdd(root, title) {
    const p = project(root);
    const part = { id: id(), title: title.trim() || `${p.parts.length + 1}부`, docs: [] };
    p.parts.push(part);
    return part.id;
  },
  async partRename(root, partId, title) {
    const part = project(root).parts.find((x) => x.id === partId);
    if (!part) throw '부를 찾을 수 없음';
    if (!title.trim()) throw '부 이름을 적어 주세요';
    part.title = title.trim();
  },
  async partRemove(root, partId) {
    const p = project(root);
    const part = p.parts.find((x) => x.id === partId);
    if (!part) throw '부를 찾을 수 없음';
    if (part.docs.length) throw '회차가 남아 있는 부는 지울 수 없음. 회차를 먼저 옮기거나 휴지통으로 보내 주세요.';
    if (p.parts.length === 1) throw '부가 하나뿐이라 지울 수 없음';
    p.parts = p.parts.filter((x) => x.id !== partId);
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
  async docLoad(root, docId) {
    await wait();
    const d = doc(project(root), docId);
    return { meta: clone(d.meta), body: clone(d.body), counts: counts(d.body) };
  },
  async docSave(root, docId, b) {
    await wait();
    const d = doc(project(root), docId);
    d.body = clone(b);
    return { counts: counts(b), snapshot: null };
  },
  async docUpdateMeta(root, docId, patch) {
    const d = doc(project(root), docId);
    if (patch.title !== undefined) d.meta.title = patch.title.trim();
    if (patch.synopsis !== undefined) d.meta.synopsis = patch.synopsis.trim();
    if (patch.status !== undefined) d.meta.status = patch.status;
    if (patch.target !== undefined) d.meta.target = patch.target && patch.target > 0 ? patch.target : null;
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
    return { meta: clone(doc(p, docId).meta), body: clone(r.body), counts: r.info.counts };
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
    p.docs.set(t.item.docId, t.doc);
    if (t.item.section === 'planning') p.planning.splice(t.item.index, 0, t.item.docId);
    else (p.parts.find((x) => x.id === t.item.partId) ?? p.parts[p.parts.length - 1]).docs.splice(t.item.index, 0, t.item.docId);
    p.trash = p.trash.filter((x) => x !== t);
  },
  async trashDelete(root, trashId) {
    const p = project(root);
    p.trash = p.trash.filter((x) => x.item.id !== trashId);
  },
  async exportText(root, items, opts) {
    const p = project(root);
    const sep = opts.blankLineBetween ? '\n\n' : '\n';
    return items
      .map((item) => {
        const blocks = blocksFromJSON(doc(p, item.docId).body);
        const text = blocks
          .map((b) => (b.scene ? (opts.blankLineBetween ? opts.sceneBreak : `\n${opts.sceneBreak}\n`) : b.lines.join('\n')))
          .join(sep);
        return opts.includeTitles && item.heading.trim() ? `${item.heading.trim()}\n\n${text}` : text;
      })
      .join('\n\n\n');
  },
  async exportTxt(root, items, opts, dest, perDoc) {
    const text = await mockBackend.exportText(root, items, opts);
    console.info('[mock] export', { dest, perDoc, text });
    return perDoc ? items.map((i) => `${dest}\\${i.fileName}.txt`) : [dest];
  },
  async reveal(path) {
    console.info('[mock] reveal', path);
  },
  async pickFolder() {
    return 'C:\\Users\\작가\\Documents\\WriterProgram';
  },
  async pickSaveFile(_title, defaultName) {
    return `C:\\Users\\작가\\Documents\\${defaultName}`;
  },
  onCloseRequested() {},
  setWindowTitle(title) {
    document.title = title;
  },
};
