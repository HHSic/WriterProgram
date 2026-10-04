// 맞춤법 검사 in the browser preview: a handful of common misspellings stand
// in for the dictionary, which only the desktop app fetches.

import type { Backend, SearchMatch, SpellIssue, SpellStatus, SpellWords } from '../types';
import { project } from './state';

/** Misspelling → what was meant. */
const WRONG: Record<string, string[]> = {
  되요: ['돼요', '되어요'],
  몇일: ['며칠'],
  웬지: ['왠지'],
  금새: ['금세', '그새'],
  설겆이: ['설거지'],
  희안하다: ['희한하다'],
  역활: ['역할'],
  오랫만에: ['오랜만에'],
  어떻해: ['어떡해'],
  베게: ['베개'],
  할께: ['할게'],
  멱었다: ['먹었다'],
};

const NOTICE = `맞춤법 사전: hunspell-dict-ko 0.7.94
https://github.com/spellcheck-ko/hunspell-dict-ko
(미리보기에서는 흉내만 냅니다)`;

let installed = false;
const lists = new Map<string, string[]>();
let device: string[] = [];

function words(root: string): SpellWords {
  return { project: [...(lists.get(root) ?? [])], device: [...device] };
}

function check(text: string, known: Set<string>): SpellIssue[] {
  const out: SpellIssue[] = [];
  for (const m of text.matchAll(/[\p{L}\p{N}]+/gu)) {
    const word = m[0];
    if (m.index === undefined || known.has(word) || !WRONG[word]) continue;
    out.push({ start: m.index, end: m.index + word.length, word });
  }
  return out;
}

function status(): SpellStatus {
  return {
    installed: installed ? { version: '0.7.94', folder: '(미리보기)', source: 'https://github.com/spellcheck-ko/hunspell-dict-ko', notice: NOTICE } : null,
    version: '0.7.94',
    downloadBytes: 897_723,
    installedBytes: 13_996_687,
    source: 'https://github.com/spellcheck-ko/hunspell-dict-ko',
    notice: NOTICE,
  };
}

export const spellMethods = {
  async spellStatus() {
    return status();
  },
  async spellDownload() {
    await new Promise((r) => setTimeout(r, 600));
    installed = true;
    return status().installed!;
  },
  async spellRemove() {
    installed = false;
  },
  async spellCheck(texts, known) {
    if (!installed) throw '맞춤법 사전이 없음';
    const k = new Set(known);
    return texts.map((t) => check(t, k));
  },
  async spellSuggest(word) {
    return WRONG[word] ?? [];
  },
  async spellCheckWork(root, known, docIds) {
    if (!installed) throw '맞춤법 사전이 없음';
    const p = project(root);
    const k = new Set(known);
    const ids = docIds ?? [...p.parts.flatMap((x) => x.docs), ...p.planning];
    const docs: { docId: string; locked: boolean; issues: SearchMatch[] }[] = [];
    let total = 0;
    let skipped = 0;
    for (const docId of ids) {
      const d = p.docs.get(docId);
      if (!d) continue;
      if (d.broken) {
        skipped += 1;
        continue;
      }
      const issues: SearchMatch[] = [];
      (d.body.content ?? []).forEach((node, block) => {
        if (node.type !== 'paragraph') return;
        const text = (node.content ?? []).map((i) => (i.type === 'hardBreak' ? '\n' : (i.text ?? ''))).join('');
        for (const i of check(text, k)) {
          issues.push({
            block,
            start: i.start,
            end: i.end,
            before: text.slice(Math.max(0, i.start - 24), i.start),
            text: i.word,
            after: text.slice(i.end, i.end + 24),
          });
        }
      });
      if (issues.length) {
        docs.push({ docId, locked: !!d.meta.locked, issues });
        total += issues.length;
      }
    }
    return { docs, total, truncated: false, skipped };
  },
  async spellWords(root) {
    return words(root);
  },
  async spellWordAdd(root, word, place) {
    const w = word.trim();
    if (!w || /\s/.test(w)) throw '띄어 쓰지 않은 낱말 하나만 넣을 수 있음';
    if (place === 'device') device = [...new Set([...device, w])].sort();
    else lists.set(root, [...new Set([...(lists.get(root) ?? []), w])].sort());
    return words(root);
  },
  async spellWordRemove(root, word, place) {
    if (place === 'device') device = device.filter((x) => x !== word);
    else lists.set(root, (lists.get(root) ?? []).filter((x) => x !== word));
    return words(root);
  },
} satisfies Partial<Backend>;
