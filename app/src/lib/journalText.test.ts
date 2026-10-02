import { describe, expect, it } from 'vitest';
import type { JournalSummary } from '../api/types';
import { anchorResultText, anchorText, journalCheckText, journalSummaryText } from './journalText';

const empty: JournalSummary = {
  since: null,
  saves: 0,
  sessions: 0,
  pastes: 0,
  imports: 0,
  exchanges: 0,
  anchors: 0,
  lastAnchor: null,
  devices: 0,
  thisDevice: 0,
};

describe('time stamp sentences', () => {
  it('says when the last one was signed', () => {
    expect(anchorText(empty)).toBe('아직 날짜 증명을 받지 않았습니다.');
    expect(anchorText({ ...empty, anchors: 6, lastAnchor: '2026-10-02T03:00:00.000Z' })).toBe(
      '마지막 날짜 증명: 2026년 10월 2일',
    );
  });

  it('says what asking came to', () => {
    expect(anchorResultText({ state: 'signed', signed: ['DigiCert', 'FreeTSA'] })).toBe(
      'DigiCert, FreeTSA에서 날짜 증명을 받았습니다.',
    );
    expect(anchorResultText({ state: 'unchanged', signed: [] })).toContain('바뀐 것이 없');
    expect(anchorResultText({ state: 'offline', signed: [] })).toContain('나중에');
  });
});

describe('journal sentences', () => {
  it('says how long and how often', () => {
    expect(journalSummaryText(empty)).toContain('아직 기록이 없습니다');
    const s = { ...empty, since: '2026-10-02T12:00:00.000Z', saves: 1234, devices: 2, thisDevice: 10 };
    expect(journalSummaryText(s)).toBe('2026년 10월 2일부터 1,234번 저장했습니다. 다른 기기 1대의 기록도 함께 있습니다.');
  });

  it('says where a check failed', () => {
    expect(journalCheckText({ ok: true, files: [{ device: 'a', lines: 3, firstBad: null }] }, 'a')).toContain('빠진 곳이 없습니다');
    const bad = journalCheckText(
      {
        ok: false,
        files: [
          { device: 'a', lines: 3, firstBad: [12, 'broken'] },
          { device: 'b', lines: 3, firstBad: null },
        ],
      },
      'a',
    );
    expect(bad).toBe('이 기기 기록의 12번째 항목이 앞 항목과 이어지지 않습니다. 그 앞이 고쳐졌거나 빠졌습니다.');
  });
});
