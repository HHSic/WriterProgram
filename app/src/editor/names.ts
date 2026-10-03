// Where a setting card's name stands as a word: "서하" in "서하가", "서하는",
// "서하의", "서하에게" or before punctuation, but not inside "서하늘". Same
// rule as crates/core/src/cards.rs (`find_names`); both are checked against
// crates/core/tests/fixtures/names.json.

/** First syllables of the particles and endings a name may run into
 * (서하가, 서하에게, 서하처럼, 서하씨 …). Same list as `cards::NAME_ENDINGS`. */
export const NAME_ENDINGS = '은는이가을를의에엔와과도만로으랑한께처보부까마조야아여씨님네요다라란든나들뿐밖더같대쯤였예';

function isSyllable(code: number): boolean {
  return code >= 0xac00 && code <= 0xd7a3;
}

/** Whether a name ending at UTF-16 offset `end` of `text` stands as a word:
 * what follows is not a Hangul syllable, or starts a particle. */
export function endsWord(text: string, end: number): boolean {
  const next = text.codePointAt(end);
  if (next === undefined || !isSyllable(next)) return true;
  return NAME_ENDINGS.includes(String.fromCodePoint(next));
}

export interface NameMatch {
  /** UTF-16 offset in the text. */
  index: number;
  name: string;
}

/**
 * Names found by `regex` (a global alternation, longest names first) that
 * stand as words. A name running into a longer word is skipped and the search
 * goes on from the next character, as on the Rust side.
 */
export function findNames(regex: RegExp, text: string): NameMatch[] {
  const out: NameMatch[] = [];
  const next = regex.exec.bind(regex, text);
  regex.lastIndex = 0;
  for (let m = next(); m; m = next()) {
    const end = m.index + m[0].length;
    if (m[0] && endsWord(text, end)) {
      out.push({ index: m.index, name: m[0] });
      regex.lastIndex = end;
    } else {
      regex.lastIndex = m.index + ((text.codePointAt(m.index) ?? 0) > 0xffff ? 2 : 1);
    }
  }
  regex.lastIndex = 0;
  return out;
}
