import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AnchorResult, JournalSettings, Overview } from '../api/types';

const journalAnchor = vi.fn<(root: string, occasion: string) => Promise<AnchorResult>>();
const journalSettings = vi.fn<() => Promise<JournalSettings>>();
vi.mock('../api', () => ({ api: { journalAnchor, journalSettings } }));
vi.mock('../editor/journal', () => ({ setJournalOn: () => {} }));

const { noteManuscriptOut, noteStatusChange, stampIfDue } = await import('./journal');
const { useApp } = await import('./state');

const settings = (patch: Partial<JournalSettings> = {}): JournalSettings => ({
  device: 'dev000000001',
  enabled: true,
  noticed: true,
  anchor: true,
  anchorAsk: false,
  anchorNotify: true,
  ...patch,
});
const flush = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => {
  journalAnchor.mockReset();
  journalSettings.mockReset();
  journalSettings.mockResolvedValue(settings());
  useApp.setState({ overview: { root: '/작품' } as Overview, cornerNote: null });
});

describe('date proofs (날짜 증명)', () => {
  it('says so in the corner when one came, unless the writer turned that off', async () => {
    journalAnchor.mockResolvedValue({ state: 'signed', signed: ['DigiCert'] });
    await stampIfDue('/작품', 'check');
    await flush();
    expect(useApp.getState().cornerNote?.text).toContain('날짜 증명을 받았습니다');

    useApp.setState({ cornerNote: null });
    journalSettings.mockResolvedValue(settings({ anchorNotify: false }));
    await stampIfDue('/작품', 'check');
    await flush();
    expect(useApp.getState().cornerNote).toBeNull();
  });

  it('says nothing when none was due', async () => {
    for (const state of ['notYet', 'enough', 'unchanged', 'offline'] as const) {
      journalAnchor.mockResolvedValue({ state, signed: [] });
      await stampIfDue('/작품', 'check');
    }
    await flush();
    expect(useApp.getState().cornerNote).toBeNull();
  });

  it('asks in the corner with 물어보고 받기, and 받기 takes it now', async () => {
    journalAnchor.mockResolvedValueOnce({ state: 'ask', signed: [] });
    await stampIfDue('/작품', 'moment');
    const note = useApp.getState().cornerNote!;
    expect(note.text).toContain('받을까요');
    expect(note.stay).toBe(true);
    journalAnchor.mockResolvedValueOnce({ state: 'signed', signed: ['DigiCert'] });
    note.actions!.find((a) => a.label === '받기')!.run();
    await flush();
    expect(journalAnchor).toHaveBeenLastCalledWith('/작품', 'now');
  });

  it('takes finishing a chapter and sending a manuscript as moments', async () => {
    journalAnchor.mockResolvedValue({ state: 'unchanged', signed: [] });
    noteStatusChange('draft');
    expect(journalAnchor).not.toHaveBeenCalled();
    noteStatusChange('done');
    noteStatusChange('published');
    noteManuscriptOut();
    expect(journalAnchor.mock.calls.map((c) => c[1])).toEqual(['moment', 'moment', 'moment']);
  });
});
