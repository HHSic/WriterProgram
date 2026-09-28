// In-memory stand-in for the Rust side, used when the screens run in a plain
// browser (`npm run dev` without Tauri). It keeps everything in memory and
// starts with one sample project, so layouts can be checked quickly.

import type { JSONContent } from '@tiptap/core';
import { blocksFromJSON, countBlocks } from '../editor/counts';
import { buildRegex } from '../editor/search';
import type {
  Appearance,
  Backend,
  Card,
  Change,
  CopyInfo,
  DriveInfo,
  DriveLink,
  DriveProvider,
  CardSummary,
  CardType,
  DocMeta,
  DocStatus,
  DocSummary,
  FormatCatalog,
  Note,
  ManuscriptFormat,
  Overview,
  Place,
  ProjectInfo,
  ProjectKind,
  RecentItem,
  SearchMatch,
  Section,
  SnapshotInfo,
  SnapshotKind,
  TrashItem,
  UserPreset,
} from './types';

interface MockDoc {
  meta: DocMeta;
  body: JSONContent;
  section: Section;
  modified?: string;
}

interface MockProject {
  info: ProjectInfo;
  cardTypes: CardType[];
  cards: Map<string, Card>;
  notes: Map<string, Note>;
  parts: { id: string; title: string; docs: string[] }[];
  planning: string[];
  docs: Map<string, MockDoc>;
  records: Map<string, { info: SnapshotInfo; body: JSONContent }[]>;
  trash: { item: TrashItem; doc: MockDoc }[];
  /** Copies "left by a sync program" (see otherDevice below). */
  copies: { info: CopyInfo; doc?: MockDoc; card?: Card }[];
}

const projects = new Map<string, MockProject>();
/** The screen's handler for changes "from another device", while a project is watched. */
let watcher: { root: string; onChange: (changes: Change[]) => void } | null = null;
let recent: RecentItem[] = [];
let userPresets: UserPreset[] = [];

// Same presets as crates/core/src/format.rs.
const SUBMISSION: ManuscriptFormat = {
  preset: 'submission-a4',
  paper: { kind: 'a4', widthMm: 210, heightMm: 297 },
  margins: { top: 20, bottom: 15, inside: 30, outside: 30, header: 15, footer: 15 },
  font: 'batang',
  sizePt: 10,
  lineSpacing: 160,
  letterSpacing: 0,
  indent: 1,
  blankLineBetween: false,
  chapterNewPage: true,
  pageNumbers: true,
  header: { content: 'none', text: '', align: 'center', skipChapterFirst: true },
};
const WEBNOVEL: ManuscriptFormat = {
  ...SUBMISSION,
  preset: 'webnovel',
  paper: { kind: 'none', widthMm: 210, heightMm: 297 },
  font: 'dotum',
  indent: 0,
  blankLineBetween: true,
  chapterNewPage: false,
  pageNumbers: false,
};
const SHINGUK: ManuscriptFormat = {
  ...SUBMISSION,
  preset: 'book-shinguk',
  paper: { kind: 'shinguk', widthMm: 152, heightMm: 225 },
  margins: { top: 22, bottom: 20, inside: 22, outside: 20, header: 8, footer: 8 },
  font: 'noto-serif',
  lineSpacing: 180,
  letterSpacing: -3,
};
const BOOK46: ManuscriptFormat = {
  ...SHINGUK,
  preset: 'book-46',
  paper: { kind: '46', widthMm: 128, heightMm: 188 },
  margins: { top: 18, bottom: 18, inside: 18, outside: 15, header: 7, footer: 7 },
  sizePt: 9.5,
  lineSpacing: 175,
};
const CATALOG: Omit<FormatCatalog, 'user'> = {
  builtin: [
    { id: 'submission-a4', name: '투고 원고 (A4)', format: SUBMISSION },
    { id: 'webnovel', name: '웹소설 플랫폼', format: WEBNOVEL },
    { id: 'book-shinguk', name: '책 (신국판)', format: SHINGUK },
    { id: 'book-46', name: '책 (46판)', format: BOOK46 },
  ],
  fonts: [
    { key: 'batang', label: '바탕 계열' },
    { key: 'dotum', label: '돋움 계열' },
    { key: 'nanum-myeongjo', label: '나눔명조' },
    { key: 'noto-serif', label: '본명조' },
    { key: 'gowun-batang', label: '고운바탕' },
  ],
  papers: [
    { key: 'a4', label: 'A4', widthMm: 210, heightMm: 297 },
    { key: 'b5', label: 'B5', widthMm: 182, heightMm: 257 },
    { key: 'a5', label: 'A5', widthMm: 148, heightMm: 210 },
    { key: 'shinguk', label: '신국판', widthMm: 152, heightMm: 225 },
    { key: '46', label: '46판', widthMm: 128, heightMm: 188 },
  ],
};

