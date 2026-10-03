// Recent projects, creating and opening a project, and its parts (부).

import type { Backend } from '../types';
import { bytesOf, oldAutoRecords } from './docs';
import { clone, createProject, forgetRecent, id, newDoc, overview, project, recentItems, touchRecent, wait } from './state';

const MB = 1024 * 1024;

export const projectMethods = {
  async projectSize(root) {
    const p = project(root);
    const writing = bytesOf(p.info) + bytesOf([...p.docs.values()]) + bytesOf([...p.cards.values()]) + bytesOf([...p.notes.values()]);
    const records = bytesOf([...p.records.values()]);
    const tidyFrees = oldAutoRecords(p).reduce((sum, r) => sum + r.size, 0);
    return {
      writing,
      records,
      daily: 0,
      trash: p.trash.length ? bytesOf(p.trash) : 0,
      journal: 0,
      exchanges: 0,
      tidyFrees,
      suggestTidy: records > 20 * MB && records > 10 * writing && tidyFrees > 0,
      diskFree: 64 * 1024 * MB,
    };
  },
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
  async projectRecovery() {
    // The stand-in's project files are never damaged.
    return null;
  },
  async projectRecover() {
    throw '작품 구조 파일(project.json)에 고칠 곳이 없습니다';
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
