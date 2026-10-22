// 한국어 소설 원고 점검(T0, 문자 규칙). 막는 점검이 아니라 재는 점검이다.
//   contract: 원고에 어시스턴트 말투나 마크다운이 섞임. hook이 모델에게 돌려준다.
//   warn: 사람 글보다 AI 글에서 잦은 표지. 작가에게만 알린다.
//   info: 참고. 리포트에만 나온다.

import { paragraphs, sentences, lastWord } from './text.js';
import { mixesPoliteness } from './speech.js';
import {
  META_START,
  META_ANY,
  META_WRITE,
  META_OPENER,
  META_TOPIC,
  DOUBLE_PASSIVE,
  UI_NOUNS,
  TRANSLATIONESE,
  CONNECTIVE_COMMA,
  NEGATIVE_CONTRAST,
  CLICHES,
} from './rules.js';

const J_NIEUN = 4;
const J_SSANGSIOS = 20;
const jong = (ch) => {
  const c = ch?.charCodeAt(0) ?? 0;
  return c >= 0xac00 && c <= 0xd7a3 ? (c - 0xac00) % 28 : -1;
};

/** 문장 끝맺음의 갈래: 과거(-었다), 현재(-ㄴ다), 그 밖(끝 두 글자). */
export function endingClass(sentence) {
  const w = lastWord(sentence);
  if (!w) return '';
  if (w.endsWith('다') && jong(w.at(-2)) === J_SSANGSIOS) return '-었다';
  if (w.endsWith('는다') || (w.endsWith('다') && jong(w.at(-2)) === J_NIEUN)) return '-ㄴ다';
  return w.slice(-2);
}

const firstWord = (s) => s.trim().split(/\s+/)[0] ?? '';

/**
 * @returns {{findings: object[], stats: object}}
 * finding: {rule, name, severity, line, match, hint}
 */
