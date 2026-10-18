import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { handle, setCommand } from '../hooks/kfiction-hook.mjs';
import { patchFiles, run } from '../integrations/codex/hooks/kfiction-codex-hook.mjs';

const STORY = '서하는 셔터를 내렸다. 비가 왔다.\n\n“영업 끝났나요?”\n\n“들어오세요.”\n\n남자가 봉투를 내려놓았다. 물 자국이 번지는 것이 보여졌다.\n';

function project(files) {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'kfiction-'));
  for (const [name, text] of Object.entries(files)) writeFileSync(path.join(dir, name), text);
  return dir;
}

test('style warnings go to the user only', () => {
  const cwd = project({ '1화.md': STORY });
  const r = handle({ cwd, tool_name: 'Write', tool_input: { file_path: '1화.md' } });
  assert.equal(r.code, 0);
  assert.match(JSON.parse(r.stdout).systemMessage, /겹친 피동/);
});

test('assistant talk goes back to the model, twice at most', () => {
  const cwd = project({ '2화.txt': '다음은 이어서 쓴 장면입니다.\n\n' + STORY });
  const input = { cwd, tool_name: 'Edit', tool_input: { file_path: '2화.txt' } };
  assert.equal(handle(input, 1000).code, 2);
  assert.equal(handle(input, 2000).code, 2);
  const third = handle(input, 3000);
  assert.equal(third.code, 0);
  assert.match(JSON.parse(third.stdout).systemMessage, /고치지 못한 계약 위반/);
});

test('technical docs and other tools are left alone', () => {
  const cwd = project({ 'README.md': '# 설치\n\n- 받기\n- 설치\n- 실행\n- 끝\n', '1화.md': STORY });
  assert.deepEqual(handle({ cwd, tool_name: 'Write', tool_input: { file_path: 'README.md' } }), { code: 0 });
  assert.deepEqual(handle({ cwd, tool_name: 'Read', tool_input: { file_path: '1화.md' } }), { code: 0 });
});

test('the hook can be turned off', () => {
  const cwd = project({ '1화.md': STORY });
  assert.match(setCommand(['off'], cwd).text, /off/);
  assert.deepEqual(handle({ cwd, tool_name: 'Write', tool_input: { file_path: '1화.md' } }), { code: 0 });
  assert.equal(setCommand(['bogus'], cwd).ok, false);
});

test('codex patches are read for changed manuscripts', () => {
  assert.deepEqual(patchFiles('*** Begin Patch\n*** Update File: 3화.md\n@@\n*** Add File: notes.txt\n*** End Patch'), ['3화.md', 'notes.txt']);
  const cwd = project({ '3화.md': '원하시면 더 다듬어 드릴게요.\n\n' + STORY });
  assert.equal(run({ cwd, tool_input: { command: '*** Update File: 3화.md\n' } }).code, 2);
});
