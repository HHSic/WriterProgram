#!/usr/bin/env node
// ChatGPT 데스크톱 앱(과 Codex)의 개인 플러그인으로 kfiction을 설치한다.
//   node integrations/chatgpt/install-desktop.mjs [--dry-run] [--uninstall] [--home <dir>]
// OpenAI 플러그인 문서(developers.openai.com/plugins/build/plugins)의 개인 설치 방식:
//   ~/.codex/plugins/kfiction/             플러그인 폴더 복사 (plugin.json, skills/)
//   ~/.agents/plugins/marketplace.json     개인 마켓플레이스에 kfiction 항목 추가. 바꾸기 전에 백업
// 설치 뒤 앱을 다시 시작하고 플러그인 목록에서 kfiction을 설치한다.
import { existsSync, readFileSync, writeFileSync, mkdirSync, copyFileSync, rmSync, cpSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const NAME = 'kfiction';
/** 앱이 쓰는 것만 복사한다. 점검기는 명령을 실행하는 Codex에서 쓸 수 있게 함께 둔다. */
const PARTS = ['plugin.json', 'skills', 'bin', 'lib', 'README.md'];

export function entry() {
  return {
    name: NAME,
    source: { source: 'local', path: `./.codex/plugins/${NAME}` },
    policy: { installation: 'AVAILABLE', authentication: 'ON_INSTALL' },
    category: 'Productivity',
  };
}

export function installDesktop(opts = {}) {
  const home = opts.home ? path.resolve(opts.home) : os.homedir();
  const dry = !!opts.dryRun;
  const log = [];
  const say = (m) => log.push(dry ? `[dry-run] ${m}` : m);
  const dest = path.join(home, '.codex', 'plugins', NAME);
  const market = path.join(home, '.agents', 'plugins', 'marketplace.json');

  let data = { name: 'personal', interface: { displayName: 'Personal' }, plugins: [] };
  if (existsSync(market)) {
    try {
      data = JSON.parse(readFileSync(market, 'utf8'));
    } catch {
      throw new Error(`마켓플레이스 파일을 읽지 못함: ${market}. 직접 고친 뒤 다시 실행하세요.`);
    }
    data.plugins ??= [];
  }
  const others = data.plugins.filter((p) => p.name !== NAME);
  const next = { ...data, plugins: opts.uninstall ? others : [...others, entry()] };
  const changed = JSON.stringify(next) !== JSON.stringify(data);

  if (changed) {
    if (existsSync(market)) {
      const bak = `${market}.kfiction-bak-${new Date().toISOString().replace(/[:.]/g, '-')}`;
      say(`백업: ${market} -> ${bak}`);
      if (!dry) copyFileSync(market, bak);
    }
    say(`${opts.uninstall ? '제거' : '기록'}: 마켓플레이스 항목 (${market})`);
    if (!dry) {
      mkdirSync(path.dirname(market), { recursive: true });
      writeFileSync(market, JSON.stringify(next, null, 2) + '\n');
    }
  } else say(`건너뜀: 마켓플레이스 ${opts.uninstall ? '항목 없음' : '이미 최신'} (${market})`);

  if (existsSync(dest)) {
    say(`삭제: 이전 플러그인 폴더 ${dest}`);
    if (!dry) rmSync(dest, { recursive: true, force: true });
  }
  if (!opts.uninstall) {
    say(`복사: ${PARTS.join(', ')} -> ${dest}`);
    if (!dry) for (const part of PARTS) cpSync(path.join(ROOT, part), path.join(dest, part), { recursive: true });
  }
  return log;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const argv = process.argv.slice(2);
  const opts = { dryRun: argv.includes('--dry-run'), uninstall: argv.includes('--uninstall'), home: argv.includes('--home') ? argv[argv.indexOf('--home') + 1] : null };
  try {
    process.stdout.write(installDesktop(opts).join('\n') + '\n');
    if (!opts.dryRun && !opts.uninstall) process.stdout.write('완료. ChatGPT 데스크톱 앱을 다시 시작하고 플러그인 목록에서 kfiction을 설치하세요.\n');
  } catch (e) {
    process.stderr.write(e.message + '\n');
    process.exitCode = 2;
  }
}
