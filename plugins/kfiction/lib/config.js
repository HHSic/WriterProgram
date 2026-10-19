// 사용자 설정. 프로젝트마다 두는 hook 상태(.kfiction/state.json)와 달리 사용자 한 사람에 하나다.
//   ~/.kfiction/config.json  { "dir": "G:/내 드라이브/소설" }
// dir 은 원고 폴더. Google Drive 같은 동기화 폴더를 두면 어디서 claude 를 켜도 원고가 그리로 간다.

import { readFileSync, writeFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export function configDir() {
  return process.env.KFICTION_CONFIG_DIR || path.join(os.homedir(), '.kfiction');
}
export const configPath = () => path.join(configDir(), 'config.json');

export function loadConfig() {
  try {
    return JSON.parse(readFileSync(configPath(), 'utf8'));
  } catch {
    return {};
  }
}
export function saveConfig(config) {
  mkdirSync(configDir(), { recursive: true });
  writeFileSync(configPath(), JSON.stringify(config, null, 2) + '\n');
}

const isDir = (p) => existsSync(p) && statSync(p).isDirectory();

/** "~/소설", 상대 경로를 절대 경로로. */
export function expandPath(p, cwd = process.cwd()) {
  const s = String(p).trim();
  if (s === '~' || /^~[\\/]/.test(s)) return path.join(os.homedir(), s.slice(1));
  return path.resolve(cwd, s);
}

/**
 * 원고 폴더를 정한다. 없으면 만들되 바로 위 폴더는 있어야 한다.
 * 드라이브가 꺼져 있을 때 오타 난 경로나 빈 동기화 경로를 로컬에 잘못 만들지 않게 하기 위해서다.
 */
export function setManuscriptDir(p, cwd) {
  const dir = expandPath(p, cwd);
  if (existsSync(dir) && !isDir(dir)) return { ok: false, text: `폴더가 아님: ${dir}` };
  if (!existsSync(dir)) {
    if (!isDir(path.dirname(dir))) return { ok: false, text: `위 폴더가 없음: ${path.dirname(dir)}\nGoogle Drive가 켜져 있는지, 경로가 맞는지 확인합니다.` };
    mkdirSync(dir);
  }
  saveConfig({ ...loadConfig(), dir });
  return { ok: true, dir };
}

export function clearManuscriptDir() {
  const { dir, ...rest } = loadConfig();
  saveConfig(rest);
  return dir;
}

/** 설정된 원고 폴더와 지금 쓸 수 있는지. 설정이 없으면 null. */
export function manuscriptDir() {
  const { dir } = loadConfig();
  return dir ? { dir, present: isDir(dir) } : null;
}

/** 이름만 준 원고("1화.md")가 지금 폴더에 없으면 원고 폴더에서 찾는다. */
export function resolveManuscript(file, cwd = process.cwd()) {
  if (file === '-' || path.isAbsolute(file)) return file;
  const here = path.resolve(cwd, file);
  if (existsSync(here)) return here;
  const m = manuscriptDir();
  const there = m?.present ? path.join(m.dir, file) : null;
  return there && existsSync(there) ? there : here;
}
