// Rescue copies (비상 보관), kept in memory like everything else here.

import type { JSONContent } from '@tiptap/core';
import type { Backend, RescueFile } from '../types';

interface Kept {
  file: RescueFile;
  body: JSONContent | null;
}

const kept = new Map<string, Kept>();

function stamp(ms: number): string {
  const d = new Date(ms);
  const two = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}${two(d.getMonth() + 1)}${two(d.getDate())}-${two(d.getHours())}${two(d.getMinutes())}${two(d.getSeconds())}`;
}

export const rescueMethods = {
  async rescueSave(projectId, item, since, content) {
    const path = `C:\\Users\\작가\\AppData\\Roaming\\WriterProgram\\rescue\\${projectId}\\${item}-${stamp(since)}.md`;
    kept.set(path, {
      file: { item, path, saved: new Date().toISOString() },
      body: 'body' in content ? content.body : null,
    });
    return path;
  },
  async rescueList(projectId) {
    return [...kept.values()]
      .filter((k) => k.file.path.includes(`\\rescue\\${projectId}\\`))
      .map((k) => k.file)
      .sort((a, b) => b.saved.localeCompare(a.saved));
  },
  async rescueLoad(path) {
    const body = kept.get(path)?.body;
    if (!body) throw '비상 보관 파일이 아님';
    return body;
  },
  async rescueSetAside(path) {
    kept.delete(path);
  },
  async rescueFolder(projectId) {
    return `C:\\Users\\작가\\AppData\\Roaming\\WriterProgram\\rescue\\${projectId}`;
  },
} satisfies Partial<Backend>;