export function lint(text) {
  const paras = paragraphs(text);
  const findings = [];
  const add = (rule, name, severity, line, match, hint) => findings.push({ rule, name, severity, line, match, hint });

  // 문장 단위 자료: 지문 문장(줄 번호 포함)과 대사.
  const narr = [];
  const lines = [];
  // 반복을 세는 줄: 대사가 든 문단은 null로 끊는다(대사 한 줄이 끝맺음 반복을 끊는다).
  const flow = [];
  for (const p of paras) {
    if (p.excluded) continue;
    if (p.dialogues.length) flow.push(null);
    for (const s of sentences(p.narration)) {
      narr.push({ line: p.line, text: s });
      flow.push(narr.at(-1));
    }
    for (const d of p.dialogues) lines.push({ line: p.line, text: d });
  }
  const prose = paras.filter((p) => !p.excluded);
  const allText = prose.map((p) => p.text).join('\n');

  // F1 메타 문장
  for (const p of prose) {
    const talk = META_ANY.test(p.narration) || META_WRITE.test(p.narration) || (META_OPENER.test(p.narration.trim()) && META_TOPIC.test(p.narration));
    if (META_START.test(p.text) || talk) add('F1', '어시스턴트 말투', 'contract', p.line, cut(p.text), '원고 밖의 설명이나 인사는 지운다');
  }
  // F2 마크다운
  for (const p of prose) {
    if (/\*\*|__/.test(p.text) || /^[-*+]\s|^\d+\.\s/.test(p.text)) add('F2', '마크다운', 'contract', p.line, cut(p.text), '원고에는 강조 기호와 목록 기호를 쓰지 않는다');
  }
  // F16 영문
  for (const p of prose) {
    const m = p.text.match(/[A-Za-z]{2,}/);
    if (m) add('F16', '영문', 'warn', p.line, m[0], '설정에 없는 영문이면 한국어로');
  }
  // F3 겹친 피동
  for (const p of prose) {
    for (const m of p.text.matchAll(DOUBLE_PASSIVE)) add('F3', '겹친 피동', 'warn', p.line, m[0], "‘보여졌다 → 보였다’처럼 피동을 하나만");
  }
  // F4 '의' 연쇄
  for (const s of narr) {
    const ui = s.text.split(/\s+/).filter((w) => w.length >= 2 && w.endsWith('의') && !UI_NOUNS.has(w) && ![...UI_NOUNS].some((n) => w.endsWith(n)));
    if (ui.length >= 3 || /[가-힣]+의 [가-힣]+의 /.test(s.text)) add('F4', "'의'가 이어짐", 'warn', s.line, ui.join(' '), "‘할머니의 서점 → 할머니 서점’처럼 하나를 뺀다");
  }
  // F5 같은 끝맺음이 여섯 번 넘게, 길이까지 고르게 이어짐. 과거형 서술은 '-었다'가 기본이라
  // 횟수만 세면 사람 글도 다 걸린다(1924년 「운수 좋은 날」에서 확인).
  runs(flow, (s) => (s ? endingClass(s.text) : ''), 6, (run, key) => {
    if (variation(run.map((s) => s.text.length)) < 0.35) add('F5', '같은 끝맺음', 'warn', run[0].line, `${key} ${run.length}번, 길이도 비슷함`, '한 문장은 다른 끝맺음으로, 또는 두 문장을 합친다');
  });
  // F6 같은 말로 시작 세 번
  runs(flow, (s) => (s && firstWord(s.text).length >= 2 ? firstWord(s.text) : ''), 3, (run, key) => add('F6', '같은 말로 시작', 'warn', run[0].line, `${key} ${run.length}번`, '주어를 빼거나 문장을 합친다'));
  // F7 연결 어미 뒤 쉼표
  const commas = narr.filter((s) => CONNECTIVE_COMMA.test(s.text));
  const commaRate = narr.length ? commas.length / narr.length : 0;
  if (commas.length >= 3 && commaRate > 0.15) {
    add('F7', '연결 어미 뒤 쉼표', 'warn', commas[0].line, `지문 문장의 ${(commaRate * 100).toFixed(0)}%`, '‘-고,’ ‘-는데,’ 뒤 쉼표를 뺀다 (사람 글은 4% 안팎)');
  }
  // F8 부정 대조
  const contrasts = [...allText.matchAll(NEGATIVE_CONTRAST)];
  if (contrasts.length >= 2 && contrasts.length / Math.max(allText.length, 1) >= 1 / 1500) {
    add('F8', '부정 대조 반복', 'warn', lineOf(paras, contrasts[0][0]), `${contrasts.length}번`, '‘A가 아니라 B’는 꼭 필요한 한 곳에만');
  }
  // F9 번역투
  for (const p of prose) {
    for (const [expr, hint] of TRANSLATIONESE) if (p.narration.includes(expr)) add('F9', '번역 투', 'info', p.line, expr, hint);
  }
  // F10 상투어 관찰
  for (const p of prose) {
    for (const c of CLICHES) if (p.text.includes(c)) add('F10', '상투어 후보', 'info', p.line, c, '관찰용 목록(검증 전). 이 인물만 할 수 있는 반응인지 본다');
  }
  // F11 줄표
  const dashes = (allText.match(/[가-힣.,!?]\s?[—–―]\s?[가-힣]/g) ?? []).length;
  if (dashes >= 3) add('F11', '줄표', 'info', null, `${dashes}개`, '한국어 소설에서는 드물다. 쉼표나 문장 나누기로');
  // F12, F13 문장 길이
  for (const s of narr) if (s.text.length > 100) add('F12', '긴 문장', 'info', s.line, `${s.text.length}자`, '끊을 곳이 있는지 본다');
  const narrChars = narr.reduce((n, s) => n + s.text.length, 0);
  if (narrChars >= 3000 && !narr.some((s) => s.text.length >= 80)) add('F13', '긴 문장이 없음', 'info', null, '80자 넘는 지문 문장 0개', '동작이 이어지는 긴 문장을 하나쯤 섞는다');
  // F14 대사만 이어짐
  runs(prose, (p) => (p.dialogueOnly ? 'd' : ''), 6, (run) => add('F14', '대사만 이어짐', 'info', run[0].line, `대사 문단 ${run.length}개`, '중간에 행동이나 누가 말했는지를 넣는다'));
  // F15 한 대사 안에서 높임과 안높임이 섞임
  for (const l of lines) if (mixesPoliteness(l.text)) add('F15', '한 대사 안 말 단계 섞임', 'info', l.line, cut(l.text), '청자가 바뀐 게 아니면 말 단계를 맞춘다');

  const dialogueChars = lines.reduce((n, l) => n + l.text.length, 0);
  const pastShare = narr.length ? narr.filter((s) => endingClass(s.text) === '-었다').length / narr.length : 0;
  const stats = {
    chars: allText.length,
    narrationSentences: narr.length,
    dialogueLines: lines.length,
    dialogueShare: allText.length ? dialogueChars / allText.length : 0,
    connectiveCommaRate: commaRate,
    longestSentence: narr.reduce((m, s) => Math.max(m, s.text.length), 0),
    pastEndingShare: pastShare,
  };
  findings.sort((a, b) => (a.line ?? 0) - (b.line ?? 0));
  return { findings, stats };
}

/** 같은 열쇠가 min번 넘게 이어진 구간마다 report(run, key). 빈 열쇠는 끊는다. */
function runs(list, keyOf, min, report) {
  let run = [];
  let key = '';
  const flush = () => {
    if (key && run.length >= min) report(run, key);
  };
  for (const x of list) {
    const k = keyOf(x);
    if (k && k === key) run.push(x);
    else {
      flush();
      run = [x];
      key = k;
    }
  }
  flush();
}

/** 변동계수(표준편차/평균). */
function variation(xs) {
  const mean = xs.reduce((a, b) => a + b, 0) / xs.length;
  const sd = Math.sqrt(xs.reduce((a, x) => a + (x - mean) ** 2, 0) / xs.length);
  return mean ? sd / mean : 0;
}

function lineOf(paras, fragment) {
  return paras.find((p) => p.text.includes(fragment.trim().slice(0, 12)))?.line ?? null;
}

const cut = (s, n = 40) => (s.length > n ? s.slice(0, n) + '…' : s);
