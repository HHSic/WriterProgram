import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  projectFileSave: vi.fn(), projectFileRescue: vi.fn(), pickSaveFile: vi.fn(), reveal: vi.fn(),
}));
vi.mock('../api', () => ({ api }));

import { projectMutated } from '../api/projectChanges';
import type { Overview } from '../api/types';
import { flushAll, registerFlusher } from '../lib/flush';
import { get, set } from './state';
import { rescueAll, resetSaves, unsavedKeys } from './saves';
import { finishProjectFile, saveProjectFileAs, startProjectFile, stopProjectFile } from './wpm';

const ov = { root: '/working/project', projectFile: '/documents/book.wpm', project: { id: 'proj1', title: '달빛 아래' } } as Overview;

beforeEach(() => {
  vi.useFakeTimers();
  set({ overview: ov, saves: {}, toast: null });
  for (const fn of Object.values(api)) fn.mockReset();
  api.projectFileSave.mockResolvedValue('/documents/book.wpm');
  api.projectFileRescue.mockResolvedValue('/rescue/proj1/book.wpm');
});

afterEach(async () => {
  api.projectFileSave.mockResolvedValue('/documents/book.wpm');
  await finishProjectFile();
  stopProjectFile();
  resetSaves();
  vi.useRealTimers();
});

describe('native project file saving', () => {
  it('coalesces changes in the open project and updates the linked file', async () => {
    startProjectFile(ov);
    projectMutated('/different/project');
    await vi.advanceTimersByTimeAsync(1300);
    expect(api.projectFileSave).not.toHaveBeenCalled();
    projectMutated(ov.root);
    projectMutated(ov.root);
    expect(get().saves[`wpm:${ov.root}`].state).toBe('saving');
    await vi.advanceTimersByTimeAsync(1300);
    expect(api.projectFileSave).toHaveBeenCalledExactlyOnceWith(ov.root);
    expect(get().saves[`wpm:${ov.root}`].state).toBe('saved');
  });

  it('packages only after the pending editor writes finish on Ctrl+S/close', async () => {
    startProjectFile(ov);
    const order: string[] = [];
    const unregister = registerFlusher(async () => {
      await Promise.resolve();
      order.push('text');
      projectMutated(ov.root);
    });
    api.projectFileSave.mockImplementation(async () => { order.push('package'); return ov.projectFile; });
    try { await flushAll(); } finally { unregister(); }
    expect(order).toEqual(['text', 'package']);
    expect(unsavedKeys()).toEqual([]);
  });

  it('holds a failed package for retry and can keep an independently openable rescue', async () => {
    api.projectFileSave.mockRejectedValue('디스크 공간 부족');
    startProjectFile(ov);
    projectMutated(ov.root);
    await flushAll();
    expect(unsavedKeys()).toContain(`wpm:${ov.root}`);
    expect(get().saves[`wpm:${ov.root}`].state).toBe('error');
    expect(await rescueAll()).toBe(true);
    expect(api.projectFileRescue).toHaveBeenCalledWith(ov.root);
    api.projectFileSave.mockResolvedValue(ov.projectFile);
    await vi.advanceTimersByTimeAsync(2100);
    expect(unsavedKeys()).toEqual([]);
  });

  it('Save As recovers from an unwritable old file and binds future autosaves to the new one', async () => {
    api.projectFileSave.mockRejectedValue('읽기 전용 위치');
    startProjectFile(ov);
    projectMutated(ov.root);
    await flushAll();
    api.pickSaveFile.mockResolvedValue('/documents/new.wpm');
    api.projectFileSave.mockResolvedValue('/documents/new.wpm');
    await saveProjectFileAs();
    expect(api.projectFileSave).toHaveBeenCalledWith(ov.root, '/documents/new.wpm');
    expect(get().overview?.projectFile).toBe('/documents/new.wpm');
    expect(unsavedKeys()).toEqual([]);
    projectMutated(ov.root);
    await flushAll();
    expect(api.projectFileSave).toHaveBeenLastCalledWith(ov.root);
  });

  it('Save As refuses to package stale writing after an editor save failed', async () => {
    set({ saves: { 'doc:a': { state: 'error', error: 'not saved' } } });
    api.pickSaveFile.mockResolvedValue('/documents/new.wpm');
    await saveProjectFileAs();
    expect(api.projectFileSave).not.toHaveBeenCalled();
    expect(get().overview?.projectFile).toBe(ov.projectFile);
  });

  it('cancelling Save As leaves the current file and save state alone', async () => {
    startProjectFile(ov);
    api.pickSaveFile.mockResolvedValue(null);
    await saveProjectFileAs();
    expect(api.projectFileSave).not.toHaveBeenCalled();
    expect(get().overview?.projectFile).toBe(ov.projectFile);
  });
});
