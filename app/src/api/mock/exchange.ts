// 교정본 주고받기 in the browser preview. Sending keeps the chapters' text as
// sent; reading any file gives back what an editor might have done to it:
// on the sample's 3화 a particle, spacing, punctuation, a struck word, a
// coloured insertion, a highlighted phrase, notes and one paragraph the
// writer changed after sending (겹침); on other chapters a particle and a
// comma. Like the Rust side, what is still open is placed in the chapter as
// it is now, and accepting splices the text keeping its marks.

import type { JSONContent } from '@tiptap/core';
import { blocksFromJSON } from '../../editor/counts';
import type {
  Backend,
  ChangeClass,
  ChangeKind,
  ChapterReview,
  Correction,
  EditorNote,
  Exchange,
  ExchangeInfo,
  Found,
  Note,
  Review,
  ReviewLook,
  ReviewSpan,
} from '../types';
import { noteMockExchange } from './journal';
import { clone, doc, id, now, project, record } from './state';

const exchanges = new Map<string, Exchange[]>();
/** Exchange id → chapter id → paragraphs as sent. */
const sentTexts = new Map<string, Map<string, string[]>>();
const reviews = new Map<string, Review>();

const SCENE = '';
const span = (block: number, from: number, to: number): ReviewSpan => ({ from: { block, offset: from }, to: { block, offset: to } });

/** Paragraphs as the review counts them: line breaks `\n`, a scene break one character. */
function paragraphs(body: JSONContent): string[] {
  return blocksFromJSON(body).map((b) => (b.scene ? SCENE : b.lines.join('\n')));
}

// A sample of corrections --------------------------------------------------------

interface SampleChange {
  /** Text around the change in the paragraph as sent, and where the change starts in it. */
  find: string;
  at: number;
  before: string;
  after: string;
  kind: ChangeKind;
  cls: ChangeClass;
  how: Found;
  tracked?: boolean;
  /** The writer changed this paragraph after sending: `[now, as sent]`. */
  writerChanged?: [string, string];
}

const EDITOR = '김편집';
const TRACKED_AT = '2026-10-01T09:12:00+09:00';

const SAMPLE: SampleChange[] = [
  { find: '문턱에 선 남자는', at: 8, before: '는', after: '가', kind: 'replace', cls: 'wording', how: 'tracked', tracked: true },
  { find: '종이봉투만은', at: 2, before: '', after: ' ', kind: 'insert', cls: 'spacing', how: 'compared' },
  { find: '“영업, 끝났나요?”', at: 3, before: ',', after: '', kind: 'delete', cls: 'punctuation', how: 'compared' },
  { find: '적어도 지난', at: 0, before: '적어도 ', after: '', kind: 'delete', cls: 'wording', how: 'strike' },
  { find: '같은 이름이', at: 0, before: '', after: '똑', kind: 'insert', cls: 'wording', how: 'color' },
  {
    find: '그런데 들어오세요',
    at: 0,
    before: '그런데',
    after: '그래도',
    kind: 'replace',
    cls: 'wording',
    how: 'compared',
    writerChanged: ['것 같으니까', '거 같으니까'],
  },
];

const SAMPLE_LOOK = '이상하리만치';
const SAMPLE_NOTES = [
  { find: '남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다.', text: '여기서 한 박자 쉬어 가면 좋겠습니다. 문단을 나눠 보시면 어떨까요?' },
  { find: null, text: '전체적으로 문장 호흡이 좋습니다. 대사 앞뒤 쉼표만 한 번 같이 맞춰 주세요.' },
];

interface Built {
  changes: Correction[];
  looks: ReviewLook[];
  notes: EditorNote[];
  /** Paragraphs as sent, when the sample pretends the writer changed one since. */
  sent: string[];
}