const defaultFormat = (kind: ProjectKind) => clone(kind === 'webnovel' ? WEBNOVEL : SUBMISSION);

/** Rough page estimate for the preview; the real one is in crates/core/src/layout.rs. */
function mockPages(chars: number, f: ManuscriptFormat): number | null {
  if (f.paper.kind === 'none') return null;
  const size = (f.sizePt * 25.4) / 72;
  const perLine = (f.paper.widthMm - f.margins.inside - f.margins.outside) / (size * (1 + f.letterSpacing / 100));
  const lines = Math.floor(
    (f.paper.heightMm - f.margins.top - f.margins.bottom - f.margins.header - f.margins.footer) / ((size * f.lineSpacing) / 100),
  );
  return Math.max(1, Math.ceil(chars / (perLine * lines * 0.85)));
}
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

const DEFAULT_TYPES: CardType[] = [
  { id: 'person', name: '인물', fields: ['나이', '직업', '말투', '외모', '관계'] },
  { id: 'place', name: '장소', fields: ['위치', '특징'] },
  { id: 'term', name: '용어', fields: ['정의'] },
];

function cardSummary(c: Card): CardSummary {
  const fields = c.fields
    .filter(([, v]) => v.trim())
    .slice(0, 3)
    .map(([k, v]) => `${k}: ${v.trim()}`);
  const line = c.description.split('\n').find((l) => l.trim());
  return {
    id: c.id,
    cardType: c.cardType,
    name: c.name,
    aliases: c.aliases,
    highlight: c.highlight,
    summary: [...(fields.length ? fields.join(' · ') : (line ?? ''))].slice(0, 80).join(''),
  };
}

/** A body without the marks of one note. */
function unmarkMemo(body: JSONContent, noteId: string): JSONContent {
  const strip = (node: JSONContent): JSONContent => ({
    ...node,
    marks: node.marks?.filter((m) => !(m.type === 'memo' && m.attrs?.id === noteId)),
    content: node.content?.map(strip),
  });
  return strip(clone(body));
}

function cardNames(c: Card): string[] {
  return [c.name, ...c.aliases]
    .map((n) => n.trim())
    .filter((n) => [...n].length >= 2)
    .sort((a, b) => [...b].length - [...a].length);
}

function namesRegex(names: string[]): RegExp | null {
  if (!names.length) return null;
  return new RegExp(names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'gu');
}

