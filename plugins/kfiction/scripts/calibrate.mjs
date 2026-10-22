#!/usr/bin/env node
// 점검 규칙이 사람 글과 모델 글에서 얼마나 켜지는지 잰다. 사람 글에서도 자주 켜지는 규칙은
// 사람과 모델을 가르지 못하므로 경고로 두지 않는다(references/revision.md "사람 글에서 잰 것").
//   node plugins/kfiction/scripts/calibrate.mjs --group 사람 a.txt b.txt --group 모델 c.txt d.txt
// 글마다 규칙이 켜졌는지와 1,000자당 몇 번인지를 무리별로 보인다. 계약 위반(F1, F2)은 뺀다.
import { readFileSync } from 'node:fs';
import { basename } from 'node:path';
import { lint } from '../lib/lint.js';

const groups = [];
for (const arg of process.argv.slice(2)) {
  if (arg === '--group') groups.push({ name: '', files: [] });
  else if (groups.length && !groups.at(-1).name) groups.at(-1).name = arg;
  else if (groups.length) groups.at(-1).files.push(arg);
}
if (!groups.length || groups.some((g) => !g.files.length)) {
  console.error('사용법: calibrate.mjs --group <이름> <파일…> [--group <이름> <파일…>]');
  process.exit(2);
}

const rows = new Map();
const totals = [];
for (const g of groups) {
  let chars = 0;
  for (const file of g.files) {
    const text = readFileSync(file, 'utf8');
    chars += text.replace(/\s/g, '').length;
    const fired = new Map();
    for (const f of lint(text).findings) {
      if (f.severity === 'contract') continue;
      const key = `${f.rule} ${f.name}`;
      fired.set(key, (fired.get(key) ?? 0) + 1);
    }
    for (const [key, n] of fired) {
      const row = rows.get(key) ?? {};
      const cell = (row[g.name] ??= { texts: 0, count: 0 });
      cell.texts += 1;
      cell.count += n;
      rows.set(key, row);
    }
  }
  totals.push({ name: g.name, texts: g.files.length, chars, files: g.files.map((f) => basename(f)) });
}

for (const t of totals) console.log(`${t.name}: ${t.texts}편, 공백 뺀 ${t.chars.toLocaleString()}자 (${t.files.join(', ')})`);
console.log();
const table = [...rows.keys()].sort().map((key) => {
  const row = { 규칙: key };
  for (const t of totals) {
    const cell = rows.get(key)[t.name] ?? { texts: 0, count: 0 };
    row[`${t.name} 켜진 글`] = `${cell.texts}/${t.texts}`;
    row[`${t.name} 1천자당`] = ((cell.count * 1000) / t.chars).toFixed(2);
  }
  return row;
});
console.table(table);