/** What an editor might send back for one chapter (`texts`: paragraphs as sent). */
function sampleFor(texts: string[], next: { c: number; l: number; n: number }): Built {
  const sent = [...texts];
  const changes: Correction[] = [];
  const base = { now: null, overlap: false, author: null, date: null, state: 'pending' as const };
  const push = (block: number, from: number, s: Omit<SampleChange, 'find' | 'at'>) => {
    const text = sent[block];
    const to = from + s.before.length;
    changes.push({
      ...base,
      id: `c${++next.c}`,
      kind: s.kind,
      class: s.cls,
      before: s.before,
      after: s.after,
      lead: text.slice(Math.max(0, from - 12), from),
      trail: text.slice(to, to + 12),
      at: span(block, from, to),
      how: s.how,
      author: s.tracked ? EDITOR : null,
      date: s.tracked ? TRACKED_AT : null,
    });
  };
  for (const s of SAMPLE) {
    const block = sent.findIndex((t) => t.includes(s.find));
    if (block < 0) continue;
    if (s.writerChanged) sent[block] = sent[block].replace(s.writerChanged[0], s.writerChanged[1]);
    push(block, sent[block].indexOf(s.find) + s.at, s);
  }
  if (!changes.length) {
    // Any other chapter: a particle and a comma.
    sent.forEach((text, block) => {
      const particle = text.search(/[가-힣]는 /);
      if (particle >= 0 && !changes.some((c) => c.class === 'wording')) {
        push(block, particle + 1, { before: '는', after: '가', kind: 'replace', cls: 'wording', how: 'tracked', tracked: true });
      }
      const comma = text.indexOf(', ');
      if (comma > 0 && !changes.some((c) => c.class === 'punctuation')) {
        push(block, comma, { before: ',', after: '', kind: 'delete', cls: 'punctuation', how: 'compared' });
      }
    });
    changes.sort((a, b) => a.at.from.block - b.at.from.block || a.at.from.offset - b.at.from.offset);
  }
  const looks: ReviewLook[] = [];
  const lookBlock = sent.findIndex((t) => t.includes(SAMPLE_LOOK));
  if (lookBlock >= 0) {
    const from = sent[lookBlock].indexOf(SAMPLE_LOOK);
    looks.push({
      id: `l${++next.l}`,
      at: span(lookBlock, from, from + SAMPLE_LOOK.length),
      now: null,
      quote: SAMPLE_LOOK,
      underline: false,
      color: false,
      highlight: true,
    });
  }
  const notes: EditorNote[] = [];
  if (changes.some((c) => c.how === 'strike')) {
    for (const n of SAMPLE_NOTES) {
      const block = n.find ? sent.findIndex((t) => t.includes(n.find)) : -1;
      if (n.find && block < 0) continue;
      const from = block >= 0 ? sent[block].indexOf(n.find!) : 0;
      notes.push({
        id: `n${++next.n}`,
        text: n.text,
        author: EDITOR,
        date: '2026-10-01 10:20',
        at: block >= 0 ? span(block, from, from + n.find!.length) : null,
        now: null,
        quote: n.find ?? '',
        state: 'pending',
        memo: null,
      });
    }
  }
  return { changes, looks, notes, sent };
}

// Placing in the chapter as it is now ------------------------------------------------

/** A paragraph as sent with the changes accepted in it so far, and how far each offset moved. */
function expected(sent: string, accepted: Correction[]): { text: string; shift: (offset: number) => number } {
  const list = [...accepted].sort((a, b) => a.at.from.offset - b.at.from.offset);
  let text = '';
  let at = 0;
  for (const c of list) {
    text += sent.slice(at, c.at.from.offset) + c.after;
    at = c.at.to.offset;
  }
  text += sent.slice(at);
  const shift = (offset: number) =>
    offset +
    list
      .filter((c) => c.at.to.offset <= offset && !(c.at.from.offset === offset && c.before === ''))
      .reduce((n, c) => n + c.after.length - c.before.length, 0);
  return { text, shift };
}

