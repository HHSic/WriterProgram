#!/usr/bin/env node
// ChatGPT용 묶음을 만든다. 원본은 skills/kfiction 하나뿐이고 여기서 바꿔 쓴다.
//   node integrations/chatgpt/build.mjs [--out <dir>]   (기본 integrations/chatgpt/dist)
// 만드는 것:
//   kfiction-skill.zip   맨 위 폴더 하나(kfiction/)에 SKILL.md 와 references/. 스킬 올리기·@skill-creator 용
//   instructions.md      프로젝트·GPT 지침 칸에 붙일 한 파일 (8,000자 안)
//   knowledge/*.md       프로젝트·GPT 지식 파일로 올릴 참고 문서
// ChatGPT에서는 명령을 실행할 수 없으므로 "원고 점검" 절은 뺀다.
import { readFileSync, writeFileSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { zip } from './zip.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const SKILL = path.join(ROOT, 'skills', 'kfiction');
// Windows에서 git이 줄바꿈을 CRLF로 바꿔 받아도 같은 결과가 나오게 LF로 맞춘다.
const read = (f) => readFileSync(f, 'utf8').replace(/\r\n/g, '\n');
export const INSTRUCTIONS_LIMIT = 8000;

/** "## 원고 점검" 절을 지운다. 명령을 못 돌리는 곳에서는 쓸모가 없다. */
export function withoutChecker(md) {
  return md.replace(/\n## 원고 점검\n[\s\S]*?(?=\n## |$)/, '\n').trimEnd() + '\n';
}

/** 참고 문서 링크를 지식 파일 이름으로 바꾼다. */
const toKnowledge = (md) => md.replace(/\[references\/([\w-]+\.md)\]\(references\/[\w-]+\.md\)/g, '지식 파일 `$1`');

const body = (md) => md.replace(/^---\n[\s\S]*?\n---\n/, '');

export function instructions() {
  const skill = toKnowledge(withoutChecker(body(read(path.join(SKILL, 'SKILL.md')))));
  const section = (file, heading) => read(path.join(SKILL, 'references', file)).match(new RegExp(`## ${heading}\\n[\\s\\S]*?(?=\\n## |$)`))[0].trim();
  const parts = [skill.trim(), section('context.md', '관계 표'), section('context.md', '장면 카드'), section('speech-levels.md', '어미 표').replace('## 어미 표', '## 말 단계 어미 표')];
  return parts.join('\n\n') + '\n';
}

export function build(out = path.join(HERE, 'dist')) {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(path.join(out, 'knowledge'), { recursive: true });
  const refs = readdirSync(path.join(SKILL, 'references')).filter((f) => f.endsWith('.md')).sort();
  const files = [{ name: 'kfiction/SKILL.md', data: withoutChecker(read(path.join(SKILL, 'SKILL.md'))) }];
  for (const f of refs) {
    const text = read(path.join(SKILL, 'references', f));
    files.push({ name: `kfiction/references/${f}`, data: text });
    writeFileSync(path.join(out, 'knowledge', f), text);
  }
  writeFileSync(path.join(out, 'kfiction-skill.zip'), zip(files));
  const text = instructions();
  writeFileSync(path.join(out, 'instructions.md'), text);
  return { out, instructionsChars: [...text].length, files: files.map((f) => f.name) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const i = process.argv.indexOf('--out');
  const r = build(i > 0 ? path.resolve(process.argv[i + 1]) : undefined);
  process.stdout.write(`만듦: ${r.out}\n  kfiction-skill.zip (${r.files.length}개 파일)\n  instructions.md (${r.instructionsChars}자)\n  knowledge/ (${r.files.length - 1}개)\n`);
  if (r.instructionsChars > INSTRUCTIONS_LIMIT) {
    process.stderr.write(`경고: instructions.md가 ${INSTRUCTIONS_LIMIT}자를 넘음. GPT 지침 칸에 다 들어가지 않을 수 있음\n`);
    process.exitCode = 1;
  }
}
