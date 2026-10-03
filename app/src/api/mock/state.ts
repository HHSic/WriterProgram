// The stand-in's projects in memory, the helpers every part of it shares, and the sample project.

import type { JSONContent } from '@tiptap/core';
import { blocksFromJSON, countBlocks } from '../../editor/counts';
import type {
  Card,
  CardSummary,
  CardType,
  CopyInfo,
  DocMeta,
  DocStatus,
  DocSummary,
  ManuscriptFormat,
  Note,
  Overview,
  ProjectInfo,
  ProjectKind,
  RecentItem,
  Section,
  SnapshotInfo,
  SnapshotKind,
  TrashItem,
  UnreadableDoc,
} from '../types';
import { SUBMISSION, WEBNOVEL, mockPages } from './presets';

export interface MockDoc {
  meta: DocMeta;
  body: JSONContent;
  section: Section;
  modified?: string;
  /** Why its file "cannot be read" (고쳐 열기 clears it); see `__breakDoc`. */
  broken?: string;
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

export const projects = new Map<string, MockProject>();
let recent: RecentItem[] = [];

/** The recently opened projects, newest first. */
export const recentItems = () => recent;

export function forgetRecent(path: string) {
  recent = recent.filter((r) => r.path !== path);
}

let seq = 0;

export const id = () => `m${(++seq).toString(36)}${Math.random().toString(36).slice(2, 6)}`;
export const now = () => new Date().toISOString();
export const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
export const wait = () => new Promise((r) => setTimeout(r, 40));

export function body(...paragraphs: string[]): JSONContent {
  return {
    type: 'doc',
    content: paragraphs.map((p) =>
      p === '***' ? { type: 'sceneBreak' } : p ? { type: 'paragraph', content: [{ type: 'text', text: p }] } : { type: 'paragraph' },
    ),
  };
}

export function newDoc(p: MockProject, section: Section, title: string, text: JSONContent = body(''), status: DocStatus = 'draft') {
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

export function cardSummary(c: Card): CardSummary {
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

export function project(root: string): MockProject {
  const p = projects.get(root);
  if (!p) throw '작품 폴더가 아님 (project.json이 없음)';
  return p;
}

export function doc(p: MockProject, docId: string): MockDoc {
  const d = p.docs.get(docId);
  if (!d) throw '문서를 찾을 수 없음';
  return d;
}

export const counts = (b: JSONContent) => countBlocks(blocksFromJSON(b));

export function summary(d: MockDoc, format?: ManuscriptFormat): DocSummary {
  const c = counts(d.body);
  return {
    ...clone(d.meta),
    counts: c,
    pages: format && d.section === 'manuscript' ? mockPages(c.withSpaces, format) : null,
    modified: d.modified ?? d.meta.created,
  };
}

/** Fingerprint of a text, like the Rust side's (different numbers, same use). */
export function revOf(body: JSONContent): string {
  let h = 0x811c9dc5;
  for (const ch of JSON.stringify(blocksFromJSON(body))) {
    h ^= ch.codePointAt(0)!;
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, '0');
}

export function overview(root: string): Overview {
  const p = project(root);
  const format = p.info.manuscriptFormat;
  const unreadable: UnreadableDoc[] = [];
  // Like listed_docs on the Rust side: unreadable ones out of the list, at their place in `unreadable`.
  const readable = (ids: string[], section: Section, part: string | null) =>
    ids.filter((i, index) => {
      const d = doc(p, i);
      if (!d.broken) return true;
      unreadable.push({ id: i, section, part, index, titleGuess: d.meta.title, reason: d.broken, repairable: true });
      return false;
    });
  const parts = p.parts.map((part) => ({
    id: part.id,
    title: part.title,
    docs: readable(part.docs, 'manuscript', part.id).map((i) => summary(doc(p, i), format)),
  }));
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
    planning: readable(p.planning, 'planning', null).map((i) => summary(doc(p, i))),
    trashCount: p.trash.length,
    total,
    totalPages: mockPages(total.withSpaces, format),
    cardTypes: clone(p.cardTypes),
    cards: [...p.cards.values()].sort((a, b) => a.name.localeCompare(b.name)).map(cardSummary),
    copies: p.copies.map((c) => clone(c.info)),
    unreadable,
  };
}

export function touchRecent(root: string) {
  const ov = overview(root);
  recent = [
    { path: root, title: ov.project.title, kind: ov.project.kind, chars: ov.total.withSpaces, openedAt: now(), exists: true },
    ...recent.filter((r) => r.path !== root),
  ];
}

export function record(p: MockProject, d: MockDoc, kind: SnapshotKind, name: string): SnapshotInfo {
  const info: SnapshotInfo = { id: `${Date.now()}.${kind}`, kind, name: name.trim(), at: now(), counts: counts(d.body) };
  const list = p.records.get(d.meta.id) ?? [];
  list.unshift({ info, body: clone(d.body) });
  p.records.set(d.meta.id, list);
  return info;
}

const defaultFormat = (kind: ProjectKind) => clone(kind === 'webnovel' ? WEBNOVEL : SUBMISSION);

export function createProject(parent: string, title: string, kind: ProjectInfo['kind'], perDoc: number | null, countSpaces: boolean): string {
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
      keepDaily: false,
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

export function seed() {
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
