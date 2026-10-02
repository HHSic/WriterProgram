// Sentences of a paragraph for 소리 내어 읽기: the pieces read one at a time
// and lit up while they are read (editor/readAloud.ts).

/** A sentence as UTF-16 offsets [start, end) into the text, without the
 * spaces around it. */
export interface Span {
  start: number;
  end: number;
}

/** Marks that can end a sentence. A run of them (?!, ……, ...) ends it once. */
const ENDS = '.!?…‥。！？';

/** Closing quotes and brackets that stay with the sentence they close. */
const CLOSERS = '"\'”’」』〉》)）]］';

/** After a closing quote these carry the same sentence on:
 * "가자!" 하고 외쳤다. / "정말?" 이라며 웃었다. */
const QUOTE_GOES_ON = /^\s*(?:하고|하며|하면서|하는|하자|하니|하던|라고|라며|라면서|라는|이라고|이라며|이라는)/;

const SPACE = /\s/;

/**
 * Splits a paragraph's text into sentences. A sentence ends at . ! ? … (or a
 * run of them) followed by a space or the end of the text, taking any closing
 * quotes with it, so dialogue stays one piece: "어디 가?" is one sentence.
 * A line break ("\n", Shift+Enter) always ends one. A full stop not followed
 * by a space (3.5, 1.2배) ends nothing.
 */
export function splitSentences(text: string): Span[] {
  const out: Span[] = [];
  let start = 0;

  const close = (end: number, next: number) => {
    let s = start;
    let e = end;
    while (s < e && SPACE.test(text[s])) s += 1;
    while (e > s && SPACE.test(text[e - 1])) e -= 1;
    if (e > s) out.push({ start: s, end: e });
    start = next;
  };

  let i = 0;
  while (i < text.length) {
    const ch = text[i];
    if (ch === '\n') {
      close(i, i + 1);
      i += 1;
      continue;
    }
    if (!ENDS.includes(ch)) {
      i += 1;
      continue;
    }
    let j = i + 1;
    while (j < text.length && ENDS.includes(text[j])) j += 1;
    let quoted = false;
    while (j < text.length && CLOSERS.includes(text[j])) {
      j += 1;
      quoted = true;
    }
    const atEnd = j >= text.length || SPACE.test(text[j]);
    if (atEnd && !(quoted && QUOTE_GOES_ON.test(text.slice(j, j + 8)))) close(j, j);
    i = j;
  }
  close(text.length, text.length);
  return out;
}
