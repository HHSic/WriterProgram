// 교정본 검토 (workspace/ReviewPane.tsx): the writer's choices before they are
// applied, and a chapter's text cut into pieces with the editor's changes,
// looks and notes in place. Pure helpers; docs/corrections.md has the data.

import type {
  Applied,
  ChangeClass,
  ChapterReview,
  Correction,
  Decisions,
  EditorNote,
  Exchange,
  ExchangeInfo,
  ReviewLook,
  ReviewSpan,
} from '../api/types';
import { num } from './format';

export const CLASSES: ChangeClass[] = ['spacing', 'punctuation', 'wording'];

export const CLASS_LABEL: Record<ChangeClass, string> = {
  spacing: '띄어쓰기',
  punctuation: '문장부호',
  wording: '문장 고침',
};

/** U+2029 paragraph break, U+E000 scene break (as in `before` / `after`). */
export const PARA = ' ';
export const SCENE = '';

/** A change's text for a list or the page: breaks shown as marks. */
export function visible(text: string, sceneBreak = '◆'): string {
  return text.replaceAll(PARA, '¶').replaceAll(SCENE, sceneBreak).replaceAll('\n', '↵');
}

// Choices --------------------------------------------------------------------

/** What the writer picked on this screen, not yet applied (반영하기). */
export interface Choices {
  changes: Record<string, 'accept' | 'reject'>;
  notes: Record<string, 'keep' | 'drop'>;
}

export const NO_CHOICES: Choices = { changes: {}, notes: {} };

/** accepted / rejected: decided before; accept / reject: picked now; open: neither. */
export type ChangeView = 'accepted' | 'rejected' | 'accept' | 'reject' | 'open';

export function changeView(c: Correction, choices: Choices): ChangeView {
  if (c.state !== 'pending') return c.state;
  return choices.changes[c.id] ?? 'open';
}

/** Can be applied as it is: still open and placed in the chapter now (not 겹침). */
export function canAccept(c: Correction): boolean {
  return c.state === 'pending' && !c.overlap && c.now !== null;
}

export function chooseChange(choices: Choices, id: string, choice: 'accept' | 'reject' | null): Choices {
  const changes = { ...choices.changes };
  if (choice) changes[id] = choice;
  else delete changes[id];
  return { ...choices, changes };
}

export function chooseNote(choices: Choices, id: string, choice: 'keep' | 'drop' | null): Choices {
  const notes = { ...choices.notes };
  if (choice) notes[id] = choice;
  else delete notes[id];
  return { ...choices, notes };
}

/** 모두 받아들이기 for one class: every change of it that can be applied. */
export function acceptClass(choices: Choices, changes: Correction[], cls: ChangeClass): Choices {
  const next = { ...choices.changes };
  for (const c of changes) if (c.class === cls && canAccept(c)) next[c.id] = 'accept';
  return { ...choices, changes: next };
}

/** 남은 곳 모두 받아들이기: every change with nothing picked that can be applied. */
export function acceptRemaining(choices: Choices, changes: Correction[]): Choices {
  const next = { ...choices.changes };
  for (const c of changes) if (canAccept(c) && !next[c.id]) next[c.id] = 'accept';
  return { ...choices, changes: next };
}

/** Drops picks for changes and notes that are no longer open (applied, or from another file). */
export function keepOpen(choices: Choices, chapters: ChapterReview[]): Choices {
  const open = new Set(chapters.flatMap((c) => [...c.changes, ...c.notes]).filter((x) => x.state === 'pending').map((x) => x.id));
  const only = <T>(map: Record<string, T>) => Object.fromEntries(Object.entries(map).filter(([id]) => open.has(id)));
  return { changes: only(choices.changes), notes: only(choices.notes) };
}

/** Takes back what was picked for one class. */
export function clearClass(choices: Choices, changes: Correction[], cls: ChangeClass): Choices {
  const next = { ...choices.changes };
  for (const c of changes) if (c.class === cls && c.state === 'pending') delete next[c.id];
  return { ...choices, changes: next };
}

export interface ClassCount {
  /** Open changes of the class that can be applied. */
  appliable: number;
  /** Of those, picked to accept. */
  accepting: number;
  /** Open with nothing picked yet (겹침 included). */
  open: number;
}

export function classCounts(changes: Correction[], choices: Choices): Record<ChangeClass, ClassCount> {
  const out = {} as Record<ChangeClass, ClassCount>;
  for (const cls of CLASSES) out[cls] = { appliable: 0, accepting: 0, open: 0 };
  for (const c of changes) {
    if (c.state !== 'pending') continue;
    const n = out[c.class];
    const picked = choices.changes[c.id];
    if (!picked) n.open += 1;
    if (canAccept(c)) {
      n.appliable += 1;
      if (picked === 'accept') n.accepting += 1;
    }
  }
  return out;
}

