// 창작 일지 in 작품 설정: this device's on/off switch, what the journal holds
// so far, a check that nothing in it was changed, the daily time stamp
// (날짜 증명), keeping each day's last state (창작 과정 보관) and the way to
// the certificate (docs/creation-proof.md).

import { useEffect, useState } from 'react';
import { api } from '../../api';
import type { JournalSettings, JournalSummary } from '../../api/types';
import { anchorResultText, anchorText, journalCheckText, journalSummaryText } from '../../lib/journalText';
import { anchorNow, openDialog, setAnchorAllowed, setJournalEnabled, toastError } from '../../store';

export function JournalField({
  root,
  keepDaily,
  onKeepDaily,
}: {
  root: string;
  /** 창작 과정 보관, saved with the rest of 작품 설정. */
  keepDaily: boolean;
  onKeepDaily: (keep: boolean) => void;
}) {
  const [settings, setSettings] = useState<JournalSettings | null>(null);
  const [summary, setSummary] = useState<JournalSummary | null>(null);
  const [check, setCheck] = useState<{ text: string; ok: boolean } | null>(null);
  const [checking, setChecking] = useState(false);
  const [anchoring, setAnchoring] = useState(false);
  const [anchorNote, setAnchorNote] = useState<string | null>(null);

  const loadSummary = () =>
    api.journalSummary(root).then(
      (s) => setSummary(s),
      () => {},
    );

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

  const allowAnchor = async (allowed: boolean) => {
    const next = await setAnchorAllowed(allowed);
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

  const anchor = async () => {
    if (anchoring) return;
    setAnchoring(true);
    const result = await anchorNow(root, true);
    setAnchoring(false);
    setAnchorNote(result ? anchorResultText(result) : '날짜 증명을 받지 못했습니다. 나중에 다시 시도합니다.');
    void loadSummary();
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

      <label className="check">
        <input
          type="checkbox"
          checked={settings.anchor === true}
          disabled={!settings.enabled}
          onChange={(e) => void allowAnchor(e.target.checked)}
        />
        하루 한 번 날짜 증명 받기
      </label>
      <small className="hint">
        그날까지 쓴 기록의 지문(32바이트)만 공개 시각 인증 기관에 보내, 그날 이 원고가 있었다는 증명을 받습니다. 원고 내용은 보내지
        않습니다. 이 기기에만 적용됩니다.
      </small>
      {settings.anchor === true && summary && (
        <div className="row wrap">
          <span className="grow">{anchorNote ?? anchorText(summary)}</span>
          <button type="button" className="btn small" disabled={anchoring} onClick={() => void anchor()}>
            {anchoring ? '받는 중…' : '지금 받기'}
          </button>
        </div>
      )}

      <label className="check">
        <input type="checkbox" checked={keepDaily} onChange={(e) => onKeepDaily(e.target.checked)} />
        창작 과정 보관
      </label>
      <small className="hint">
        회차마다 하루의 마지막 기록을 지우지 않고 남깁니다. 나중에 그날의 실제 원고를 보여 줄 수 있습니다. 작품 크기가 조금 늘어납니다.
      </small>

      <div className="row wrap">
        <button type="button" className="btn small" onClick={() => openDialog({ kind: 'proof' })}>
          창작 과정 증명서 만들기…
        </button>
      </div>
    </div>
  );
}
