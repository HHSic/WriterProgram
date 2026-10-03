// Asks the writer about many files removed at once on one side (a folder
// moved or emptied by mistake): remove them on the other side too, bring
// them back, or decide later. The pass held those removals back
// (crates/sync engine.rs, store/drives.ts).

import { useState } from 'react';
import { Modal } from '../components/Modal';
import { keepText, removalQuestion } from '../lib/syncText';
import { answerRemovals, putOffRemovals, useApp } from '../store';

export function RemovalsDialog() {
  const held = useApp((s) => s.heldRemovals);
  const syncing = useApp((s) => s.syncing);
  const [busy, setBusy] = useState(false);
  if (!held) return null;

  const answer = async (remove: boolean) => {
    setBusy(true);
    try {
      await answerRemovals(remove);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="한꺼번에 지우기 확인"
      onClose={putOffRemovals}
      width={460}
      footer={
        <>
          <button type="button" className="btn" data-autofocus onClick={putOffRemovals} disabled={busy}>
            나중에
          </button>
          <button type="button" className="btn" onClick={() => void answer(false)} disabled={busy || syncing}>
            되살리기
          </button>
          <button type="button" className="btn primary danger" onClick={() => void answer(true)} disabled={busy || syncing}>
            지우기
          </button>
        </>
      }
    >
      <p className="dialog-text">{removalQuestion(held.there, held.here)}</p>
      <p className="hint">
        {keepText(held.there, held.here)} 나중에를 고르면 지금은 아무것도 지우지 않고, 나머지 글은 계속 맞춥니다.
      </p>
    </Modal>
  );
}
