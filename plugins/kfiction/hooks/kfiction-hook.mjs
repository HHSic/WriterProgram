#!/usr/bin/env node
// kfiction hook + 상태 관리.
//   hook 모드 (stdin 에 PostToolUse JSON):  node kfiction-hook.mjs
//   상태 모드:                              node kfiction-hook.mjs set <on|off|status>
//   원고 폴더:                              node kfiction-hook.mjs set dir [<폴더>|off]
// 소설 원고(.md, .txt)를 쓰거나 고치면 점검한다. 막지 않는다.
//   계약 위반(어시스턴트 말투, 마크다운)만 종료 코드 2로 모델에게 돌려준다. 파일당 2회까지.
//   경고는 stdout JSON systemMessage 로 사용자에게만 한 줄 알린다.
// SessionStart 에서는 원고 폴더가 정해져 있으면 모델에게 알린다.
import { readFileSync, writeFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { lint } from '../lib/lint.js';
import { looksLikeFiction } from '../lib/report.js';
import { manuscriptDir, setManuscriptDir, clearManuscriptDir, configPath } from '../lib/config.js';

export const MAX_FIX = 2;
const STALE_MS = 30 * 60 * 1000;
const MAX_BYTES = 500 * 1024;
const DEFAULT_STATE = { enabled: true, retries: {} };

export function stateDir(cwd) {
  return process.env.KFICTION_STATE_DIR || path.join(process.env.CLAUDE_PROJECT_DIR || cwd || process.cwd(), '.kfiction');
}
const statePath = (cwd) => path.join(stateDir(cwd), 'state.json');

export function loadState(cwd) {
  try {
    const s = JSON.parse(readFileSync(statePath(cwd), 'utf8'));
    return { ...DEFAULT_STATE, ...s, retries: s.retries ?? {} };
  } catch {
    return { ...DEFAULT_STATE, retries: {} };
  }
}
export function saveState(cwd, state) {
  mkdirSync(stateDir(cwd), { recursive: true });
  writeFileSync(statePath(cwd), JSON.stringify(state, null, 2) + '\n');
}

const row = (f) => `L${f.line ?? '-'} ${f.rule} ${f.name}: "${f.match}" -> ${f.hint}`;

/** 판정. {code, stdout?, stderr?} */
export function handle(input, now = Date.now()) {
  const cwd = input.cwd || process.cwd();
  const state = loadState(cwd);
  if (!state.enabled) return { code: 0 };
  if (input.tool_name && !['Write', 'Edit', 'MultiEdit'].includes(input.tool_name)) return { code: 0 };
  const file = input.tool_input?.file_path;
  if (!file || !/\.(md|txt)$/i.test(file)) return { code: 0 };
  const abs = path.resolve(cwd, file);
  if (/[\\/](node_modules|\.git|\.kfiction|\.kste)[\\/]/.test(abs)) return { code: 0 };
  if (!existsSync(abs) || statSync(abs).size > MAX_BYTES) return { code: 0 };
  const text = readFileSync(abs, 'utf8');
  if (!looksLikeFiction(text)) return { code: 0 };

  const { findings } = lint(text);
  const contract = findings.filter((f) => f.severity === 'contract');
  const warns = findings.filter((f) => f.severity === 'warn');
  const name = path.basename(abs);
  const prev = state.retries[abs];
  const fresh = !prev || now - prev.at > STALE_MS;

  if (contract.length === 0 || (!fresh && prev.count >= MAX_FIX)) {
    delete state.retries[abs];
    saveState(cwd, state);
    const parts = [];
    if (contract.length) parts.push(`고치지 못한 계약 위반 ${contract.length}건`);
    if (warns.length) parts.push(`경고 ${warns.length}건 (${[...new Set(warns.map((f) => f.name))].slice(0, 3).join(', ')})`);
    if (!parts.length) return { code: 0 };
    return { code: 0, stdout: JSON.stringify({ systemMessage: `kfiction: ${name} ${parts.join(' · ')}. /kfiction:check 로 봅니다.` }) };
  }

  const count = (fresh ? 0 : prev.count) + 1;
  state.retries[abs] = { count, at: now };
  saveState(cwd, state);
  const lines = [
    `[kfiction] ${name}: 원고에 원고가 아닌 것이 섞였습니다 (${count}/${MAX_FIX}회차).`,
    ...contract.map(row),
    '지적된 줄만 고치세요. 설명·인사·강조 기호를 지우고 원고 문장은 그대로 둡니다.',
  ];
  return { code: 2, stderr: lines.join('\n') };
}

/** SessionStart: 원고 폴더를 모델에게 알린다. 폴더가 없으면(드라이브가 꺼짐 등) 사용자에게도 알린다. */
export function sessionStart() {
  const m = manuscriptDir();
  if (!m) return { code: 0 };
  const context = m.present
    ? [
        `kfiction 원고 폴더: ${m.dir}`,
        '- 작가가 새 원고를 파일로 쓰라고 하면 이 폴더에 만든다. 작가가 다른 경로를 말하면 그 경로를 쓴다.',
        '- "1화.md"처럼 이름만 말한 원고는 지금 폴더에 없으면 이 폴더에서 찾는다.',
        '- 이 폴더는 Google Drive 같은 동기화 폴더일 수 있다. 작가가 요청하지 않은 파일은 만들지 않는다.',
      ]
    : [`kfiction 원고 폴더 ${m.dir} 가 지금 없다(드라이브가 꺼져 있거나 연결이 끊겼을 수 있다). 원고 파일을 쓰기 전에 작가에게 알리고 어디에 쓸지 묻는다.`];
  const out = { hookSpecificOutput: { hookEventName: 'SessionStart', additionalContext: context.join('\n') } };
  if (!m.present) out.systemMessage = `kfiction: 원고 폴더가 없습니다: ${m.dir}`;
  return { code: 0, stdout: JSON.stringify(out) };
}

const dirLine = () => {
  const m = manuscriptDir();
  return m ? `원고 폴더 ${m.dir}${m.present ? '' : ' (지금 없음)'}` : '원고 폴더 없음(claude를 켠 폴더에 씀)';
};

function dirCommand(args, cwd) {
  const p = args.join(' ').trim();
  if (!p) return { ok: true, text: `kfiction: ${dirLine()} · 설정 파일 ${configPath()}` };
  if (p.toLowerCase() === 'off') {
    const old = clearManuscriptDir();
    return { ok: true, text: old ? `kfiction: 원고 폴더 설정을 지움 (${old})` : 'kfiction: 원고 폴더 설정이 없음' };
  }
  const r = setManuscriptDir(p, cwd);
  if (!r.ok) return r;
  return { ok: true, text: `kfiction: 원고 폴더 ${r.dir} · 새 원고는 이 폴더에 씁니다` };
}

export function setCommand(args, cwd = process.cwd()) {
  if (String(args[0] ?? '').toLowerCase() === 'dir') return dirCommand(args.slice(1), cwd);
  const state = loadState(cwd);
  const a = String(args[0] ?? 'status').toLowerCase();
  if (a === 'on' || a === 'off') {
    state.enabled = a === 'on';
    saveState(cwd, state);
  } else if (a !== 'status') {
    return { ok: false, text: `알 수 없는 인자: ${args.join(' ')}\n사용: on | off | status | dir [<폴더>|off]` };
  }
  return { ok: true, text: `kfiction hook: ${state.enabled ? 'on' : 'off'} · 상태 파일 ${statePath(cwd)} · ${dirLine()}` };
}

function main() {
  const argv = process.argv.slice(2);
  if (argv[0] === 'set') {
    const r = setCommand(argv.slice(1));
    (r.ok ? process.stdout : process.stderr).write(r.text + '\n');
    return r.ok ? 0 : 1;
  }
  let input;
  try {
    input = JSON.parse(readFileSync(0, 'utf8') || '{}');
  } catch {
    return 0;
  }
  try {
    const r = input.hook_event_name === 'SessionStart' ? sessionStart() : handle(input);
    if (r.stdout) process.stdout.write(r.stdout + '\n');
    if (r.stderr) process.stderr.write(r.stderr + '\n');
    return r.code;
  } catch (e) {
    if (process.env.KFICTION_DEBUG) process.stderr.write(`kfiction hook 내부 오류: ${e.stack}\n`);
    return 0; // 점검기 고장이 작업을 막지 않게 한다
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main();
}
