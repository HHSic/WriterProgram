import { getSchema } from '@tiptap/core';
import { describe, expect, it } from 'vitest';
import { manuscriptExtensions } from './extensions';
import { koreanVoices, pickVoice, readingPlan, type ReadStep } from './readAloud';

const schema = getSchema(manuscriptExtensions('◆', { onCardOpen: () => {}, onNoteOpen: () => {}, onNoteAdd: () => {} }));

const para = (text: string) => schema.node('paragraph', null, text ? [schema.text(text)] : []);
const scene = () => schema.node('sceneBreak');

// doc: "가. 나." (pos 0..), scene break, "다. 라."
const doc = schema.node('doc', null, [para('가는 갔다. 나는 남았다.'), scene(), para('다시 봄. 라일락.')]);

const said = (steps: ReadStep[]) => steps.map((s) => s.text ?? '|');

/** Document position of `needle` (in the first text block that has it). */
function at(needle: string, offset = 0): number {
  let found = -1;
  doc.descendants((node, pos) => {
    if (found >= 0 || !node.isTextblock) return found < 0;
    const i = node.textContent.indexOf(needle);
    if (i >= 0) found = pos + 1 + i + offset;
    return false;
  });
  if (found < 0) throw new Error(needle);
  return found;
}

describe('readingPlan', () => {
  it('reads the whole chapter from the start, a scene break as a pause', () => {
    expect(said(readingPlan(doc, 0, 0))).toEqual(['가는 갔다.', '나는 남았다.', '|', '다시 봄.', '라일락.']);
  });

  it('starts at the sentence the cursor is in', () => {
    expect(said(readingPlan(doc, at('남았다'), at('남았다')))).toEqual(['나는 남았다.', '|', '다시 봄.', '라일락.']);
  });

  it('skips a sentence the cursor is right after', () => {
    const after = at('갔다.', 3);
    expect(said(readingPlan(doc, after, after))).toEqual(['나는 남았다.', '|', '다시 봄.', '라일락.']);
  });

  it('has nothing to read from the very end', () => {
    expect(readingPlan(doc, doc.content.size, doc.content.size)).toEqual([]);
  });

  it('reads just the selected text, cut at its edges', () => {
    expect(said(readingPlan(doc, at('갔다'), at('남았다', 2)))).toEqual(['갔다.', '나는 남았']);
  });

  it('takes a scene break inside the selection as a pause', () => {
    expect(said(readingPlan(doc, at('나는'), at('봄.', 2)))).toEqual(['나는 남았다.', '|', '다시 봄.']);
  });

  it('drops a pause at the start or end', () => {
    expect(said(readingPlan(doc, at('남았다.', 4), at('봄.', 2)))).toEqual(['다시 봄.']);
  });

  it('gives document ranges of the sentences', () => {
    const [first] = readingPlan(doc, 0, 0);
    expect(doc.textBetween(first.from, first.to)).toBe('가는 갔다.');
  });

  it('leaves out bits with nothing to say', () => {
    const d = schema.node('doc', null, [para('……'), para('“…”'), para('끝.')]);
    expect(said(readingPlan(d, 0, 0))).toEqual(['끝.']);
  });

  it('reads a line break as a sentence end', () => {
    const d = schema.node('doc', null, [
      schema.node('paragraph', null, [schema.text('첫 줄'), schema.node('hardBreak'), schema.text('둘째 줄')]),
    ]);
    expect(said(readingPlan(d, 0, 0))).toEqual(['첫 줄', '둘째 줄']);
  });
});

describe('voices', () => {
  const voices = [
    { lang: 'en-US', voiceURI: 'david', localService: true },
    { lang: 'ko-KR', voiceURI: 'google-ko', localService: false },
    { lang: 'ko-KR', voiceURI: 'heami', localService: true },
  ];

  it('keeps Korean voices, the ones on the device first', () => {
    expect(koreanVoices(voices).map((v) => v.voiceURI)).toEqual(['heami', 'google-ko']);
  });

  it('picks the chosen voice, or the first Korean one', () => {
    expect(pickVoice(voices, 'google-ko')?.voiceURI).toBe('google-ko');
    expect(pickVoice(voices, '')?.voiceURI).toBe('heami');
    expect(pickVoice(voices, 'gone')?.voiceURI).toBe('heami');
  });

  it('never falls back to another language', () => {
    expect(pickVoice([voices[0]], '')).toBeNull();
    expect(pickVoice([voices[0]], 'david')).toBeNull();
  });
});
