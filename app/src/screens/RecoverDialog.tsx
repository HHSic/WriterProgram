// The recovery screen for a project whose project.json is damaged
// (docs/safety-design.md S5). The chapters are files of their own and are
// all still there; only the structure (parts, order, settings) is brought
// back: from another device's copy, from the newest daily backup, or rebuilt
// from the files. The damaged file is never removed.

import { useState } from 'react';
import type { RecoverWay, Recovery, RecoveryChoice } from '../api/types';
import { Modal } from '../components/Modal';
import { num, timeLabel } from '../lib/format';
import { closeDialog, recoverProject } from '../store';

/** "2026-10-02" → "2026년 10월 2일". */
function dayText(day: string): string {
  const [y, m, d] = day.split('-').map(Number);
  return y && m && d ? `${y}년 ${m}월 ${d}일` : day;
}

const counts = (c: RecoveryChoice) => `부 ${num(c.parts)}개, 회차 ${num(c.chapters)}개`;

interface Option {
  way: RecoverWay;
  label: string;
  hint: string;
  ready: boolean;
}

function options(r: Recovery): Option[] {
  const out: Option[] = [];
  if (r.copy) {
    const when = r.copy.modified ? `${timeLabel(r.copy.modified)}에 저장된 ` : '';
    out.push({
      way: 'copy',
      label: '다른 기기 사본으로 되돌리기',
      hint: `${when}사본 ‘${r.copy.name}’ · ${counts(r.copy)}`,
      ready: true,
    });
  }
  out.push({
    way: 'backup',
    label: '가장 최근 백업으로 되돌리기',
    hint: r.backup
      ? `${dayText(r.backup.name)} 백업 · ${counts(r.backup)}. 백업 뒤에 생긴 회차는 마지막 부 끝에 붙입니다.`
      : '백업이 없습니다.',
    ready: r.backup !== null,
  });
  out.push({
    way: 'rebuild',
    label: '원고 파일로 구조 다시 만들기',
    hint: `회차 파일 ${num(r.chapterFiles)}개를 만든 순서대로 부 하나에 넣고, 기획 문서와 설정 카드도 다시 모읍니다. 부 나눔과 작품 설정(서식, 목표)은 처음 값으로 돌아갑니다.`,
    ready: true,
  });
  return out;
}

export function RecoverDialog({ path, recovery }: { path: string; recovery: Recovery }) {
  const list = options(recovery);
  const [way, setWay] = useState<RecoverWay>(list.find((o) => o.ready)!.way);
  const [busy, setBusy] = useState(false);
  const picked = list.find((o) => o.way === way)!;

  const go = async () => {
    if (busy) return;
    setBusy(true);
    if (!(await recoverProject(path, way))) setBusy(false);
  };

  return (
    <Modal
      title="작품을 열 수 없음"
      onClose={closeDialog}
      width={540}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            닫기
          </button>
          <button type="button" className="btn primary" disabled={busy || !picked.ready} onClick={() => void go()}>
            {busy ? '되살리는 중…' : picked.label}
          </button>
        </>
      }
    >
      <div className="form">
        <p className="dialog-text">
          작품 구조 파일(project.json)이 손상되어 열 수 없습니다. 원고 파일은 그대로 있습니다.
        </p>
        <fieldset className="field">
          <legend className="field-label">되살리는 방법</legend>
          <div className="recover-options">
            {list.map((o) => (
              <label key={o.way} className={`kind-option${way === o.way ? ' on' : ''}${o.ready ? '' : ' off'}`}>
                <input type="radio" name="recover" disabled={!o.ready} checked={way === o.way} onChange={() => setWay(o.way)} />
                <strong>{o.label}</strong>
                <small>{o.hint}</small>
              </label>
            ))}
          </div>
        </fieldset>
        <p className="hint">어느 쪽을 골라도 손상된 파일은 지우지 않고 ‘project.json.damaged-시각’ 이름으로 남겨 둡니다.</p>
      </div>
    </Modal>
  );
}