/** Sets `now` (and 겹침) of everything still open from the chapter's text now. */
function place(chapter: ChapterReview, sent: string[], current: string[] | null) {
  chapter.gone = current === null;
  const where = (at: ReviewSpan | null): ReviewSpan | null => {
    if (!at || !current || at.from.block !== at.to.block) return null;
    const block = at.from.block;
    const accepted = chapter.changes.filter((c) => c.state === 'accepted' && c.at.from.block === block);
    const exp = expected(sent[block] ?? '', accepted);
    if (current[block] !== exp.text) return null;
    return span(block, exp.shift(at.from.offset), exp.shift(at.to.offset));
  };
  for (const c of chapter.changes) {
    if (c.state !== 'pending') continue;
    c.now = where(c.at);
    c.overlap = c.now === null;
  }
  for (const l of chapter.looks) l.now = where(l.at);
  for (const n of chapter.notes) if (n.state === 'pending') n.now = where(n.at);
}

function placeAll(root: string, review: Review) {
  const p = project(root);
  const sent = sentTexts.get(review.exchange);
  for (const chapter of review.chapters) {
    const d = p.docs.get(chapter.docId);
    place(chapter, sent?.get(chapter.docId) ?? [], d ? paragraphs(d.body) : null);
  }
}

/** Replaces UTF-16 `from..to` of a paragraph with `text`; new text takes the marks of the first character replaced (or the one before). */
function splice(node: JSONContent, from: number, to: number, text: string) {
  const cells: { ch: string; marks?: JSONContent['marks'] }[] = [];
  for (const inline of node.content ?? []) {
    if (inline.type === 'hardBreak') cells.push({ ch: '\n' });
    else for (const ch of (inline.text ?? '').split('')) cells.push({ ch, marks: inline.marks });
  }
  const marks = (cells[from]?.ch !== '\n' ? cells[from]?.marks : undefined) ?? cells[from - 1]?.marks;
  cells.splice(from, to - from, ...text.split('').map((ch) => ({ ch, marks })));
  const content: JSONContent[] = [];
  for (const cell of cells) {
    if (cell.ch === '\n') {
      content.push({ type: 'hardBreak' });
      continue;
    }
    const last = content[content.length - 1];
    if (last?.type === 'text' && JSON.stringify(last.marks ?? null) === JSON.stringify(cell.marks ?? null)) last.text += cell.ch;
    else content.push({ type: 'text', text: cell.ch, ...(cell.marks ? { marks: cell.marks } : {}) });
  }
  node.content = content.length ? content : undefined;
}

const OVERLAP = '보낸 뒤 작가가 고친 문단이라 직접 고쳐 주세요';

