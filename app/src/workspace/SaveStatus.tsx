// 저장 표시 in the top bar: the worst of every save target (store/saves.ts),
// so a card saved fine never hides a chapter that could not be saved.

import { useEffect, useState } from 'react';
import { retryIn } from '../lib/saveReason';
import { useApp } from '../store';
import { openRescueFolder, retryNow, worstSave } from '../store/saves';

export function SaveStatus() {
  const saves = useApp((s) => s.saves);
  const save = worstSave(saves);
  const failing = save.state === 'error';
  const [now, setNow] = useState(() => Date.now());

  // Count down to the next try while something is failing.
  useEffect(() => {
    if (!failing) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [failing]);

  if (!failing) {
    return (
      <span className={`save-state ${save.state}`} role="status">
        <span className="dot-mark" />
        {save.state === 'saving' ? '저장 중' : '저장됨'}
      </span>
    );
  }

  const next = save.retryAt !== undefined ? `${retryIn(save.retryAt - now)} 다시 시도` : '다시 시도 기다리는 중';
  const where = save.failing > 1 ? ` (${save.failing}곳)` : '';
  return (
    <span className="save-state error" role="alert" title={save.error}>
      <span className="dot-mark" />
      {save.rescued ? (
        <span>작품 폴더에 저장하지 못해 비상 위치에 보관했습니다{where}</span>
      ) : (
        <span>
          저장하지 못함{where} · {next}
        </span>
      )}
      <button type="button" className="save-retry" onClick={() => void retryNow()}>
        지금 다시
      </button>
      {save.rescued && (
        <button type="button" className="save-retry" onClick={() => void openRescueFolder()}>
          위치 열기
        </button>
      )}
    </span>
  );
}
