import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { lint, endingClass } from '../lib/lint.js';
import { looksLikeFiction } from '../lib/report.js';

const rules = (text) => lint(text).findings.map((f) => f.rule);
const SAMPLE = readFileSync(new URL('../../../crates/revise/testdata/sample-12.txt', import.meta.url), 'utf8');

test('the planted problems in the revise sample are found', () => {
  const found = rules(SAMPLE);
  for (const r of ['F3', 'F4', 'F5', 'F6', 'F9', 'F14']) assert.ok(found.includes(r), r);
});

test('assistant talk and markdown are contract breaks', () => {
  const { findings } = lint('다음은 이어서 쓴 장면입니다.\n\n그는 문을 열었다. **비**가 왔다.\n\n원하시면 더 다듬어 드릴게요.');
  const contract = findings.filter((f) => f.severity === 'contract').map((f) => f.rule);
  assert.deepEqual(contract, ['F1', 'F2', 'F1']);
});

test('dialogue breaks a run of endings, and uneven lengths are not flagged', () => {
  const even = '문을 열었다. 비가 왔다. 길이 젖었다. 등이 켜졌다. 차가 섰다. 개가 짖었다.';
  assert.ok(rules(even).includes('F5'));
  const broken = '문을 열었다. 비가 왔다. 길이 젖었다.\n“누구야?”\n등이 켜졌다. 차가 섰다. 개가 짖었다.';
  assert.ok(!rules(broken).includes('F5'));
  const uneven = '문을 열었다. 밤새 내리던 비가 그칠 기미도 없이 지붕을 두드리고 있었다. 길이 젖었다. 골목 끝 가로등이 깜빡이다가 겨우 켜졌다. 차가 섰다. 개가 짖었다.';
  assert.ok(!rules(uneven).includes('F5'));
});

test('words ending in 의 that are not particles are skipped', () => {
  assert.ok(!rules('회의의 정의와 동의의 의의를 따졌다.').includes('F4'));
  assert.ok(rules('카드의 뒷면에는 할머니의 서점의 옛 주소가 있었다.').includes('F4'));
});

test('ending classes', () => {
  assert.equal(endingClass('그녀는 손을 멈췄다.'), '-었다');
  assert.equal(endingClass('비가 온다.'), '-ㄴ다');
  assert.equal(endingClass('그는 떠났을까?'), '을까');
});

test('fiction is told apart from notes and technical docs', () => {
  assert.ok(looksLikeFiction(SAMPLE));
  assert.ok(!looksLikeFiction('# 설치\n\n- 노드를 설치합니다.\n- 저장소를 받습니다.\n- npm test 를 실행합니다.\n- 끝입니다.'));
  assert.ok(!looksLikeFiction('This is an English story. "Hello," she said. "Hi," he said.'));
});
