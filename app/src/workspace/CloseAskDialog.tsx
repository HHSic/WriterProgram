// Closing the window while some writing could not be saved
// (docs/safety-design.md S7): try again, keep it in the rescue folder and
// close, or stay. Shown over whatever dialog is open (store `closeAsk`).

import { useState } from 'react';
import { Modal } from '../components/Modal';
import { flushAll } from '../lib/flush';
import { useApp } from '../store';
import { rescueAll, retryNow, unsavedKeys, worstSave } from '../store/saves';

export function CloseAskDialog() {
  const ask = useApp((s) => s.closeAsk);
  const saves = useApp((s) => s.saves);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  if (!ask) return null;
  const worst = worstSave(saves);

  const run = async (fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    setNote(null);
    try {
      await fn();
    } finally {
      setBusy(false);
    }
  };

  const tryAgain = () =>
    run(async () => {
      await retryNow();
      await flushAll();
      if (unsavedKeys().length === 0) ask.resolve(true);
      else setNote('아직 저장하지 못했습니다.');
    });

  const rescueAndClose = () =>
    run(async () => {
      if (await rescueAll()) ask.resolve(true);
      else setNote('비상 위치에도 저장하지 못했습니다. 닫지 말고 디스크 공간이나 폴더를 확인해 주세요.');
    });

  return (
    <Modal
      title="저장하지 못한 글이 있습니다"
      onClose={() => ask.resolve(false)}
      width={480}
      footer={
        <>
          <button type="button" className="btn" disabled={busy} onClick={() => ask.resolve(false)}>
            닫지 않기
          </button>
          <span className="grow" />
          <button type="button" className="btn" disabled={busy} onClick={() => void rescueAndClose()}>
            비상 저장 위치에 두고 닫기
          </button>
          <button type="button" className="btn primary" disabled={busy} onClick={() => void tryAgain()}>
            다시 시도
          </button>
        </>
      }
    >
      <p className="dialog-text">
        {worst.error ? `${worst.error}. ` : ''}
        지금 닫으면 마지막에 고친 내용을 잃을 수 있습니다. 비상 저장 위치(앱 데이터 폴더)에 두면 다음에 작품을 열 때 비교해서
        되살릴 수 있습니다.
      </p>
      {note && <p className="warn-text">{note}</p>}
    </Modal>
  );
}
