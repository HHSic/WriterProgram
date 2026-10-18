#!/usr/bin/env node
// kfiction: 한국어 소설 원고 점검.
//   kfiction check <파일> [--all] [--json]   원고 점검 리포트
//   kfiction levels <파일>                    대사별 말 단계 어림
// 파일 대신 '-'를 주면 표준 입력을 읽는다.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { lint } from '../lib/lint.js';
import { renderReport, renderLevels } from '../lib/report.js';

const USAGE = '사용법: kfiction check <파일|-> [--all] [--json] | kfiction levels <파일|->';

export function main(argv, read = (f) => readFileSync(f === '-' ? 0 : f, 'utf8')) {
  const [cmd, file, ...rest] = argv;
  if (!cmd || !file) return { code: 2, err: USAGE };
  let text;
  try {
    text = read(file);
  } catch (e) {
    return { code: 2, err: `파일을 읽지 못함: ${file} (${e.message})` };
  }
  if (cmd === 'levels') return { code: 0, out: renderLevels(text) };
  if (cmd !== 'check') return { code: 2, err: USAGE };
  const result = lint(text);
  if (rest.includes('--json')) return { code: 0, out: JSON.stringify(result, null, 2) };
  return { code: 0, out: renderReport(result, { file: file === '-' ? '표준 입력' : path.basename(file), all: rest.includes('--all') }) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const r = main(process.argv.slice(2));
  if (r.out) process.stdout.write(r.out + '\n');
  if (r.err) process.stderr.write(r.err + '\n');
  process.exitCode = r.code;
}
