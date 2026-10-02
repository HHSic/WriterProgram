// A dialog asking the writer to confirm an action.

import { useState } from 'react';
import { Modal } from '../components/Modal';
import { closeDialog, useApp, type Dialog } from '../store';

export function ConfirmDialog({ dialog }: { dialog: Extract<Dialog, { kind: 'confirm' }> }) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title={dialog.title}
      onClose={closeDialog}
      width={440}
      footer={
        <>
          <button
            type="button"
            className="btn"
            onClick={() => {
              dialog.onCancel?.();
              closeDialog();
            }}
          >
            {dialog.cancel ?? '취소'}
          </button>
          <button
            type="button"
            className={`btn primary${dialog.danger ? ' danger' : ''}`}
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              await dialog.onConfirm();
              // onConfirm may have opened the dialog to go back to.
              if (useApp.getState().dialog === dialog) closeDialog();
            }}
          >
            {dialog.confirm}
          </button>
        </>
      }
    >
      <p className="dialog-text">{dialog.message}</p>
    </Modal>
  );
}