/** 남은 곳: open changes with nothing picked. */
export function remaining(changes: Correction[], choices: Choices): number {
  return changes.filter((c) => c.state === 'pending' && !choices.changes[c.id]).length;
}

export function hasChoices(choices: Choices): boolean {
  return Object.keys(choices.changes).length > 0 || Object.keys(choices.notes).length > 0;
}

export function decisionsOf(choices: Choices): Decisions {
  const pick = <T extends string>(map: Record<string, T>, value: T) =>
    Object.entries(map)
      .filter(([, v]) => v === value)
      .map(([id]) => id);
  return {
    accept: pick(choices.changes, 'accept'),
    reject: pick(choices.changes, 'reject'),
    keepNotes: pick(choices.notes, 'keep'),
    dropNotes: pick(choices.notes, 'drop'),
  };
}

/** What 반영하기 did, in sentences for a toast. */
export function appliedText(a: Applied): string {
  const parts: string[] = [];
  if (a.accepted.length && a.rejected.length) {
    parts.push(`바뀐 곳 ${num(a.accepted.length)}군데를 받아들이고 ${num(a.rejected.length)}군데는 원래대로 두었습니다.`);
  } else if (a.accepted.length) {
    parts.push(`바뀐 곳 ${num(a.accepted.length)}군데를 받아들였습니다.`);
  } else if (a.rejected.length) {
    parts.push(`바뀐 곳 ${num(a.rejected.length)}군데를 원래대로 두었습니다.`);
  }
  if (a.memos.length) parts.push(`편집자 메모 ${num(a.memos.length)}개를 메모로 남겼습니다.`);
  if (a.skipped.length) parts.push(`${num(a.skipped.length)}군데는 반영하지 못해 직접 고쳐야 합니다.`);
  if (a.docs.length) parts.push('고치기 전 원고는 기록에 ‘교정 반영 전’으로 남았습니다.');
  return parts.join(' ') || '반영할 것이 없었습니다.';
}

// Words for the list of what was sent ---------------------------------------------

/** "10월 3일" in local time. */
export function dayLabel(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : `${d.getMonth() + 1}월 ${d.getDate()}일`;
}

/** The chapters sent: "3화 비에 젖은 손님", or "1화 문 닫는 시간 외 2개". */
export function sentChaptersText(ex: Exchange): string {
  const first = ex.chapters[0];
  if (!first) return '';
  const name = first.heading || first.title;
  return ex.chapters.length > 1 ? `${name} 외 ${num(ex.chapters.length - 1)}개` : name;
}

/** Where an exchange stands, in one sentence. */
export function exchangeStateText(ex: ExchangeInfo): string {
  const last = ex.received[ex.received.length - 1];
  if (!last) return '교정본을 아직 받지 않았습니다.';
  const got = `${dayLabel(last.at)}에 받은 교정본(${last.name})`;
  if (ex.pending === null) return `${got}을 읽었습니다.`;
  if (ex.pending === 0) return `${got}을 모두 살펴보았습니다.`;
  return `${got}에 아직 정하지 않은 곳이 ${num(ex.pending)}군데 있습니다.`;
}

/** How the editor marked a change, for its line in the list (nothing for a plain comparison). */
export function howText(c: Correction): string {
  const who = [c.author, c.date ? dayTime(c.date) : null].filter(Boolean).join(' · ');
  switch (c.how) {
    case 'tracked':
      return who ? `변경 내용 추적 · ${who}` : '변경 내용 추적';
    case 'strike':
      return '취소선으로 표시';
    case 'color':
      return '색 글자로 넣음';
    case 'compared':
      return '';
  }
}

