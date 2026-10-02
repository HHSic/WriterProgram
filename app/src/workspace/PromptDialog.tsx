// A small dialog asking for one line of text (a name, a number).

import { useState } from 'react';
import { Modal } from '../components/Modal';
import { closeDialog, type Dialog } from '../store';

export function PromptDialog({ dialog }: { dialog: Extract<Dialog, { kind: 'prompt' }> }) {
  const [value, setValue] = useState(dialog.value);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (busy) return;
    setBusy(true);
    await dialog.onSubmit(value);
    closeDialog();
  };
  return (
    <Modal
      title={dialog.title}
      onClose={closeDialog}
      width={420}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="submit" form="prompt-form" className="btn primary" disabled={busy}>
            {dialog.confirm}
          </button>
        </>
      }
    >
      <form
        id="prompt-form"
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <label className="field">
          <span className="field-label">{dialog.label}</span>
          <input
            data-autofocus
            value={value}
            inputMode={dialog.inputMode}
            onChange={(e) => setValue(e.target.value)}
            onFocus={(e) => e.target.select()}
          />
        </label>
      </form>
    </Modal>
  );
}
