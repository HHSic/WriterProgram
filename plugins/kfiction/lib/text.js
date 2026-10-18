// 원고를 문단과 문장으로 나눈다. 대사(따옴표 안)와 지문을 가른다.
// 형태소 분석 없이 문자만 본다. 줄 번호는 1부터.

const OPEN = { '“': '”', '「': '」', '『': '』', '"': '"' };
const BREAK = /^[\s◆◇*\-―─•·~=]+$/;

/** 머리말(front matter)과 코드 블록을 지운다. 줄 수는 그대로 둔다. */
export function stripNonProse(text) {
  const blank = (m) => m.replace(/[^\n]/g, '');
  return text
    .replace(/^﻿/, '')
    .replace(/\r\n/g, '\n')
    .replace(/^---\n[\s\S]*?\n---(?=\n)/, blank)
    .replace(/```[\s\S]*?```|~~~[\s\S]*?~~~/g, blank);
}

/** 한글 비율: 한글 / (한글 + 라틴 글자). */
export function koreanRatio(text) {
  const ko = (text.match(/[가-힣]/g) ?? []).length;
  const en = (text.match(/[A-Za-z]/g) ?? []).length;
  return ko < 3 ? 0 : ko / (ko + en);
}

/**
 * 문단 목록. 장면 구분자, 상태창([ ], 【 】), 제목(#)은 excluded.
 * @returns {{line:number, text:string, excluded:boolean, dialogues:string[], narration:string, dialogueOnly:boolean}[]}
 */
export function paragraphs(text) {
  const out = [];
  stripNonProse(text)
    .split('\n')
    .forEach((raw, i) => {
      const t = raw.trim();
      if (!t) return;
      const excluded = BREAK.test(t) || /^#{1,6}\s/.test(t) || /^\[.*\]$/.test(t) || /^【.*】$/.test(t);
      const { dialogues, narration } = splitQuotes(t);
      out.push({ line: i + 1, text: t, excluded, dialogues, narration, dialogueOnly: dialogues.length > 0 && !/[가-힣]{2}/.test(narration) });
    });
  return out;
}

/** 따옴표 안(대사)과 밖(지문)을 나눈다. 짝이 없는 따옴표는 문단 끝까지 대사로 본다. */
export function splitQuotes(t) {
  const dialogues = [];
  let narration = '';
  let i = 0;
  while (i < t.length) {
    const close = OPEN[t[i]];
    if (close) {
      const end = t.indexOf(close, i + 1);
      const stop = end < 0 ? t.length : end;
      dialogues.push(t.slice(i + 1, stop).trim());
      narration += ' ';
      i = stop + 1;
    } else {
      narration += t[i];
      i++;
    }
  }
  return { dialogues, narration: narration.replace(/\s+/g, ' ').trim() };
}

/** 지문을 문장으로 나눈다. 마침표·물음표·느낌표·말줄임표가 이어지면 하나로 본다. */
export function sentences(s) {
  return (s.match(/[^.?!…]+[.?!…]*/g) ?? []).map((x) => x.trim()).filter((x) => /[가-힣]/.test(x));
}

/** 마지막 어절(문장 부호를 뗀 것). */
export function lastWord(sentence) {
  const words = sentence.replace(/[^가-힣A-Za-z0-9\s]+$/g, '').trim().split(/\s+/);
  return (words[words.length - 1] ?? '').replace(/[^가-힣]+$/g, '');
}