/** "10월 1일 9:12" in local time. */
function dayTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return `${d.getMonth() + 1}월 ${d.getDate()}일 ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
}

// Pieces of the text -----------------------------------------------------------

export type PieceKind = 'text' | 'del' | 'ins';

/** A run of a paragraph with the same marks. */
export interface Piece {
  text: string;
  kind: PieceKind;
  /** The change it shows (del, ins). */
  change?: string;
  /** Looks and notes on it. */
  looks: string[];
  notes: string[];
  /** Looks and notes that start here (their mark is drawn here). */
  starts: string[];
  /** A paragraph break deleted (¶). */
  para?: boolean;
}

interface Ranged {
  id: string;
  span: ReviewSpan;
}

/** The part of `span` in paragraph `block` (length `len`), and whether it runs past its end. */
function inBlock(span: ReviewSpan, block: number, len: number): { from: number; to: number; pastEnd: boolean } | null {
  const { from, to } = span;
  if (block < from.block || block > to.block) return null;
  const start = block === from.block ? Math.min(from.offset, len) : 0;
  const end = block === to.block ? Math.min(Math.max(to.offset, 0), len) : len;
  return { from: start, to: Math.max(start, end), pastEnd: block < to.block };
}

/**
 * The chapter's paragraphs as they are now (`texts`: one string a
 * paragraph, line breaks as `\n`, a scene break as U+E000) cut into pieces:
 * open changes as deleted and inserted text where they are now, looks and
 * notes over the text they are on. Changes decided earlier are already in
 * (or out of) the text and show nothing; 겹침 has no place and shows nothing.
 */
export function reviewParagraphs(texts: string[], chapter: ChapterReview): Piece[][] {
  const changes = chapter.changes.filter((c) => c.state === 'pending' && c.now) as (Correction & { now: ReviewSpan })[];
  const looks: Ranged[] = chapter.looks.filter((l): l is ReviewLook & { now: ReviewSpan } => l.now !== null).map((l) => ({ id: l.id, span: l.now }));
  const notes: Ranged[] = chapter.notes
    .filter((n): n is EditorNote & { now: ReviewSpan } => n.state === 'pending' && n.now !== null)
    .map((n) => ({ id: n.id, span: n.now }));
  const started = new Set<string>();

  return texts.map((text, block) => {
    const len = text.length;
    const dels: { id: string; from: number; to: number }[] = [];
    // Inserted text by where it goes; changes running past the end go there.
    const inserts = new Map<number, Correction[]>();
    const atEnd: Correction[] = [];
    for (const c of changes) {
      const r = inBlock(c.now, block, len);
      if (!r) continue;
      if (r.to > r.from) dels.push({ id: c.id, from: r.from, to: r.to });
      if (r.pastEnd) {
        if (block === c.now.from.block) atEnd.push(c);
      } else if (block === c.now.from.block || c.now.from.block === c.now.to.block) {
        const at = r.to;
        inserts.set(at, [...(inserts.get(at) ?? []), c]);
      }
    }
    const marks = (list: Ranged[]) =>
      list.map((m) => ({ id: m.id, r: inBlock(m.span, block, len) })).filter((m) => m.r && m.r.to > m.r.from) as {
        id: string;
        r: { from: number; to: number };
      }[];
    const lookMarks = marks(looks);
    const noteMarks = marks(notes);

    const cuts = new Set<number>([0, len]);
    for (const d of dels) cuts.add(d.from).add(d.to);
    for (const m of [...lookMarks, ...noteMarks]) cuts.add(m.r.from).add(m.r.to);
    for (const at of inserts.keys()) cuts.add(at);
    const points = [...cuts].filter((p) => p >= 0 && p <= len).sort((a, b) => a - b);

    const out: Piece[] = [];
    const startsAt = (ids: string[]) => {
      const fresh = ids.filter((id) => !started.has(id));
      fresh.forEach((id) => started.add(id));
      return fresh;
    };
    const pushInserts = (list: Correction[] | undefined, looksHere: string[], notesHere: string[]) => {
      for (const c of list ?? []) {
        if (c.after) out.push({ text: c.after, kind: 'ins', change: c.id, looks: looksHere, notes: notesHere, starts: [] });
      }
    };
    for (let i = 0; i < points.length; i++) {
      const p = points[i];
      const next = points[i + 1];
      // Text inserted here goes after what was deleted up to here.
      const around = (m: { r: { from: number; to: number } }) => m.r.from < p && m.r.to > p;
      pushInserts(
        inserts.get(p),
        lookMarks.filter(around).map((m) => m.id),
        noteMarks.filter(around).map((m) => m.id),
      );
      if (next === undefined || next === p) continue;
      const covers = (r: { from: number; to: number }) => r.from <= p && r.to >= next;
      const del = dels.find(covers);
      const lookIds = lookMarks.filter((m) => covers(m.r)).map((m) => m.id);
      const noteIds = noteMarks.filter((m) => covers(m.r)).map((m) => m.id);
      out.push({
        text: text.slice(p, next),
        kind: del ? 'del' : 'text',
        change: del?.id,
        looks: lookIds,
        notes: noteIds,
        starts: startsAt([...lookIds, ...noteIds]),
      });
    }
    for (const c of atEnd) {
      out.push({ text: '¶', kind: 'del', change: c.id, looks: [], notes: [], starts: [], para: true });
      if (c.after) out.push({ text: c.after, kind: 'ins', change: c.id, looks: [], notes: [], starts: [] });
    }
    return out;
  });
}
