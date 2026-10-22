#!/usr/bin/env node
// 점검기 시험 사례를 만든다. 결과는 이 JS 점검기가 낸 그대로다.
//   node plugins/kfiction/scripts/lint-cases.mjs
// test/fixtures/lint-cases.json 을 다시 쓴다. Rust로 옮긴 점검기(crates/mcp/src/check)와
// JS 점검기가 같은 판정을 내는지 두 쪽 시험이 이 파일로 확인한다.
// 규칙을 바꾸면 이 스크립트를 다시 돌리고, Rust 쪽도 같이 고친다.
import { readFileSync, writeFileSync } from 'node:fs';
import { lint } from '../lib/lint.js';
import { readLine } from '../lib/speech.js';

const here = (p) => new URL(p, import.meta.url);
const SAMPLE = readFileSync(here('../../../crates/revise/testdata/sample-12.txt'), 'utf8');
const gold = JSON.parse(readFileSync(here('../../../evals/context-sentences/R.json'), 'utf8')).flatMap((s) =>
  s.slots.map((slot) => slot.gold),
);

const narration = SAMPLE.split('\n').filter((l) => l.trim() && !/[“"]/.test(l));

const texts = {
  'revise sample': SAMPLE,
  'contract breaks': '다음은 이어서 쓴 장면입니다.\n\n그는 문을 열었다. **비**가 왔다.\n\n원하시면 더 다듬어 드릴게요.\n\n- 목록 하나\n\n1. 번호 목록',
  'assistant talk': [
    '물론이죠. 다음과 같이 이어 썼습니다.',
    '네, 요청하신 대로 장면을 이어 썼습니다.',
    '좋습니다. 하진의 시점으로 다음 장면을 써 봤어요.',
    '알겠습니다! 초안을 다듬어 드렸습니다.',
    '아래와 같이 대사를 고쳤습니다.',
    '이어서 써 봤습니다.',
    '문장을 조금 다듬었습니다.',
    '마음에 드시면 다음 회차도 이어 갈게요.',
    '말씀하신 대로 시점을 하진으로 바꿨습니다.',
  ].join('\n\n'),
  'fiction that sounds like talk': [
    '편지에는 다음과 같이 적혀 있었습니다.',
    '나는 그날 밤 편지를 다시 썼습니다.',
    '“물론이죠. 다음과 같이 이어 썼습니다.” 그녀가 웃으며 말했다.',
    '네 시가 넘어서야 비가 그쳤다.',
    '알겠습니다. 저는 그렇게 대답하고 방을 나왔습니다.',
    '원고는 책상 위에 그대로 있었다.',
    '물론이죠, 하고 그는 웃었다.',
    '그는 시계를 고쳐 놓았습니다.',
  ].join('\n\n'),
  'even endings': '문을 열었다. 비가 왔다. 길이 젖었다. 등이 켜졌다. 차가 섰다. 개가 짖었다.',
  'dialogue breaks a run': '문을 열었다. 비가 왔다. 길이 젖었다.\n“누구야?”\n등이 켜졌다. 차가 섰다. 개가 짖었다.',
  'uneven lengths': '문을 열었다. 밤새 내리던 비가 그칠 기미도 없이 지붕을 두드리고 있었다. 길이 젖었다. 골목 끝 가로등이 깜빡이다가 겨우 켜졌다. 차가 섰다. 개가 짖었다.',
  'ui nouns': '회의의 정의와 동의의 의의를 따졌다.\n\n카드의 뒷면에는 할머니의 서점의 옛 주소가 있었다.',
  'front matter and blocks': '---\ntitle: "1화"\nstatus: draft\n---\n\n# 1화\n\n[상태창: 레벨 3]\n\n【 경고 】\n\n```\n**코드**\n```\n\n***\n\n그는 Hunter 라이선스를 꺼냈다. 문이 열려졌다. 이름이 불려졌다.',
  'quotes and levels': '「어서 오세요」 하고 그가 말했다.\n\n『책 이름』을 펼쳤다.\n\n"알았어. 가 볼게요."\n\n“네. 바로 가겠습니다.”\n\n“짝이 없는 따옴표로 끝나는 대사',
  'commas, contrast, dashes': [
    '그는 문을 열고, 밖을 보았다. 비가 왔는데, 우산이 없었다. 길은 젖었지만, 걸었다. 바람이 불면서, 등이 켜졌다.',
    '그것은 두려움이 아니라 설렘이었다. 슬픔이 아니라 안도였다. 그건 끝이 아니라 시작이었다.',
    '그는 웃었다—그리고 울었다. 문이—닫혔다. 비가 — 왔다. 사람이 — 떠났다.',
  ].join('\n\n'),
  'translationese and cliches': '그 사람으로부터 연락을 받았다. 회의에 있어서 그는 말이 없었다.\n\n그녀는 숨을 깊이 들이쉬고 입술을 깨물었다. 심장이 쿵 내려앉았다.\n\n문제에 대한 답을 통해 알게 된 것은 그가 침묵하지 않을 수 없었다는 사실이었다.',
  'dialogue only': ['“가자.”', '“어디로?”', '“집.”', '“지금?”', '“응.”', '“알았어.”', '“빨리.”'].join('\n\n'),
  'long sentence': '그는 시장 끝 골목에서 오래된 열쇠 가게의 셔터를 반쯤 올리고 허리를 숙여 안으로 들어간 뒤 어둠 속에서 손을 더듬어 전등 줄을 찾아 당기고 나서야 카운터 위에 놓인 낯선 열쇠 꾸러미를 보았는데 그 꾸러미는 어제 문을 닫을 때까지만 해도 분명히 거기에 없었다.',
  'long narration': Array.from({ length: 6 }, () => narration.join('\n\n')).join('\n\n'),
  'no long sentence': Array.from({ length: 60 }, (_, i) => `${i + 1}번째 골목에서 윤재는 걸음을 멈추었다. 셔터 내린 가게 앞으로 바람이 지나갔다. 서아는 대답 대신 손전등을 켰다.`).join('\n\n'),
  'empty and odd': '\n\n   \n\n…\n\n?!\n\n“”',
};

const speechLines = [
  '네, 알겠습니다.',
  '다들 갑시다.',
  '내일까지 드릴게요.',
  '그만 좀 해. 내가 할게.',
  '어서 들어가거라.',
  '자네도 한잔하게나.',
  '어디서 왔소?',
  '요즘 시끄럽다네.',
  '네.',
  '무릎이 시원찮은데… 같이 가 볼게요.',
  '알았어. 가 볼게요.',
  '그 사람 누군가?',
  '내가 할게. 너는 가.',
  '바다.',
  ...gold,
];

const fixture = {
  note: 'plugins/kfiction/scripts/lint-cases.mjs 가 JS 점검기로 만든 것. 손으로 고치지 않는다.',
  cases: Object.entries(texts).map(([name, text]) => ({
    name,
    text,
    findings: lint(text).findings.map(({ rule, severity, line, match }) => ({ rule, severity, line, match })),
  })),
  speech: speechLines.map((line) => ({ line, levels: readLine(line).map((s) => s.levels) })),
};

writeFileSync(here('../test/fixtures/lint-cases.json'), JSON.stringify(fixture, null, 1) + '\n');
const total = fixture.cases.reduce((n, c) => n + c.findings.length, 0);
process.stdout.write(`만듦: test/fixtures/lint-cases.json · 사례 ${fixture.cases.length}개, 찾은 것 ${total}개, 대사 ${fixture.speech.length}줄\n`);
