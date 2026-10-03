// 완료 회차 잠금: locking and unlocking a chapter, and its menu item.

import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  docUpdateMeta: vi.fn(),
}));
vi.mock('../api', () => ({ api }));

import type { DocSummary, Overview } from '../api/types';
import type { MenuItem } from '../components/Menu';
import { docMenu } from '../workspace/sidebarMenus';
import { setLocked } from './docs';
import { findDoc } from './ui';
import { get, set } from './state';

function chapter(locked: boolean): DocSummary {
  return {
    id: 'c1',
    title: '끝난 회차',
    synopsis: '',
    status: 'done',
    target: null,
    locked,
    counts: { withSpaces: 0, withoutSpaces: 0, manuscriptLines: 0, manuscriptPages: 0, plainMarks: 0, wide: 0, htmlExtra: 0 },
    pages: null,
    modified: null,
  };
}

function overview(locked: boolean): Overview {
  return {
    root: 'R',
    project: { id: 'p', kind: 'webnovel' },
    parts: [{ id: 'part', title: '1부', docs: [chapter(locked)] }],
    planning: [],
  } as unknown as Overview;
}

const locked = () => findDoc(get().overview!, 'c1')!.doc.locked;

beforeEach(() => {
  api.docUpdateMeta.mockReset();
  set({ overview: overview(false), toast: null, saves: {} });
});

describe('setLocked', () => {
  it('stores the lock with the chapter and shows it in the tree', async () => {
    api.docUpdateMeta.mockResolvedValue({});
    await setLocked('c1', true);
    expect(api.docUpdateMeta).toHaveBeenCalledWith('R', 'c1', { locked: true });
    expect(locked()).toBe(true);

    await setLocked('c1', false);
    expect(api.docUpdateMeta).toHaveBeenLastCalledWith('R', 'c1', { locked: false });
    expect(locked()).toBe(false);
  });

  it('leaves the tree as it was when the file could not be written', async () => {
    api.docUpdateMeta.mockRejectedValue('권한 없음');
    await setLocked('c1', true);
    expect(locked()).toBe(false);
    expect(get().toast?.text).toContain('잠그지 못함');
  });
});

describe('chapter menu', () => {
  const labels = (items: MenuItem[]) => items.map((i) => ('label' in i ? i.label : ''));

  it('offers to lock an open chapter and to unlock a locked one', () => {
    const open = overview(false);
    expect(labels(docMenu({ ov: open, expand() {} }, open.parts[0].docs[0], open.parts[0]))).toContain('이 회차 잠그기 (고치지 않게)');
    const shut = overview(true);
    const items = labels(docMenu({ ov: shut, expand() {} }, shut.parts[0].docs[0], shut.parts[0]));
    expect(items).toContain('잠금 풀기');
    expect(items).not.toContain('이 회차 잠그기 (고치지 않게)');
  });
});
