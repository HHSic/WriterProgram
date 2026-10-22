import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readLine, summarizeLine, mixesPoliteness } from '../lib/speech.js';

const levels = (line) => readLine(line).map((s) => s.levels);

test('reads each level', () => {
  assert.deepEqual(levels('네, 알겠습니다.'), [['하십시오체']]);
  assert.deepEqual(levels('다들 갑시다.'), [['하십시오체']]);
  assert.deepEqual(levels('내일까지 드릴게요.'), [['해요체']]);
  assert.deepEqual(levels('그만 좀 해. 내가 할게.'), [['해체'], ['해체']]);
  assert.deepEqual(levels('어서 들어가거라.'), [['해라체']]);
  assert.deepEqual(levels('자네도 한잔하게나.'), [['하게체']]);
  assert.deepEqual(levels('어디서 왔소?'), [['하오체']]);
  assert.deepEqual(levels('요즘 시끄럽다네.'), [['해체', '하게체']]);
});

test('fragments and trailing-off sentences stay unread', () => {
  assert.deepEqual(levels('네.'), [[]]);
  assert.deepEqual(levels('무릎이 시원찮은데… 같이 가 볼게요.'), [[], ['해요체']]);
});

test('mixing polite and plain in one line is noticed', () => {
  assert.ok(mixesPoliteness('알았어. 가 볼게요.'));
  assert.ok(!mixesPoliteness('알았어. 갈게.'));
  assert.equal(summarizeLine('네. 바로 가겠습니다.'), '하십시오체');
});

// WriterProgram 의 평가셋 정답 대사로 문자 규칙을 확인한다. 형태소 판정기(crates/revise)보다 거칠다.
test('agrees with the eval set gold lines on polite vs plain', () => {
  const scenes = JSON.parse(readFileSync(new URL('../../../evals/context-sentences/R.json', import.meta.url), 'utf8'));
  const POLITE = new Set(['하십시오체', '해요체', '하오체']);
  let read = 0;
  let agree = 0;
  for (const s of scenes) {
    for (const slot of s.slots) {
      const got = readLine(slot.gold).flatMap((x) => x.levels);
      if (!got.length) continue;
      read++;
      if (got.every((l) => POLITE.has(l) === POLITE.has(slot.level))) agree++;
    }
  }
  assert.ok(read >= 65, `read ${read}`);
  assert.ok(agree / read >= 0.95, `agree ${agree}/${read}`);
});

test('a fragment cut off without an ending is not read as 해체', () => {
  assert.equal(mixesPoliteness('할머니, 그거… 제 거죠.'), false);
  assert.deepEqual(readLine('알았어… 갈게.').map((s) => s.levels), [['해체'], ['해체']]);
});
