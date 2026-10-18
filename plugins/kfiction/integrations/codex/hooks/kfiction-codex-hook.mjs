#!/usr/bin/env node
// Codex PostToolUse hook. apply_patch 가 고친 원고(.md, .txt)에 kfiction 점검을 돌린다.
// 계약 위반만 종료 코드 2 + stderr 로 모델에게 돌려준다. 그 밖은 0.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { handle } from '../../../hooks/kfiction-hook.mjs';

/** apply_patch 본문에서 추가·수정된 파일 경로를 뽑는다. */
export function patchFiles(patch) {
  const files = [];
  for (const m of String(patch ?? '').matchAll(/^\*\*\* (?:Add|Update) File: (.+?)\s*$|^\*\*\* Move to: (.+?)\s*$/gm)) files.push(m[1] ?? m[2]);
  return [...new Set(files)];
}

export function run(input) {
  const errs = [];
  for (const file of patchFiles(input.tool_input?.command).filter((f) => /\.(md|txt)$/i.test(f))) {
    const r = handle({ cwd: input.cwd, tool_name: 'Write', tool_input: { file_path: file } });
    if (r.code === 2 && r.stderr) errs.push(r.stderr);
  }
  return errs.length ? { code: 2, stderr: errs.join('\n') } : { code: 0 };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const r = run(JSON.parse(readFileSync(0, 'utf8') || '{}'));
    if (r.stderr) process.stderr.write(r.stderr + '\n');
    process.exitCode = r.code;
  } catch {
    process.exitCode = 0; // 점검기 고장이 작업을 막지 않게 한다
  }
}
