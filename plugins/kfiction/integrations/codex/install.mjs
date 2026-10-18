#!/usr/bin/env node
// kfiction Codex CLI 설치 스크립트. KSTE 의 integrations/codex/install.mjs 와 같은 방식이다.
//   node integrations/codex/install.mjs [--dry-run] [--uninstall] [--with-hooks] [--home <dir>]
// 건드리는 곳 (home 기본값 = 사용자 홈, CODEX_HOME 이 있으면 그 경로가 Codex 디렉터리):
//   <codex>/AGENTS.md         kfiction:begin/end 마커 사이 절만 추가·교체
//   <codex>/config.toml       [mcp_servers.kfiction] (+ --with-hooks 면 PostToolUse hook). 바꾸기 전에 백업
//   <codex>/prompts/          kfiction-candidates.md, kfiction-check.md
//   <home>/.agents/skills/kfiction/   Codex 사용자 skill 위치. 플러그인의 skills/kfiction 을 복사하며
//                                     ${CLAUDE_PLUGIN_ROOT} 를 이 플러그인의 절대 경로로 바꾼다.
import { existsSync, readFileSync, writeFileSync, mkdirSync, copyFileSync, rmSync, readdirSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const ROOT = path.resolve(HERE, '..', '..');
const PROMPTS = ['kfiction-candidates.md', 'kfiction-check.md'];
const MARK = {
  agents: /<!-- kfiction:begin -->[\s\S]*?<!-- kfiction:end -->\n?/,
  mcp: /# kfiction:mcp:begin[\s\S]*?# kfiction:mcp:end\n?/,
  hooks: /# kfiction:hooks:begin[\s\S]*?# kfiction:hooks:end\n?/,
};

export function parseArgs(argv) {
  const o = { dryRun: false, uninstall: false, withHooks: false, home: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dry-run') o.dryRun = true;
    else if (a === '--uninstall') o.uninstall = true;
    else if (a === '--with-hooks') o.withHooks = true;
    else if (a === '--home') o.home = argv[++i];
    else throw new Error(`알 수 없는 옵션: ${a}\n사용법: install.mjs [--dry-run] [--uninstall] [--with-hooks] [--home <dir>]`);
  }
  return o;
}

const toml = (s) => JSON.stringify(s);
const fwd = (p) => p.replace(/\\/g, '/');

export function mcpBlock(serverPath) {
  return `# kfiction:mcp:begin\n[mcp_servers.kfiction]\ncommand = "node"\nargs = [${toml(fwd(serverPath))}]\nstartup_timeout_sec = 20\ntool_timeout_sec = 60\n# kfiction:mcp:end\n`;
}
export function hooksBlock(hookPath) {
  return `# kfiction:hooks:begin\n[[hooks.PostToolUse]]\nmatcher = "apply_patch"\n\n[[hooks.PostToolUse.hooks]]\ntype = "command"\ncommand = ${toml(`node ${fwd(hookPath)}`)}\ntimeout = 20\n# kfiction:hooks:end\n`;
}

/** Claude Code 용 파일의 ${CLAUDE_PLUGIN_ROOT} 를 실제 경로로 바꾼다. */
export function localize(text) {
  return text.replaceAll('${CLAUDE_PLUGIN_ROOT}', fwd(ROOT));
}

function filesUnder(dir) {
  return readdirSync(dir).flatMap((name) => {
    const p = path.join(dir, name);
    return statSync(p).isDirectory() ? filesUnder(p) : [p];
  });
}

export function install(opts = {}) {
  const home = opts.home ? path.resolve(opts.home) : os.homedir();
  const codex = opts.home ? path.join(home, '.codex') : process.env.CODEX_HOME || path.join(home, '.codex');
  const dry = !!opts.dryRun;
  const log = [];
  const say = (m) => log.push(dry ? `[dry-run] ${m}` : m);
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const backedUp = new Set();
  const read = (f) => (existsSync(f) ? readFileSync(f, 'utf8') : '');
  const write = (f, text) => {
    if (dry) return;
    mkdirSync(path.dirname(f), { recursive: true });
    writeFileSync(f, text);
  };
  const backup = (f) => {
    if (!existsSync(f) || backedUp.has(f)) return;
    backedUp.add(f);
    say(`백업: ${f} -> ${f}.kfiction-bak-${stamp}`);
    if (!dry) copyFileSync(f, `${f}.kfiction-bak-${stamp}`);
  };
  const editBlock = (file, re, block, label) => {
    const cur = read(file);
    let next;
    if (opts.uninstall) {
      if (!re.test(cur)) return say(`건너뜀: ${label}에 kfiction 항목 없음 (${file})`);
      next = cur.replace(re, '').replace(/\n{3,}/g, '\n\n');
    } else if (re.test(cur)) {
      next = cur.replace(re, () => block);
      if (next === cur) return say(`건너뜀: ${label} 이미 최신 (${file})`);
    } else {
      next = cur + (cur && !cur.endsWith('\n\n') ? (cur.endsWith('\n') ? '\n' : '\n\n') : '') + block;
    }
    backup(file);
    say(`${opts.uninstall ? '제거' : '기록'}: ${label} (${file})`);
    write(file, next);
  };

  const configFile = path.join(codex, 'config.toml');
  const skillSrc = path.join(ROOT, 'skills', 'kfiction');
  const skillDst = path.join(home, '.agents', 'skills', 'kfiction');
  editBlock(path.join(codex, 'AGENTS.md'), MARK.agents, readFileSync(path.join(HERE, 'AGENTS.md'), 'utf8').trimEnd() + '\n', 'AGENTS.md kfiction 절');
  editBlock(configFile, MARK.mcp, mcpBlock(path.join(HERE, 'mcp', 'kfiction-mcp.mjs')), 'config.toml MCP 서버');
  if (opts.withHooks || opts.uninstall) editBlock(configFile, MARK.hooks, hooksBlock(path.join(HERE, 'hooks', 'kfiction-codex-hook.mjs')), 'config.toml hook');

  const targets = [...PROMPTS.map((p) => path.join(codex, 'prompts', p)), skillDst];
  if (opts.uninstall) {
    for (const t of targets) {
      if (!existsSync(t)) {
        say(`건너뜀: 없음 (${t})`);
        continue;
      }
      say(`삭제: ${t}`);
      if (!dry) rmSync(t, { recursive: true, force: true });
    }
    return log;
  }
  for (const p of PROMPTS) {
    say(`복사: prompts/${p} -> ${path.join(codex, 'prompts', p)}`);
    write(path.join(codex, 'prompts', p), localize(readFileSync(path.join(HERE, 'prompts', p), 'utf8')));
  }
  say(`복사: skills/kfiction -> ${skillDst}`);
  for (const f of filesUnder(skillSrc)) write(path.join(skillDst, path.relative(skillSrc, f)), localize(readFileSync(f, 'utf8')));
  return log;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const opts = parseArgs(process.argv.slice(2));
    process.stdout.write(install(opts).join('\n') + '\n');
    if (!opts.dryRun && !opts.uninstall) process.stdout.write('완료. Codex 를 다시 시작하세요. hook 은 첫 실행 때 trust 확인을 받습니다.\n');
  } catch (e) {
    process.stderr.write(e.message + '\n');
    process.exitCode = 2;
  }
}
