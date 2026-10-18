// 리포트와 원고 판정.

import { koreanRatio, paragraphs, sentences } from './text.js';
import { endingClass } from './lint.js';
import { summarizeLine } from './speech.js';
import { OUT_OF_SCOPE } from './rules.js';

const LABEL = { contract: '계약 위반', warn: '경고', info: '참고' };
const pct = (x) => `${(x * 100).toFixed(0)}%`;

/** 소설 원고로 보이는가. hook이 기술문서나 메모를 건드리지 않게 한다. */
export function looksLikeFiction(text) {
  if (koreanRatio(text) < 0.5) return false;
  const paras = paragraphs(text).filter((p) => !p.excluded);
  if (paras.length < 3) return false;
  const listy = paras.filter((p) => /^([-*+]\s|\d+\.\s|\|)/.test(p.text)).length;
  if (listy / paras.length > 0.2) return false;
  const dialogue = paras.filter((p) => p.dialogues.length > 0).length;
  const narr = paras.flatMap((p) => sentences(p.narration));
  const past = narr.filter((s) => endingClass(s) === '-었다' || endingClass(s) === '-ㄴ다').length;
  return dialogue >= 2 || past >= 5;
}

export function renderReport({ findings, stats }, { file, all = false } = {}) {
  const shown = all ? findings : findings.filter((f) => f.severity !== 'info');
  const count = (sev) => findings.filter((f) => f.severity === sev).length;
  const out = [];
  out.push(`[kfiction] ${file ?? '본문'} · 계약 위반 ${count('contract')} · 경고 ${count('warn')} · 참고 ${count('info')}${all ? '' : ' (참고는 --all)'}`);
  out.push(
    `글자 ${stats.chars} · 지문 문장 ${stats.narrationSentences} · 대사 ${stats.dialogueLines}줄 (분량의 ${pct(stats.dialogueShare)}) · 연결 어미 뒤 쉼표 ${pct(stats.connectiveCommaRate)} · 가장 긴 지문 문장 ${stats.longestSentence}자 · '-었다' 끝맺음 ${pct(stats.pastEndingShare)}`,
  );
  for (const sev of ['contract', 'warn', 'info']) {
    const list = shown.filter((f) => f.severity === sev);
    if (!list.length) continue;
    out.push('', `${LABEL[sev]}`);
    for (const f of list) out.push(`  ${f.line ? 'L' + f.line : '전체'} ${f.rule} ${f.name}: "${f.match}" → ${f.hint}`);
  }
  out.push('', '점검기가 못 보는 것 (사람이나 모델이 확인)');
  for (const x of OUT_OF_SCOPE) out.push(`  · ${x}`);
  out.push('', '이 점검은 막는 기준이 아니다. 사람이 쓴 소설도 1,000자당 두어 곳씩 걸린다. 지적된 곳만 보고 원고 전체를 다시 쓰지 않는다.');
  return out.join('\n');
}

/** 대사마다 말 단계 어림. */
export function renderLevels(text) {
  const out = ['[kfiction] 대사별 말 단계 어림 (문자 규칙, 참고용)'];
  for (const p of paragraphs(text)) {
    if (p.excluded) continue;
    for (const d of p.dialogues) out.push(`  L${p.line} ${summarizeLine(d) || '판정 불가'}: “${d}”`);
  }
  if (out.length === 1) out.push('  대사가 없음');
  return out.join('\n');
}