export const exchangeMethods = {
  async exchangeSend(root, items, opts, _format, kind, dest, perDoc) {
    const p = project(root);
    const name = (path: string) => path.split('\\').pop() ?? path;
    const files = perDoc ? items.map((i) => `${i.fileName}.${kind}`) : [name(dest)];
    const ex: Exchange = {
      id: id(),
      created: now(),
      kind,
      files,
      sceneBreak: opts.sceneBreak,
      chapters: items.map((i, n) => ({
        docId: i.docId,
        title: doc(p, i.docId).meta.title,
        heading: i.heading,
        file: perDoc ? files[n] : files[0],
        fingerprint: id(),
      })),
      received: [],
    };
    sentTexts.set(ex.id, new Map(items.map((i) => [i.docId, paragraphs(doc(p, i.docId).body)])));
    exchanges.set(root, [ex, ...(exchanges.get(root) ?? [])]);
    noteMockExchange(root);
    return clone(ex);
  },
  async exchangeList(root) {
    return (exchanges.get(root) ?? []).map(
      (ex): ExchangeInfo => ({
        ...clone(ex),
        pending: reviews.get(ex.id)?.chapters.flatMap((c) => c.changes).filter((c) => c.state === 'pending').length ?? null,
      }),
    );
  },
  async exchangeRead(root, exchangeId, path) {
    const ex = (exchanges.get(root) ?? []).find((e) => e.id === exchangeId);
    const sent = sentTexts.get(exchangeId);
    if (!ex || !sent) throw '보낸 원고 기록을 찾을 수 없음';
    if (!/\.(hwpx|docx)$/i.test(path)) throw '한글(hwpx)이나 Word(docx) 파일만 읽을 수 있습니다';
    const name = path.split('\\').pop() ?? path;
    const at = now();
    const next = { c: 0, l: 0, n: 0 };
    const review: Review = {
      exchange: ex.id,
      file: name,
      stored: `received/${Date.now()}.${ex.kind}`,
      at,
      chapters: ex.chapters.map((c): ChapterReview => {
        const built = sampleFor(sent.get(c.docId) ?? [], next);
        // The sample's 겹침 is a paragraph that was different when it went out.
        sent.set(c.docId, built.sent);
        return { docId: c.docId, title: c.title, found: true, gone: false, edited: false, changes: built.changes, looks: built.looks, notes: built.notes };
      }),
    };
    placeAll(root, review);
    const p = project(root);
    for (const chapter of review.chapters) {
      const d = p.docs.get(chapter.docId);
      chapter.edited = !!d && JSON.stringify(paragraphs(d.body)) !== JSON.stringify(sent.get(chapter.docId));
    }
    ex.received.push({ at, name, stored: review.stored, fingerprint: id() });
    reviews.set(ex.id, review);
    noteMockExchange(root);
    return clone(review);
  },
  async exchangeReview(root, exchangeId) {
    const review = reviews.get(exchangeId);
    if (!review) return null;
    placeAll(root, review);
    return clone(review);
  },
  async exchangeApply(root, exchangeId, decisions) {
    const p = project(root);
    const review = reviews.get(exchangeId);
    if (!review) throw '먼저 교정본을 불러와 주세요';
    placeAll(root, review);
    const accept = new Set(decisions.accept ?? []);
    const reject = new Set(decisions.reject ?? []);
    const keep = new Set(decisions.keepNotes ?? []);
    const drop = new Set(decisions.dropNotes ?? []);
    const classes = new Set(decisions.acceptClasses ?? []);
    const out = { docs: [] as string[], accepted: [] as string[], rejected: [] as string[], skipped: [] as { id: string; reason: string }[], memos: [] as string[] };
    for (const chapter of review.chapters) {
      const d = p.docs.get(chapter.docId);
      for (const c of chapter.changes) {
        if (c.state === 'pending' && reject.has(c.id)) {
          c.state = 'rejected';
          out.rejected.push(c.id);
        }
      }
      for (const n of chapter.notes) {
        if (n.state === 'pending' && drop.has(n.id)) n.state = 'rejected';
      }
      const wanted = chapter.changes.filter((c) => c.state === 'pending' && (accept.has(c.id) || classes.has(c.class)));
      const kept = chapter.notes.filter((n) => n.state === 'pending' && keep.has(n.id));
      if (!d) {
        for (const x of [...wanted, ...kept]) out.skipped.push({ id: x.id, reason: '회차가 지워져서 반영할 수 없음' });
        continue;
      }
      const placeable = wanted.filter((c) => c.now);
      for (const c of wanted) if (!c.now) out.skipped.push({ id: c.id, reason: OVERLAP });
      if (placeable.length) {
        record(p, d, 'before-corrections', `교정 반영 전 · ${review.file}`);
        // Right to left, so earlier places stay right.
        const order = [...placeable].sort((a, b) => b.now!.from.block - a.now!.from.block || b.now!.from.offset - a.now!.from.offset);
        for (const c of order) {
          const node = d.body.content?.[c.now!.from.block];
          if (node) splice(node, c.now!.from.offset, c.now!.to.offset, c.after);
          c.state = 'accepted';
          out.accepted.push(c.id);
        }
        d.modified = now();
        out.docs.push(d.meta.id);
      }
      for (const n of kept) {
        const at = now();
        const memo: Note = {
          id: id(),
          anchor: 'doc',
          target: d.meta.id,
          quote: n.quote,
          text: `${n.author || '편집자'}: ${n.text}`,
          replies: [],
          tags: [],
          done: false,
          created: at,
          updated: at,
        };
        p.notes.set(memo.id, memo);
        n.state = 'accepted';
        n.memo = memo.id;
        out.memos.push(memo.id);
      }
    }
    placeAll(root, review);
    if (out.docs.length) noteMockExchange(root);
    return { ...out, review: clone(review) };
  },
} satisfies Partial<Backend>;
