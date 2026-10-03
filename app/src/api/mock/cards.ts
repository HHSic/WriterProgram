// Cards (인물, 장소, 용어 …), their kinds, and where their names appear.

import type { JSONContent } from '@tiptap/core';
import { findNames } from '../../editor/names';
import type { Appearance, Backend, Card, CardType, SearchMatch, TrashItem } from '../types';
import { cardSummary, clone, doc, id, now, project, type MockDoc } from './state';

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

export const cardMethods = {
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
      meta: { id: cardId, title: c.name, synopsis: '', status: 'draft', target: null, locked: false, created: c.created },
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
        for (const m of findNames(re, text)) {
          count += 1;
          if (samples.length < 3) {
            samples.push({
              block,
              start: m.index,
              end: m.index + m.name.length,
              before: text.slice(Math.max(0, m.index - 24), m.index),
              text: m.name,
              after: text.slice(m.index + m.name.length, m.index + m.name.length + 24),
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
            .filter((d) => paragraphTexts(doc(p, d).body).some(({ text }) => findNames(re, text).length > 0)).length
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
} satisfies Partial<Backend>;
