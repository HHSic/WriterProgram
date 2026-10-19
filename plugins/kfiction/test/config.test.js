import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const tmp = (name) => mkdtempSync(path.join(os.tmpdir(), name));
process.env.KFICTION_CONFIG_DIR = tmp('kfiction-config-');

const { setCommand, sessionStart } = await import('../hooks/kfiction-hook.mjs');
const { resolveManuscript, manuscriptDir } = await import('../lib/config.js');
const { main } = await import('../bin/kfiction.js');

const STORY = '서하는 셔터를 내렸다. 비가 왔다.\n\n“영업 끝났나요?”\n\n“들어오세요.”\n\n남자가 봉투를 내려놓았다.\n';

test('no folder set: session start says nothing', () => {
  setCommand(['dir', 'off']);
  assert.deepEqual(sessionStart(), { code: 0 });
  assert.match(setCommand(['status'], tmp('kfiction-')).text, /원고 폴더 없음/);
});

test('dir sets a folder, creating it under an existing parent', () => {
  const drive = tmp('kfiction-drive-');
  const r = setCommand(['dir', path.join(drive, '내 소설')]);
  assert.equal(r.ok, true);
  assert.ok(existsSync(path.join(drive, '내 소설')));
  assert.equal(manuscriptDir().dir, path.join(drive, '내 소설'));
  const ctx = JSON.parse(sessionStart().stdout).hookSpecificOutput;
  assert.equal(ctx.hookEventName, 'SessionStart');
  assert.match(ctx.additionalContext, /내 소설/);
});

test('unquoted paths with spaces are joined back', () => {
  const drive = tmp('kfiction-drive-');
  mkdirSync(path.join(drive, '내 드라이브'));
  assert.equal(setCommand(['dir', `${drive}/내`, '드라이브']).ok, true);
  assert.equal(manuscriptDir().dir, path.join(drive, '내 드라이브'));
});

test('a missing parent is refused instead of made locally', () => {
  const before = manuscriptDir();
  const r = setCommand(['dir', path.join(tmp('kfiction-'), '없는 드라이브', '소설')]);
  assert.equal(r.ok, false);
  assert.match(r.text, /위 폴더가 없음/);
  assert.deepEqual(manuscriptDir(), before);
});

test('a folder that went away is reported at session start', () => {
  const drive = tmp('kfiction-drive-');
  setCommand(['dir', drive]);
  rmSync(drive, { recursive: true });
  const out = JSON.parse(sessionStart().stdout);
  assert.match(out.systemMessage, /원고 폴더가 없습니다/);
  assert.match(out.hookSpecificOutput.additionalContext, /작가에게 알리고/);
});

test('bare file names are found in the manuscript folder', () => {
  const drive = tmp('kfiction-drive-');
  writeFileSync(path.join(drive, '1화.md'), STORY);
  setCommand(['dir', drive]);
  const elsewhere = tmp('kfiction-');
  assert.equal(resolveManuscript('1화.md', elsewhere), path.join(drive, '1화.md'));
  assert.equal(resolveManuscript('2화.md', elsewhere), path.join(elsewhere, '2화.md'));
  writeFileSync(path.join(elsewhere, '1화.md'), STORY);
  assert.equal(resolveManuscript('1화.md', elsewhere), path.join(elsewhere, '1화.md'));
  const cwd = process.cwd();
  try {
    process.chdir(tmp('kfiction-'));
    assert.match(main(['levels', '1화.md']).out, /영업 끝났나요/);
  } finally {
    process.chdir(cwd);
  }
});

test('dir off clears the setting', () => {
  setCommand(['dir', tmp('kfiction-drive-')]);
  assert.match(setCommand(['dir', 'off']).text, /지움/);
  assert.equal(manuscriptDir(), null);
});
