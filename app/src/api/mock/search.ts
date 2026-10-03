// Find and replace across the project.

import { buildRegex } from '../../editor/search';
import type { Backend, SearchMatch, SnapshotInfo } from '../types';
import { clone, project, record } from './state';

export const searchMethods = {
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
    const locked: string[] = [];
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
      if (d.meta.locked) {
        locked.push(docId);
        continue;
      }
      const snapshot = record(p, d, 'before-replace', '');
      d.body = next;
      replaced += count;
      out.push({ docId, count, snapshot });
    }
    return { replaced, docs: out, locked };
  },
} satisfies Partial<Backend>;