function paragraphTexts(body: JSONContent): { block: number; text: string }[] {
  return (body.content ?? []).flatMap((node, block) =>
    node.type === 'paragraph'
      ? [{ block, text: (node.content ?? []).map((i) => (i.type === 'hardBreak' ? '\n' : (i.text ?? ''))).join('') }]
      : [],
  );
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

function summary(d: MockDoc, format?: ManuscriptFormat): DocSummary {
  const c = counts(d.body);
  return {
    ...clone(d.meta),
    counts: c,
    pages: format && d.section === 'manuscript' ? mockPages(c.withSpaces, format) : null,
    modified: d.modified ?? d.meta.created,
  };
}

/** Fingerprint of a text, like the Rust side's (different numbers, same use). */
function revOf(body: JSONContent): string {
  let h = 0x811c9dc5;
  for (const ch of JSON.stringify(blocksFromJSON(body))) {
    h ^= ch.codePointAt(0)!;
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, '0');
}

function overview(root: string): Overview {
  const p = project(root);
  const format = p.info.manuscriptFormat;
  const parts = p.parts.map((part) => ({ id: part.id, title: part.title, docs: part.docs.map((i) => summary(doc(p, i), format)) }));
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
    totalPages: mockPages(total.withSpaces, format),
    cardTypes: clone(p.cardTypes),
    cards: [...p.cards.values()].sort((a, b) => a.name.localeCompare(b.name)).map(cardSummary),
    copies: p.copies.map((c) => clone(c.info)),
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
      manuscriptFormat: defaultFormat(kind),
    },
    parts: [{ id: id(), title: '1부', docs: [] }],
    planning: [],
    cardTypes: clone(DEFAULT_TYPES),
    cards: new Map(),
    notes: new Map(),
    docs: new Map(),
    records: new Map(),
    trash: [],
    copies: [],
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
  const seedCards: Omit<Card, 'id' | 'created' | 'highlight'>[] = [
    {
      cardType: 'person',
      name: '윤서하',
      aliases: ['서하', '윤 사장'],
      fields: [
        ['나이', '29'],
        ['직업', '달빛 서점 주인'],
        ['말투', '존댓말, 말수가 적음'],
        ['외모', ''],
        ['관계', '할머니에게 서점을 물려받음'],
      ],
      description: '밤 열한 시에 문을 닫는다. 할머니의 장부를 아직 다 읽지 못했다.',
    },
    {
      cardType: 'place',
      name: '달빛 서점',
      aliases: ['서점'],
      fields: [
        ['위치', '골목 끝, 가로등 아래'],
        ['특징', '지하에 잠긴 서고가 있다'],
      ],
      description: '',
    },
    {
      cardType: 'term',
      name: '대여 카드',
      aliases: [],
      fields: [['정의', '할머니 시절 책을 빌려 간 사람의 이름을 적던 카드']],
      description: '',
    },
  ];
  for (const c of seedCards) {
    const card: Card = { ...c, id: id(), created: now(), highlight: true };
    p.cards.set(card.id, card);
  }
  // Notes (메모): on a line of 3화, on the chapter, and for the project.
  const third = doc(p, p.parts[0].docs[2]);
  const memoId = id();
  const line = third.body.content?.[2]?.content?.[0];
  if (line) line.marks = [{ type: 'memo', attrs: { id: memoId } }];
  const at = now();
  const seedNotes: Note[] = [
    {
      id: memoId,
      anchor: 'text',
      target: third.meta.id,
      quote: '“영업, 끝났나요?”',
      text: '첫 대사. 더 지친 목소리였으면.',
      replies: [{ at, text: '12화에서 같은 대사로 받기' }],
      tags: ['퇴고'],
      done: false,
      created: at,
      updated: at,
    },
    { id: id(), anchor: 'doc', target: third.meta.id, quote: '', text: '이번 화 끝을 조금 더 당기기', replies: [], tags: ['할 일'], done: false, created: at, updated: at },
    { id: id(), anchor: 'project', target: '', quote: '', text: '2부에서 지하 서고 열기', replies: [], tags: ['아이디어'], done: false, created: at, updated: at },
  ];
  for (const n of seedNotes) p.notes.set(n.id, n);
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
    const snapshot = elsewhere ? record(p, d, 'other-device', '') : null;
    d.body = clone(b);
    d.modified = now();
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
  async exportFile(_root, items, opts, format, kind, dest, perDoc) {
    console.info('[mock] export file', { kind, dest, perDoc, format, opts, items });
    return perDoc ? items.map((i) => `${dest}\\${i.fileName}.${kind}`) : [dest];
  },
  async search(root, query) {
    const p = project(root);
    const re = buildRegex(query);
    if (!re) throw '찾는 식이 올바르지 않음';
    const ids = query.docIds ?? [...p.parts.flatMap((x) => x.docs), ...p.planning];
    const docs: { docId: string; matches: SearchMatch[] }[] = [];
    let total = 0;
    for (const docId of ids) {
      const d = p.docs.get(docId);
      if (!d) continue;
      const matches: SearchMatch[] = [];
      (d.body.content ?? []).forEach((node, block) => {
        if (node.type !== 'paragraph') return;
        const text = (node.content ?? []).map((i) => (i.type === 'hardBreak' ? '\n' : (i.text ?? ''))).join('');
        for (const m of text.matchAll(re)) {
          if (!m[0] || m.index === undefined) continue;
          matches.push({
            block,
            start: m.index,
            end: m.index + m[0].length,
            before: text.slice(Math.max(0, m.index - 24), m.index),
            text: m[0],
            after: text.slice(m.index + m[0].length, m.index + m[0].length + 24),
          });
        }
      });
      if (matches.length) {
        docs.push({ docId, matches });
        total += matches.length;
      }
    }
    return { docs, total, truncated: false };
  },
  async replaceAll(root, query, replacement) {
    const p = project(root);
    const re = buildRegex(query);
    if (!re) throw '찾는 식이 올바르지 않음';
    const ids = query.docIds ?? [...p.parts.flatMap((x) => x.docs), ...p.planning];
    const out: { docId: string; count: number; snapshot: SnapshotInfo }[] = [];
    let replaced = 0;
    for (const docId of ids) {
      const d = p.docs.get(docId);
      if (!d) continue;
      let count = 0;
      const next = clone(d.body);
      for (const node of next.content ?? []) {
        for (const inline of node.content ?? []) {
          if (inline.type !== 'text' || !inline.text) continue;
          inline.text = inline.text.replace(re, () => {
            count += 1;
            return replacement;
          });
        }
      }
      if (!count) continue;
      const snapshot = record(p, d, 'before-replace', '');
      d.body = next;
      replaced += count;
      out.push({ docId, count, snapshot });
    }
    return { replaced, docs: out };
  },
  async cardLoad(root, cardId) {
    const c = project(root).cards.get(cardId);
    if (!c) throw '카드를 찾을 수 없음';
    return clone(c);
  },
  async cardCreate(root, typeId, name) {
    const p = project(root);
    const kind = p.cardTypes.find((t) => t.id === typeId);
    if (!kind) throw '분류를 찾을 수 없음';
    if (!name.trim()) throw '카드 이름을 적어 주세요';
    const card: Card = {
      id: id(),
      cardType: typeId,
      name: name.trim(),
      aliases: [],
      fields: kind.fields.map((f): [string, string] => [f, '']),
      highlight: true,
      created: now(),
      description: '',
    };
    p.cards.set(card.id, card);
    return clone(card);
  },
  async cardSave(root, card) {
    const p = project(root);
    if (!card.name.trim()) throw '카드 이름을 적어 주세요';
    const saved: Card = {
      ...clone(card),
      name: card.name.trim(),
      aliases: card.aliases.map((a) => a.trim()).filter((a) => a && a !== card.name.trim()),
      fields: card.fields.filter(([k]) => k.trim()),
    };
    p.cards.set(saved.id, saved);
    return cardSummary(saved);
  },
  async cardTrash(root, cardId) {
    const p = project(root);
    const c = p.cards.get(cardId);
    if (!c) throw '카드를 찾을 수 없음';
    p.cards.delete(cardId);
    const item: TrashItem = {
      id: `${Date.now()}-${cardId}`,
      docId: cardId,
      section: 'cards',
      title: c.name,
      deletedAt: now(),
      partId: null,
      index: 0,
      chars: c.description.length,
    };
    const holder: MockDoc & { card?: Card } = {
      meta: { id: cardId, title: c.name, synopsis: '', status: 'draft', target: null, created: c.created },
      body: { type: 'doc' },
      section: 'cards',
      card: c,
    };
    p.trash.unshift({ item, doc: holder });
    return clone(item);
  },
  async cardAppearances(root, cardId) {
    const p = project(root);
    const c = p.cards.get(cardId);
    if (!c) throw '카드를 찾을 수 없음';
    const re = namesRegex(cardNames(c));
    if (!re) return [];
    const out: Appearance[] = [];
    for (const docId of p.parts.flatMap((x) => x.docs)) {
      const samples: SearchMatch[] = [];
      let count = 0;
      for (const { block, text } of paragraphTexts(doc(p, docId).body)) {
        for (const m of text.matchAll(re)) {
          if (m.index === undefined) continue;
          count += 1;
          if (samples.length < 3) {
            samples.push({
              block,
              start: m.index,
              end: m.index + m[0].length,
              before: text.slice(Math.max(0, m.index - 24), m.index),
              text: m[0],
              after: text.slice(m.index + m[0].length, m.index + m[0].length + 24),
            });
          }
        }
      }
      if (count) out.push({ docId, count, samples });
    }
    return out;
  },
  async cardCounts(root) {
    const p = project(root);
    return [...p.cards.values()].map((c): [string, number] => {
      const re = namesRegex(cardNames(c));
      const n = re
        ? p.parts
            .flatMap((x) => x.docs)
            .filter((d) => paragraphTexts(doc(p, d).body).some(({ text }) => new RegExp(re.source, 'u').test(text))).length
        : 0;
      return [c.id, n];
    });
  },
  async cardTypeAdd(root, name) {
    const p = project(root);
    if (!name.trim()) throw '분류 이름을 적어 주세요';
    const kind: CardType = { id: id(), name: name.trim(), fields: [] };
    p.cardTypes.push(kind);
    return clone(kind);
  },
  async cardTypeUpdate(root, kind) {
    const p = project(root);
    const slot = p.cardTypes.find((t) => t.id === kind.id);
    if (!slot) throw '분류를 찾을 수 없음';
    slot.name = kind.name.trim();
    slot.fields = kind.fields.map((f) => f.trim()).filter(Boolean);
  },
  async cardTypeRemove(root, typeId) {
    const p = project(root);
    if ([...p.cards.values()].some((c) => c.cardType === typeId)) throw '카드가 남아 있는 분류는 지울 수 없음';
    p.cardTypes = p.cardTypes.filter((t) => t.id !== typeId);
  },
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
  async formatCatalog() {
    return clone({ ...CATALOG, user: userPresets });
  },
  async formatSavePreset(name, format) {
    const trimmed = name.trim();
    if (!trimmed) throw '서식 이름을 적어 주세요';
    if (CATALOG.builtin.some((b) => b.name === trimmed)) throw '기본 서식과 같은 이름은 쓸 수 없음';
    userPresets = [...userPresets.filter((p) => p.name !== trimmed), { name: trimmed, format: { ...clone(format), preset: trimmed } }];
    return clone(userPresets);
  },
  async formatDeletePreset(name) {
    userPresets = userPresets.filter((p) => p.name !== name);
    return clone(userPresets);
  },
  async formatEstimate(root, format) {
    return mockPages(overview(root).total.withSpaces, format);
  },
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
    recent = recent.filter((r) => r.path !== root);
    touchRecent(target);
    return { overview: overview(target), leftBehind: false };
  },
  async driveStatus() {
    return {
      drives: drives.map((d) => ({ ...d, account: d.account ? { ...d.account } : null })),
      appsFile: 'C:\\Users\\작가\\AppData\\Roaming\\com.writerprogram.desktop\\drive-apps.json',
    };
  },
  async driveConnect(provider) {
    const d = drive(provider);
    if (!d.registered) throw `${d.label} 앱 등록 전이라 아직 연결할 수 없음`;
    connecting = true;
    for (let i = 0; i < 12 && connecting; i++) await new Promise((r) => setTimeout(r, 100));
    if (!connecting) throw '연결을 그만둠';
    connecting = false;
    d.account = { name: '윤서하', email: 'writer@example.com' };
    d.connectedAt = now();
    return { ...d.account };
  },
  async driveCancel() {
    connecting = false;
  },
  async driveDisconnect(provider) {
    const d = drive(provider);
    d.account = null;
    d.connectedAt = null;
    for (const [id, link] of links) if (link.provider === provider) links.delete(id);
  },
  async driveProjects(provider) {
    if (!drive(provider).account) throw '연결되어 있지 않음';
    return [...projects.values()]
      .filter((p) => [...links.entries()].some(([id, l]) => id === p.info.id && l.provider === provider))
      .map((p) => ({ folder: p.info.title, title: p.info.title, id: p.info.id }));
  },
  async driveFetch(_provider, folder) {
    await wait();
    const root = [...projects.keys()].find((r) => project(r).info.title === folder);
    if (!root) throw '드라이브의 이 폴더에는 작품이 없음';
    touchRecent(root);
    return overview(root);
  },
  async projectLinkGet(projectId) {
    const link = links.get(projectId);
    return link ? { ...link } : null;
  },
  async projectLink(projectId, title, provider) {
    if (!drive(provider).account) throw `${drive(provider).label}에 연결되어 있지 않음`;
    const link: DriveLink = { provider, folder: title, linkedAt: now(), syncedAt: null, error: null };
    links.set(projectId, link);
    return { ...link };
  },
  async projectUnlink(projectId) {
    links.delete(projectId);
  },
  async projectSync(_root, projectId) {
    await wait();
    const link = links.get(projectId);
    if (!link) throw '이 작품은 드라이브와 맞추지 않음';
    link.syncedAt = now();
    link.error = null;
    return {
      link: { ...link },
      report: { uploaded: [], downloaded: [], removedHere: [], removedThere: [], copies: [], merged: false, later: [] },
    };
  },
  async browserOpen() {},
  async browserBounds() {},
  async browserNavigate(_label, url) {
    return url;
  },
  async browserStep() {},
  async browserClose() {},
  async browserClip() {
    throw '앱 안 브라우저는 데스크톱 앱에서만 됨';
  },
  onBrowserPage() {
    return () => {};
  },
  watchProject(root, onChange) {
    watcher = { root, onChange };
    return () => {
      if (watcher?.root === root) watcher = null;
    };
  },
  async pickSaveFile(_title, defaultName) {
    return `C:\\Users\\작가\\Documents\\${defaultName}`;
  },
  onCloseRequested() {},
  setWindowTitle(title) {
    document.title = title;
  },
};

