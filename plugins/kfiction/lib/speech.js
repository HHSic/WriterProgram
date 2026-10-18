// 대사의 말 단계를 마지막 어절의 끝 글자로 어림한다(형태소 분석 없음).
// WriterProgram 저장소의 crates/revise/src/speech.rs 가 형태소로 하는 판정을 문자 규칙으로 옮긴 것이다.
// 어림이므로 화면에는 "참고"로만 보인다. "바다." 같은 명사 문장을 해라체로 읽는 식의 오판이 있다.

import { lastWord, sentences } from './text.js';

export const LEVELS = ['하십시오체', '해요체', '하오체', '하게체', '해체', '해라체'];
export const POLITE = new Set(['하십시오체', '해요체', '하오체']);

const FRAGMENTS = new Set(['네', '예', '응', '음', '흠', '아', '어', '오', '와', '뭐', '왜', '저기', '저', '나', '너', '야']);

const jong = (ch) => {
  const c = ch?.charCodeAt(0) ?? 0;
  return c >= 0xac00 && c <= 0xd7a3 ? (c - 0xac00) % 28 : -1;
};
const vowel = (ch) => {
  const c = ch?.charCodeAt(0) ?? 0;
  return c >= 0xac00 && c <= 0xd7a3 ? Math.floor(((c - 0xac00) % 588) / 28) : -1;
};
const J_NIEUN = 4;
const J_RIEUL = 8;
const J_BIEUP = 17;

/** 어절 하나의 끝으로 말 단계 후보를 고른다. 빈 배열은 판정 불가. */
export function readWord(w, question = false) {
  if (!w || FRAGMENTS.has(w)) return [];
  const last = w.at(-1);
  const prev = w.at(-2);
  const prev2 = w.at(-3);
  const ends = (s) => w.endsWith(s);

  if (ends('습니다') || ends('습니까') || ends('십시오') || ends('읍시다')) return ['하십시오체'];
  if ((ends('니다') || ends('니까')) && jong(prev2) === J_BIEUP) return ['하십시오체'];
  if (ends('시다') && jong(prev2) === J_BIEUP) return ['하십시오체'];
  if (last === '요' || last === '죠') return ['해요체'];
  if (ends('구려') || ends('시오') || ends('하오') || ends('이오') || ends('되오')) return ['하오체'];
  if (last === '소' && jong(prev) > 0) return ['하오체'];
  if (ends('게나') || ends('구먼') || ends('는가') || ends('던가') || ends('하세')) return ['하게체'];
  if (last === '네' || (last === '가' && jong(prev) === J_NIEUN) || (question && last === '나')) return ['해체', '하게체'];
  if (last === '게' && jong(prev) === J_RIEUL) return ['해체'];
  if (last === '다' || ['냐', '니', '라', '자'].includes(last) || ends('구나')) return ['해라체'];
  if (['어', '아', '여', '야', '지', '래', '대', '데', '걸', '군', '게', '까', '든'].includes(last)) return ['해체'];
  // 가, 봐, 해, 줘, 돼, 했어 처럼 아/어 계열 모음으로 끝나고 받침이 없으면 해체.
  if (jong(last) === 0 && [0, 1, 4, 6, 9, 10, 14].includes(vowel(last))) return ['해체'];
  return [];
}

/** 대사 한 줄을 문장마다 읽는다. */
export function readLine(line) {
  return sentences(line).map((s) => {
    const trailsOff = /(…|\.\.)$/.test(s);
    const w = lastWord(s);
    // 말줄임표로 흐린 문장은 연결 어미에서 끊긴 경우가 많아 읽지 않는다(시원찮은데…).
    const levels = trailsOff && /(는데|은데|고|며|서|니까|면|지만)$/.test(w) ? [] : readWord(w, s.endsWith('?'));
    return { text: s, levels };
  });
}

/** 한 줄 요약: 읽힌 말 단계들. 섞이면 '해체·해요체'처럼 이어 붙인다. */
export function summarizeLine(line) {
  const seen = [];
  for (const { levels } of readLine(line)) {
    const label = levels.join('/');
    if (label && !seen.includes(label)) seen.push(label);
  }
  return seen.join(' · ');
}

/** 한 줄 안에서 높임과 안높임이 섞였는가(같은 청자에게 한 말이라면 대개 실수). */
export function mixesPoliteness(line) {
  const kinds = new Set();
  for (const { levels } of readLine(line)) {
    if (levels.length === 0) continue;
    const polite = levels.every((l) => POLITE.has(l));
    const plain = levels.every((l) => !POLITE.has(l));
    if (polite) kinds.add('높임');
    if (plain) kinds.add('안높임');
  }
  return kinds.size === 2;
}
