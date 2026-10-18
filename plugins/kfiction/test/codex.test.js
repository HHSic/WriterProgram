import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, existsSync, writeFileSync, mkdirSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { install, ROOT } from '../integrations/codex/install.mjs';
import { handleMessage } from '../integrations/codex/mcp/kfiction-mcp.mjs';

test('install writes the skill with real paths and can be undone', () => {
  const home = mkdtempSync(path.join(os.tmpdir(), 'kfiction-home-'));
  mkdirSync(path.join(home, '.codex'));
  writeFileSync(path.join(home, '.codex', 'AGENTS.md'), '# 내 지침\n');
  install({ home, withHooks: true });
  const skill = readFileSync(path.join(home, '.agents', 'skills', 'kfiction', 'SKILL.md'), 'utf8');
  assert.ok(!skill.includes('${CLAUDE_PLUGIN_ROOT}'));
  assert.ok(skill.includes(ROOT.replace(/\\/g, '/')));
  assert.ok(existsSync(path.join(home, '.agents', 'skills', 'kfiction', 'references', 'speech-levels.md')));
  const agents = readFileSync(path.join(home, '.codex', 'AGENTS.md'), 'utf8');
  assert.match(agents, /# 내 지침[\s\S]*kfiction:begin/);
  assert.match(readFileSync(path.join(home, '.codex', 'config.toml'), 'utf8'), /\[mcp_servers\.kfiction\][\s\S]*apply_patch/);
  const again = install({ home, withHooks: true });
  assert.ok(again.some((l) => l.includes('이미 최신')));
  install({ home, uninstall: true });
  assert.equal(readFileSync(path.join(home, '.codex', 'AGENTS.md'), 'utf8').trim(), '# 내 지침');
  assert.ok(!existsSync(path.join(home, '.agents', 'skills', 'kfiction')));
});

test('mcp server lists and runs the tools', () => {
  assert.equal(handleMessage({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-06-18' } }).result.serverInfo.name, 'kfiction');
  const tools = handleMessage({ jsonrpc: '2.0', id: 2, method: 'tools/list' }).result.tools.map((t) => t.name);
  assert.deepEqual(tools, ['kfiction_check', 'kfiction_levels']);
  const res = handleMessage({ jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'kfiction_check', arguments: { text: '물 자국이 번지는 것이 보여졌다. 비가 왔다.' } } });
  assert.match(res.result.content[0].text, /겹친 피동/);
  assert.equal(handleMessage({ jsonrpc: '2.0', method: 'notifications/initialized' }), null);
});

test('the plugin manifest, hooks and marketplace point at real files', () => {
  const manifest = JSON.parse(readFileSync(path.join(ROOT, '.claude-plugin', 'plugin.json'), 'utf8'));
  assert.equal(manifest.name, 'kfiction');
  const hooks = JSON.parse(readFileSync(path.join(ROOT, 'hooks', 'hooks.json'), 'utf8'));
  assert.match(hooks.hooks.PostToolUse[0].hooks[0].command, /kfiction-hook\.mjs/);
  const market = JSON.parse(readFileSync(path.join(ROOT, '..', '..', '.claude-plugin', 'marketplace.json'), 'utf8'));
  const entry = market.plugins.find((p) => p.name === 'kfiction');
  assert.ok(existsSync(path.join(ROOT, '..', '..', entry.source, '.claude-plugin', 'plugin.json')));
});
