// test/fixtures/lint-cases.json 은 scripts/lint-cases.mjs 가 이 점검기로 만든 것이다.
// Rust로 옮긴 점검기(crates/mcp/src/check)도 같은 파일로 시험한다. 여기서 어긋나면
// 규칙을 바꾸고 사례를 다시 만들지 않은 것이므로, 스크립트를 돌리고 Rust 쪽도 맞춘다.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { lint } from '../lib/lint.js';
import { readLine } from '../lib/speech.js';

const fixture = JSON.parse(readFileSync(new URL('./fixtures/lint-cases.json', import.meta.url), 'utf8'));
const AGAIN = '규칙이 바뀌었으면 node plugins/kfiction/scripts/lint-cases.mjs 로 사례를 다시 만들고 crates/mcp/src/check 도 맞춘다';

test('the shared lint cases still match this checker', () => {
  for (const c of fixture.cases) {
    const got = lint(c.text).findings.map(({ rule, severity, line, match }) => ({ rule, severity, line, match }));
    assert.deepEqual(got, c.findings, `${c.name}: ${AGAIN}`);
  }
});

test('the shared speech cases still match this checker', () => {
  for (const s of fixture.speech) assert.deepEqual(readLine(s.line).map((x) => x.levels), s.levels, `${s.line}: ${AGAIN}`);
});
