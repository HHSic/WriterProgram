// Recent projects, creating and opening a project, and its parts (부).

import type { Backend } from '../types';
import { clone, createProject, forgetRecent, id, newDoc, overview, project, recentItems, touchRecent, wait } from './state';

export const projectMethods = {
  async recentList() {
    await wait();
    return clone(recentItems());
  },
  async recentRemove(path) {
    forgetRecent(path);
  },
  async defaultLocation() {
    return 'C:\\Users\\작가\\Documents\\WriterProgram';
  },
  async projectCreate(opts) {
    await wait();
    const title = opts.title.trim();
    if (!title) throw '작품 제목을 적어 주세요';
    const root = createProject(opts.parent, title, opts.kind, opts.perDocGoal, opts.countSpaces);
    const p = project(root);
    if (opts.firstChapter) p.parts[0].docs.push(newDoc(p, 'manuscript', '').meta.id);
    touchRecent(root);
    return overview(root);
  },
  async projectOpen(path) {
    await wait();
    touchRecent(path);
    return overview(path);
  },
  async projectOverview(root) {
    return overview(root);
  },
  async projectUpdate(root, patch) {
    const p = project(root);
    Object.assign(p.info, patch);
    return clone(p.info);
  },
  async partAdd(root, title) {
    const p = project(root);
    const part = { id: id(), title: title.trim() || `${p.parts.length + 1}부`, docs: [] };
    p.parts.push(part);
    return part.id;
  },
  async partRename(root, partId, title) {
    const part = project(root).parts.find((x) => x.id === partId);
    if (!part) throw '부를 찾을 수 없음';
    if (!title.trim()) throw '부 이름을 적어 주세요';
    part.title = title.trim();
  },
  async partRemove(root, partId) {
    const p = project(root);
    const part = p.parts.find((x) => x.id === partId);
    if (!part) throw '부를 찾을 수 없음';
    if (part.docs.length) throw '회차가 남아 있는 부는 지울 수 없음. 회차를 먼저 옮기거나 휴지통으로 보내 주세요.';
    if (p.parts.length === 1) throw '부가 하나뿐이라 지울 수 없음';
    p.parts = p.parts.filter((x) => x.id !== partId);
  },
} satisfies Partial<Backend>;
