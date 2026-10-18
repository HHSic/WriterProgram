import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, existsSync, writeFileSync, mkdirSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { build, instructions, INSTRUCTIONS_LIMIT } from '../integrations/chatgpt/build.mjs';
import { installDesktop } from '../integrations/chatgpt/install-desktop.mjs';
import { crc32 } from '../integrations/chatgpt/zip.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const DIST = path.join(ROOT, 'integrations', 'chatgpt', 'dist');

test('instructions fit a GPT and drop the command-line checker', () => {
  const text = instructions();
  assert.ok([...text].length <= INSTRUCTIONS_LIMIT);
  assert.ok(!text.includes('CLAUDE_PLUGIN_ROOT'));
  assert.ok(!text.includes('## 원고 점검'));
  for (const h of ['## 확인 목록', '## 문체 카드', '## 관계 표', '## 말 단계 어미 표']) assert.ok(text.includes(h), h);
});

test('the committed dist matches the skill source', () => {
  const out = mkdtempSync(path.join(os.tmpdir(), 'kfiction-dist-'));
  build(out);
  for (const f of ['instructions.md', 'kfiction-skill.zip', 'knowledge/speech-levels.md']) {
    assert.ok(readFileSync(path.join(out, f)).equals(readFileSync(path.join(DIST, f))), `${f}: node integrations/chatgpt/build.mjs 로 다시 만드세요`);
  }
});

test('the skill zip has one top folder and readable entries', () => {
  const zip = readFileSync(path.join(DIST, 'kfiction-skill.zip'));
  const names = [];
  for (let i = 0; i + 30 < zip.length && zip.readUInt32LE(i) === 0x04034b50; ) {
    const size = zip.readUInt32LE(i + 18);
    const nameLen = zip.readUInt16LE(i + 26);
    const name = zip.subarray(i + 30, i + 30 + nameLen).toString('utf8');
    const data = zip.subarray(i + 30 + nameLen, i + 30 + nameLen + size);
    assert.equal(crc32(data), zip.readUInt32LE(i + 14), name);
    names.push(name);
    i += 30 + nameLen + size;
  }
  assert.ok(names.includes('kfiction/SKILL.md'));
  assert.ok(names.every((n) => n.startsWith('kfiction/')));
});

test('desktop install adds a personal marketplace entry and copies the plugin', () => {
  const home = mkdtempSync(path.join(os.tmpdir(), 'kfiction-desk-'));
  mkdirSync(path.join(home, '.agents', 'plugins'), { recursive: true });
  const market = path.join(home, '.agents', 'plugins', 'marketplace.json');
  writeFileSync(market, JSON.stringify({ name: 'personal', plugins: [{ name: 'other', source: { source: 'local', path: './.codex/plugins/other' } }] }));
  installDesktop({ home });
  const data = JSON.parse(readFileSync(market, 'utf8'));
  assert.deepEqual(data.plugins.map((p) => p.name), ['other', 'kfiction']);
  assert.equal(data.plugins[1].source.path, './.codex/plugins/kfiction');
  assert.ok(existsSync(path.join(home, '.codex', 'plugins', 'kfiction', 'plugin.json')));
  assert.ok(existsSync(path.join(home, '.codex', 'plugins', 'kfiction', 'skills', 'kfiction', 'SKILL.md')));
  assert.ok(installDesktop({ home }).some((l) => l.includes('이미 최신')));
  installDesktop({ home, uninstall: true });
  assert.deepEqual(JSON.parse(readFileSync(market, 'utf8')).plugins.map((p) => p.name), ['other']);
  assert.ok(!existsSync(path.join(home, '.codex', 'plugins', 'kfiction')));
});

test('portable manifest and repo marketplace point at the plugin', () => {
  const manifest = JSON.parse(readFileSync(path.join(ROOT, 'plugin.json'), 'utf8'));
  assert.equal(manifest.name, 'kfiction');
  assert.match(manifest.$schema, /agent-plugins\.org/);
  const market = JSON.parse(readFileSync(path.join(ROOT, '..', '..', '.agents', 'plugins', 'marketplace.json'), 'utf8'));
  const entry = market.plugins.find((p) => p.name === 'kfiction');
  assert.ok(existsSync(path.join(ROOT, '..', '..', entry.source.path, 'skills', 'kfiction', 'SKILL.md')));
});
