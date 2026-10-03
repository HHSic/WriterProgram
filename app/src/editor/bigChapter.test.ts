// A 1.4 million character chapter in one piece (가져오기 with "나누지 않기"):
// typing one character must not make the decorations look at the whole
// chapter again (docs/feature-gap-report.md 3.3). Timings are printed.

import { describe, expect, it } from 'vitest';
import { getSchema, type AnyExtension, type Editor } from '@tiptap/core';
import type { Node as PmNode, Schema } from '@tiptap/pm/model';
import { EditorState, type Plugin } from '@tiptap/pm/state';
import type { CardSummary, IndentRules } from '../api/types';
import { CardHighlight, nameIndex, setCardNames } from './cards';
import { manuscriptExtensions } from './extensions';
import { IndentRulesExtension, setIndentRules } from './indent';

const RULES: IndentRules = { chapterFirst: true, afterScene: true, margined: true, dialogue: true, dialogueHang: false };
const CARDS = ['서하', '윤서하', '강윤석', '달빛 서점', '이도윤', '한별'].map(
  (name, i) => ({ id: `c${i}`, cardType: 'person', name, aliases: [], highlight: true, summary: '' }) as CardSummary,
);

/** About `chars` characters in paragraphs of ~70, a scene break every 200. */
export function bigChapter(schema: Schema, chars: number): PmNode {
  const lines = [
    '셔터를 반쯤 내렸을 때 종이 울렸다. 서하는 고개를 들지 않고 장부를 덮었다.',
    '“오늘은 끝났어요.” 서하가 말했지만 문 앞의 그림자는 움직이지 않았다.',
    '달빛 서점의 간판은 비에 젖어 반쯤 꺼져 있었고, 강윤석은 우산을 접었다.',
    '그날 밤 한별은 오래된 대여 카드 한 장을 꺼내 책상 위에 올려 두었다.',
  ];
  const blocks: PmNode[] = [];
  let total = 0;
  for (let i = 0; total < chars; i += 1) {
    if (i > 0 && i % 200 === 0) {
      blocks.push(schema.nodes.sceneBreak.create());
      continue;
    }
    const text = lines[i % lines.length];
    total += text.length;
    blocks.push(schema.nodes.paragraph.create(null, schema.text(text)));
  }
  return schema.nodes.doc.create(null, blocks);
}

/** The ProseMirror plugins of a Tiptap extension that only reads its options. */
function pluginsOf(ext: AnyExtension): Plugin[] {
  const make = ext.config.addProseMirrorPlugins as ((this: unknown) => Plugin[]) | undefined;
  return make ? make.call({ options: ext.options, name: ext.name, editor: null }) : [];
}

/** Just enough of an Editor for setIndentRules / setCardNames. */
function fakeEditor(state: EditorState) {
  const editor = {
    isDestroyed: false,
    state,
    view: { dispatch: (tr: Parameters<EditorState['apply']>[0]) => (editor.state = editor.state.apply(tr)) },
  };
  return editor;
}

function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}

/** Median time of typing one character in the middle of the chapter. */
function typingTime(state: EditorState, times = 15): number {
  const samples: number[] = [];
  let s = state;
  const mid = Math.floor(s.doc.content.size / 2);
  const at = s.doc.resolve(mid).start();
  for (let i = 0; i < times; i += 1) {
    const tr = s.tr.insertText('가', at);
    const t0 = performance.now();
    s = s.apply(tr);
    samples.push(performance.now() - t0);
  }
  return median(samples);
}

describe('a 1.4 million character chapter', () => {
  const schema = getSchema(manuscriptExtensions('◆', { onCardOpen() {}, onNoteOpen() {}, onNoteAdd() {} }));
  const doc = bigChapter(schema, 1_400_000);

  it('types without looking at the whole chapter again', () => {
    const indent = fakeEditor(EditorState.create({ doc, plugins: pluginsOf(IndentRulesExtension) }));
    let t0 = performance.now();
    setIndentRules(indent as unknown as Editor, RULES);
    const indentFull = performance.now() - t0;
    const indentTyping = typingTime(indent.state);

    const cards = fakeEditor(EditorState.create({ doc, plugins: pluginsOf(CardHighlight) }));
    t0 = performance.now();
    setCardNames(cards as unknown as Editor, nameIndex(CARDS));
    const cardsFull = performance.now() - t0;
    const cardsTyping = typingTime(cards.state);

    console.log(
      `1.4M chars, ${doc.childCount} blocks: indent full ${indentFull.toFixed(1)}ms, typing ${indentTyping.toFixed(2)}ms; ` +
        `card names full ${cardsFull.toFixed(1)}ms, typing ${cardsTyping.toFixed(2)}ms`,
    );
    // Before: about 5 s a keystroke for the indent and 8 s to set up either
    // (DecorationSet.create and map are quadratic in the number of blocks).
    expect(indentTyping).toBeLessThan(100);
    expect(cardsTyping).toBeLessThan(100);
    expect(indentFull).toBeLessThan(1000);
    expect(cardsFull).toBeLessThan(2000);
  }, 60_000);
});
