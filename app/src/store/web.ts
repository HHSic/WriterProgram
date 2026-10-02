// In-app browser (prototype).

import { api } from '../api';
import { newId } from '../lib/ids';
import { get, set } from './state';
import { showToast, toastError } from './ui';
import { openTarget } from './tabs';
import { addNote } from './notes';

// ---------------------------------------------------------------------------
// In-app browser (prototype): web pages in tabs of the middle column.

const WEB_KEY = 'wp.web.';

export interface WebPage {
  url: string;
  title: string;
}

/** The last page of each browser tab, per project. */
export function webPages(projectId: string): Record<string, WebPage> {
  try {
    return JSON.parse(localStorage.getItem(WEB_KEY + projectId) ?? '{}') as Record<string, WebPage>;
  } catch {
    return {};
  }
}

export function rememberWebPage(projectId: string, id: string, page: Partial<WebPage>) {
  const all = webPages(projectId);
  all[id] = { url: all[id]?.url ?? '', title: all[id]?.title ?? '', ...page };
  try {
    localStorage.setItem(WEB_KEY + projectId, JSON.stringify(all));
  } catch {
    // Not critical.
  }
  set({ webVersion: get().webVersion + 1 });
}

/** Opens a new browser tab in the pane being worked in. */
export function openWeb(url = '') {
  const ov = get().overview;
  const id = newId();
  if (ov && url) rememberWebPage(ov.project.id, id, { url });
  return openTarget({ kind: 'web', id }, { newTab: true });
}

/** 자료로 보관 (prototype): the page goes to 메모함 as a project note. */
export async function clipPage(label: string) {
  try {
    const clip = await api.browserClip(label);
    const lines = [clip.title.trim() || clip.url, clip.url];
    if (clip.text.trim()) lines.push('', `“${clip.text.trim()}”`);
    const note = await addNote({ anchor: 'project', text: lines.join('\n') });
    if (note) showToast({ text: '메모함에 보관함' });
  } catch (e) {
    toastError('보관하지 못함', e);
  }
}
