// Google Drive, OneDrive and Dropbox: connecting, and linking a project to a drive folder.

import type { Backend, DriveInfo, DriveLink, DriveProvider } from '../types';
import { now, overview, project, projects, touchRecent, wait } from './state';

/** Drives in the browser preview: all "registered", none connected. */
const drives: DriveInfo[] = [
  { provider: 'google', label: 'Google Drive', registered: true, account: null, connectedAt: null },
  { provider: 'onedrive', label: 'OneDrive', registered: true, account: null, connectedAt: null },
  { provider: 'dropbox', label: 'Dropbox', registered: false, account: null, connectedAt: null },
];
const links = new Map<string, DriveLink>();
let connecting = false;

function drive(provider: DriveProvider): DriveInfo {
  return drives.find((d) => d.provider === provider)!;
}

export const driveMethods = {
  async driveStatus() {
    return {
      drives: drives.map((d) => ({ ...d, account: d.account ? { ...d.account } : null })),
      appsFile: 'C:\\Users\\작가\\AppData\\Roaming\\com.writerprogram.desktop\\drive-apps.json',
    };
  },
  async driveConnect(provider) {
    const d = drive(provider);
    if (!d.registered) throw `${d.label} 앱 등록 전이라 아직 연결할 수 없음`;
    connecting = true;
    for (let i = 0; i < 12 && connecting; i++) await new Promise((r) => setTimeout(r, 100));
    if (!connecting) throw '연결을 그만둠';
    connecting = false;
    d.account = { name: '윤서하', email: 'writer@example.com' };
    d.connectedAt = now();
    return { ...d.account };
  },
  async driveCancel() {
    connecting = false;
  },
  async driveDisconnect(provider) {
    const d = drive(provider);
    d.account = null;
    d.connectedAt = null;
    for (const [id, link] of links) if (link.provider === provider) links.delete(id);
  },
  async driveProjects(provider) {
    if (!drive(provider).account) throw '연결되어 있지 않음';
    return [...projects.values()]
      .filter((p) => [...links.entries()].some(([id, l]) => id === p.info.id && l.provider === provider))
      .map((p) => ({ folder: p.info.title, title: p.info.title, id: p.info.id }));
  },
  async driveFetch(_provider, folder) {
    await wait();
    const root = [...projects.keys()].find((r) => project(r).info.title === folder);
    if (!root) throw '드라이브의 이 폴더에는 작품이 없음';
    touchRecent(root);
    return overview(root);
  },
  async projectLinkGet(projectId) {
    const link = links.get(projectId);
    return link ? { ...link } : null;
  },
  async projectLink(projectId, title, provider) {
    if (!drive(provider).account) throw `${drive(provider).label}에 연결되어 있지 않음`;
    const link: DriveLink = { provider, folder: title, linkedAt: now(), syncedAt: null, error: null };
    links.set(projectId, link);
    return { ...link };
  },
  async projectUnlink(projectId) {
    links.delete(projectId);
  },
  async projectSync(_root, projectId) {
    await wait();
    const link = links.get(projectId);
    if (!link) throw '이 작품은 드라이브와 맞추지 않음';
    link.syncedAt = now();
    link.error = null;
    return {
      link: { ...link },
      report: {
        uploaded: [],
        downloaded: [],
        removedHere: [],
        removedThere: [],
        copies: [],
        merged: false,
        later: [],
        // The preview's Google Drive is nearly full, to show the warning.
        spaceLeft: link.provider === 'google' ? 31 * 1024 * 1024 : null,
      },
    };
  },
} satisfies Partial<Backend>;
