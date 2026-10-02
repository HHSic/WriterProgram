// 창작 일지 in 작품 설정: this device's on/off switch, what the journal holds
// so far, and a check that nothing in it was changed (docs/creation-proof.md).

import { useEffect, useState } from 'react';
import { api } from '../../api';
import type { JournalSettings, JournalSummary } from '../../api/types';
import { journalCheckText, journalSummaryText } from '../../lib/journalText';
import { setJournalEnabled, toastError } from '../../store';

export function JournalField({ root }: { root: string }) {
  const [settings, setSettings] = useState<JournalSettings | null>(null);
  const [summary, setSummary] = useState<JournalSummary | null>(null);
  const [check, setCheck] = useState<{ text: string; ok: boolean } | null>(null);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    let alive = true;
    api.journalSettings().then(
      (s) => alive && setSettings(s),
      () => {},
    );
    api.journalSummary(root).then(
      (s) => alive && setSummary(s),
      () => {},
    );
    return () => {
      alive = false;
    };
  }, [root]);

  const toggle = async (enabled: boolean) => {
    const next = await setJournalEnabled(enabled);
    if (next) setSettings(next);
  };

  const verify = async () => {
    if (!settings || checking) return;
    setChecking(true);
    try {
      const report = await api.journalVerify(root);
      setCheck({ text: journalCheckText(report, settings.device), ok: report.ok });
    } catch (e) {
      toastError('기록을 확인하지 못함', e);
    } finally {
      setChecking(false);
    }
  };

  if (!settings) return null;
  return (
    <div className="field">
      <span className="field-label">창작 일지</span>
      <label className="check">
        <input type="checkbox" checked={settings.enabled} onChange={(e) => void toggle(e.target.checked)} />
        쓰는 과정을 이 기기에 기록하기
      </label>
      <small className="hint">
        원고 내용은 기록하지 않고, 언제 얼마나 쓰고 고쳤는지만 남깁니다. 나중에 내가 이 작품을 써 왔다는 자료가 됩니다. 이 기기의
        모든 작품에 적용됩니다.
      </small>
      {summary && (
        <div className="row wrap">
          <span className="grow">{journalSummaryText(summary)}</span>
          {summary.devices > 0 && (
            <button type="button" className="btn small" disabled={checking} onClick={() => void verify()}>
              {checking ? '확인하는 중…' : '기록 확인'}
            </button>
          )}
        </div>
      )}
      {check && <small className={check.ok ? 'hint' : 'warn-text'}>{check.text}</small>}
    </div>
  );
}
