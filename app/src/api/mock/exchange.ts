// 교정본 주고받기 in the browser preview: sending keeps a record in memory,
// reading any file gives a sample review of the first chapter sent, and
// accepting a change replaces its text in that paragraph.

import type { JSONContent } from '@tiptap/core';
import type { Backend, Correction, Exchange, ExchangeInfo, Review } from '../types';
import { clone, doc, id, now, project, record } from './state';

const exchanges = new Map<string, Exchange[]>();
const reviews = new Map<string, Review>();

const span = (block: number, from: number, to: number) => ({ from: { block, offset: from }, to: { block, offset: to } });

/** Plain text of one paragraph of a body. */
function paragraphText(body: JSONContent, block: number): string {
  const node = body.content?.[block];
  return (node?.content ?? []).map((n) => n.text ?? '').join('');
}

/** A sample of what an editor sends back, built on the chapter's first paragraph. */
function sampleChanges(text: string): Correction[] {
  const base = { lead: '', trail: '', now: null, overlap: false, author: null, date: null, state: 'pending' as const };
  const out: Correction[] = [];
  const space = text.indexOf(' ');
  if (space > 0) {
    out.push({ ...base, id: 'c1', kind: 'delete', class: 'spacing', before: ' ', after: '', at: span(0, space, space + 1), now: span(0, space, space + 1), how: 'compared' });
  }
  const end = text.length;
  out.push({ ...base, id: 'c2', kind: 'insert', class: 'punctuation', before: '', after: '.', at: span(0, end, end), now: span(0, end, end), how: 'compared' });
  out.push({
    ...base,
    id: 'c3',
    kind: 'insert',
    class: 'wording',
    before: '',
    after: '그날 ',
    at: span(0, 0, 0),
    now: span(0, 0, 0),
    how: 'tracked',
    author: '김편집',
    date: '2026-10-01T09:00:00Z',
  });
  return out;
}

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
    exchanges.set(root, [ex, ...(exchanges.get(root) ?? [])]);
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
    const p = project(root);
    const ex = (exchanges.get(root) ?? []).find((e) => e.id === exchangeId);
    if (!ex) throw '보낸 원고 기록을 찾을 수 없음';
    const name = path.split('\\').pop() ?? path;
    const at = now();
    const review: Review = {
      exchange: ex.id,
      file: name,
      stored: `received/${Date.now()}.${ex.kind}`,
      at,
      chapters: ex.chapters.map((c, n) => {
        const text = n === 0 ? paragraphText(doc(p, c.docId).body, 0) : '';
        return {
          docId: c.docId,
          title: c.title,
          found: n === 0,
          gone: false,
          edited: false,
          changes: n === 0 && text ? sampleChanges(text) : [],
          looks: [],
          notes:
            n === 0 && text
              ? [{ id: 'n1', text: '첫 문장이 조금 깁니다.', author: '김편집', date: '2026-10-01 10:00', at: span(0, 0, text.length), now: span(0, 0, text.length), quote: text, state: 'pending', memo: null }]
              : [],
        };
      }),
    };
    ex.received.push({ at, name, stored: review.stored, fingerprint: id() });
    reviews.set(ex.id, review);
    return clone(review);
  },
  async exchangeReview(_root, exchangeId) {
    const review = reviews.get(exchangeId);
    return review ? clone(review) : null;
  },
  async exchangeApply(root, exchangeId, decisions) {
    const p = project(root);
    const review = reviews.get(exchangeId);
    if (!review) throw '먼저 교정본을 불러와 주세요';
    const accept = new Set(decisions.accept ?? []);
    const reject = new Set(decisions.reject ?? []);
    const classes = new Set(decisions.acceptClasses ?? []);
    const out = { docs: [] as string[], accepted: [] as string[], rejected: [] as string[], skipped: [] as { id: string; reason: string }[], memos: [] as string[] };
    for (const chapter of review.chapters) {
      const d = p.docs.get(chapter.docId);
      const wanted = chapter.changes.filter((c) => c.state === 'pending' && (accept.has(c.id) || classes.has(c.class)));
      for (const c of chapter.changes) {
        if (c.state === 'pending' && reject.has(c.id)) {
          c.state = 'rejected';
          out.rejected.push(c.id);
        }
      }
      if (!d || !wanted.length) continue;
      record(p, d, 'before-corrections', `교정 반영 전 · ${review.file}`);
      // Right to left, so earlier offsets stay right.
      for (const c of [...wanted].sort((a, b) => b.at.from.offset - a.at.from.offset)) {
        const node = d.body.content?.[c.at.from.block];
        const text = paragraphText(d.body, c.at.from.block);
        if (!node || !c.now) {
          out.skipped.push({ id: c.id, reason: '보낸 뒤 작가가 고친 문단이라 직접 고쳐 주세요' });
          continue;
        }
        const next = text.slice(0, c.now.from.offset) + c.after + text.slice(c.now.to.offset);
        node.content = next ? [{ type: 'text', text: next }] : undefined;
        c.state = 'accepted';
        out.accepted.push(c.id);
      }
      out.docs.push(d.meta.id);
    }
    return { ...out, review: clone(review) };
  },
} satisfies Partial<Backend>;
