import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DriveLink, Overview, SyncChoices, SyncReport } from '../api/types';

const projectSync = vi.fn();
vi.mock('../api', () => ({ api: { projectSync } }));

const { answerRemovals, putOffRemovals, syncNow } = await import('./drives');
const { useApp } = await import('./state');

const link: DriveLink = { provider: 'google', folder: '달빛 서점', linkedAt: '', syncedAt: null, error: null };

function report(patch: Partial<SyncReport> = {}): SyncReport {
  return {
    uploaded: [],
    downloaded: [],
    removedHere: [],
    removedThere: [],
    copies: [],
    merged: false,
    later: [],
    heldThere: [],
    heldHere: [],
    spaceLeft: null,
    ...patch,
  };
}

const gone = Array.from({ length: 32 }, (_, i) => `manuscript/c${i}.md`);

beforeEach(() => {
  projectSync.mockReset();
  useApp.setState({
    overview: { root: '/작품', project: { id: 'p1' } } as Overview,
    link,
    syncing: false,
    dialog: null,
    toast: null,
    heldRemovals: null,
  });
});

describe('many removals at once', () => {
  it('asks the writer, and sends the answer with the next pass', async () => {
    projectSync.mockResolvedValueOnce({ link, report: report({ heldThere: gone }) });
    await syncNow({ quiet: true });
    expect(useApp.getState().dialog).toEqual({ kind: 'removals' });
    expect(useApp.getState().heldRemovals).toEqual({ there: gone, here: [] });

    projectSync.mockResolvedValueOnce({ link, report: report({ removedThere: gone }) });
    await answerRemovals(true);
    const choices: SyncChoices = projectSync.mock.calls[1][2];
    expect(choices).toEqual({ remove: gone, keep: [] });
    expect(useApp.getState().heldRemovals).toBeNull();
    expect(useApp.getState().dialog).toBeNull();
  });

  it('brings the files back when the writer says no', async () => {
    projectSync.mockResolvedValueOnce({ link, report: report({ heldHere: gone }) });
    await syncNow({ quiet: true });
    projectSync.mockResolvedValueOnce({ link, report: report() });
    await answerRemovals(false);
    expect(projectSync.mock.calls[1][2]).toEqual({ remove: [], keep: gone });
  });

  it('does not ask again on quiet passes once put off, until something else is held', async () => {
    projectSync.mockResolvedValue({ link, report: report({ heldThere: gone }) });
    await syncNow({ quiet: true });
    putOffRemovals();
    expect(useApp.getState().dialog).toBeNull();
    await syncNow({ quiet: true });
    expect(useApp.getState().dialog).toBeNull();
    // Still held: nothing was removed.
    expect(useApp.getState().heldRemovals?.there).toHaveLength(32);

    projectSync.mockResolvedValue({ link, report: report({ heldThere: [...gone, 'manuscript/x.md'] }) });
    await syncNow({ quiet: true });
    expect(useApp.getState().dialog).toEqual({ kind: 'removals' });
  });
});
