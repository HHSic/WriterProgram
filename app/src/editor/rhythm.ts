// 길이 보기 in 해석 모드: narration sentences of 60 characters or more, and
// stretches of four or more short ones (20 or fewer), marked in the text so
// the writer sees where the chapter breathes long and where it stops short.
// The same counts as the kfiction check (F13) and writer-mcp's paragraphs;
// why a sentence is that long is for the reading (작품 해석) to say.

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { blockText, rangeInBlock } from './search';
import { splitSentences } from './sentences';

export const LONG = 60;
export const SHORT = 20;
export const SHORT_RUN = 4;

/** A narration sentence: its block and UTF-16 offsets there. */
export interface RhythmSpan {
  block: number;
  start: number;
  end: number;
  length: number;
}

export interface RhythmCount {
  sentences: number;
  mean: number;
  long: RhythmSpan[];
  /** Stretches of short sentences in a row. */
  shortRuns: RhythmSpan[][];
}

/** Dialogue marks, as the kfiction check reads them (plugins/kfiction/lib/text.js). Single
 * quotes stay narration: '수선 세탁'이라고 쓰인 간판. */
const CLOSE: Record<string, string> = { '“': '”', '「': '」', '『': '』', '"': '"' };

/** Offsets in `text` inside quotation marks (dialogue). */
function quoted(text: string): [number, number][] {
  const out: [number, number][] = [];
  let i = 0;
  while (i < text.length) {
    const open = text[i];
    if (CLOSE[open]) {
      const close = text.indexOf(CLOSE[open], i + 1);
      const end = close < 0 ? text.length : close + 1;
      out.push([i, end]);
      i = end;
    } else i += 1;
  }
  return out;
}

/** Narration sentences of the chapter, in order; null where dialogue breaks the flow. */
function narration(doc: PmNode): (RhythmSpan | null)[] {
  const flow: (RhythmSpan | null)[] = [];
  let pos = 0;
  doc.forEach((node, _offset, block) => {
    if (node.isTextblock) {
      const { text } = blockText(node, pos);
      const quotes = quoted(text);
      if (quotes.length) flow.push(null);
      for (const s of splitSentences(text)) {
        const inQuote = quotes.some(([a, b]) => s.start < b && a < s.end);
        if (inQuote) continue;
        const length = [...text.slice(s.start, s.end)].length;
        flow.push({ block, start: s.start, end: s.end, length });
      }
    } else flow.push(null);
    pos += node.nodeSize;
  });
  return flow;
}

export function countRhythm(doc: PmNode): RhythmCount {
  const flow = narration(doc);
  const spans = flow.filter((s): s is RhythmSpan => s !== null);
  const total = spans.reduce((n, s) => n + s.length, 0);
  const shortRuns: RhythmSpan[][] = [];
  let run: RhythmSpan[] = [];
  const close = () => {
    if (run.length >= SHORT_RUN) shortRuns.push(run);
    run = [];
  };
  for (const s of flow) {
    if (s && s.length <= SHORT) run.push(s);
    else close();
  }
  close();
  return {
    sentences: spans.length,
    mean: spans.length ? Math.round(total / spans.length) : 0,
    long: spans.filter((s) => s.length >= LONG),
    shortRuns,
  };
}

const rhythmKey = new PluginKey<{ on: boolean; set: DecorationSet }>('rhythm');

function decorate(doc: PmNode): DecorationSet {
  const { long, shortRuns } = countRhythm(doc);
  const decos: Decoration[] = [];
  const mark = (s: RhythmSpan, cls: string, title: string) => {
    const range = rangeInBlock(doc, s.block, s.start, s.end);
    if (range) decos.push(Decoration.inline(range.from, range.to, { class: cls, title }));
  };
  for (const s of long) mark(s, 'rhythm-long', `${s.length}자`);
  for (const r of shortRuns) for (const s of r) mark(s, 'rhythm-short', `짧은 문장 ${r.length}개가 이어짐`);
  return DecorationSet.create(doc, decos);
}

export const RhythmHighlight = Extension.create({
  name: 'rhythmHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<{ on: boolean; set: DecorationSet }>({
        key: rhythmKey,
        state: {
          init: () => ({ on: false, set: DecorationSet.empty }),
          apply(tr, prev) {
            const meta = tr.getMeta(rhythmKey) as boolean | undefined;
            const on = meta ?? prev.on;
            if (!on) return { on, set: DecorationSet.empty };
            if (meta !== undefined || tr.docChanged) return { on, set: decorate(tr.doc) };
            return prev;
          },
        },
        props: {
          decorations: (state) => rhythmKey.getState(state)?.set,
        },
      }),
    ];
  },
});

/** Shows or hides 길이 보기. */
export function setRhythmMarks(editor: Editor, on: boolean) {
  if (editor.isDestroyed || rhythmKey.getState(editor.state)?.on === on) return;
  editor.view.dispatch(editor.state.tr.setMeta(rhythmKey, on).setMeta('addToHistory', false));
}