const PLACES: Place[] = [
  { service: 'onedrive', label: 'OneDrive', root: 'C:\\Users\\작가\\OneDrive', suggested: 'C:\\Users\\작가\\OneDrive\\WriterProgram' },
  { service: 'googledrive', label: 'Google Drive', root: 'G:\\내 드라이브', suggested: 'G:\\내 드라이브\\WriterProgram' },
  { service: 'local', label: '이 PC', root: 'C:\\Users\\작가\\Documents', suggested: 'C:\\Users\\작가\\Documents\\WriterProgram' },
];

/**
 * Stand-in for another device, to try the screens in a browser:
 * `__otherDevice.edit(docId, ['문단', ...])` changes a chapter as a sync
 * program would, `__otherDevice.copy(docId, [...])` leaves a copy of it.
 */
const otherDevice = {
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
};
if (typeof window !== 'undefined') (window as unknown as { __otherDevice: typeof otherDevice }).__otherDevice = otherDevice;

/** Drives in the browser preview: all "registered", none connected. */
const drives: DriveInfo[] = [
  { provider: 'google', label: 'Google Drive', registered: true, account: null, connectedAt: null },
  { provider: 'onedrive', label: 'OneDrive', registered: true, account: null, connectedAt: null },
  { provider: 'dropbox', label: 'Dropbox', registered: false, account: null, connectedAt: null },
];
const links = new Map<string, DriveLink>();
let connecting = false;

function drive(provider: DriveProvider): DriveInfo {
  return drives.find((d) => d.provider === provider)!;
}
