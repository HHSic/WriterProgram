// A dialog over the screen (docs/safety-design.md K2).
//
// - Esc closes it, except while a Korean syllable is being composed: that Esc
//   cancels the composition, not the dialog.
// - Clicking outside closes only small confirm and info dialogs
//   (`dismissOnBackdrop`); dialogs with choices to make close with their
//   buttons and Esc, so a stray click does not lose them.
// - With changes not applied yet (`dirty`), Esc and the close button ask first.
// - Focus moves into the dialog, Tab and Shift+Tab go round inside it, and
//   focus goes back where it was when the dialog closes.

import { useEffect, useId, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from 'react';
import { Icon } from './Icon';

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"]), [contenteditable="true"]';

/** Open dialogs, the last on top: only the top one answers keys. */
const stack: string[] = [];

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => !el.closest('[inert], [hidden]'));
}

/** An Esc that ends a composition (한글 조합) rather than meaning "close". */
export function composingKey(e: { isComposing?: boolean; keyCode?: number }): boolean {
  return !!e.isComposing || e.keyCode === 229;
}

export function Modal({
  title,
  onClose,
  children,
  footer,
  width = 480,
  dismissOnBackdrop = false,
  dirty = false,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
  /** Clicking outside closes it: only for small confirm and info dialogs. */
  dismissOnBackdrop?: boolean;
  /** There are changes not applied yet: ask before closing with Esc or the close button. */
  dirty?: boolean;
}) {
  const id = useId();
  const panel = useRef<HTMLDivElement>(null);
  const [asking, setAsking] = useState(false);
  // Read the latest props from the key handler, which is set up once.
  const latest = useRef({ onClose, dirty, asking });
  latest.current = { onClose, dirty, asking };

  const requestClose = () => {
    if (latest.current.dirty) setAsking(true);
    else latest.current.onClose();
  };
  const requestRef = useRef(requestClose);
  requestRef.current = requestClose;

  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    stack.push(id);
    const first =
      panel.current?.querySelector<HTMLElement>('[data-autofocus], input, select, textarea, button.primary') ??
      (panel.current ? focusables(panel.current)[0] : undefined);
    (first ?? panel.current)?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (stack[stack.length - 1] !== id || e.key !== 'Escape') return;
      // The Esc that cancels a 한글 composition must not close the dialog.
      if (composingKey(e)) return;
      e.stopPropagation();
      e.preventDefault();
      if (latest.current.asking) setAsking(false);
      else requestRef.current();
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      const at = stack.lastIndexOf(id);
      if (at >= 0) stack.splice(at, 1);
      // Back to where the writer was, if that is still on screen.
      if (before && before.isConnected) before.focus();
    };
  }, [id]);

  // Keep Tab inside the dialog.
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'Tab' || !panel.current) return;
    const items = focusables(panel.current);
    if (items.length === 0) {
      e.preventDefault();
      panel.current.focus();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    const inside = active instanceof Node && panel.current.contains(active);
    if (e.shiftKey && (active === first || !inside || active === panel.current)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !inside)) {
      e.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className="overlay"
      onMouseDown={(e) => {
        if (dismissOnBackdrop && e.target === e.currentTarget) requestClose();
      }}
    >
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        style={{ width }}
        ref={panel}
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        <header className="modal-head">
          <h2>{title}</h2>
          <button type="button" className="icon-btn" aria-label="닫기" onClick={requestClose}>
            <Icon name="close" />
          </button>
        </header>
        <div className="modal-body">{children}</div>
        {footer && <footer className="modal-foot">{footer}</footer>}
        {asking && <DiscardAsk onKeep={() => setAsking(false)} onDiscard={() => latest.current.onClose()} />}
      </div>
    </div>
  );
}

/** "고친 내용을 버릴까요?" over the dialog's own footer. */
function DiscardAsk({ onKeep, onDiscard }: { onKeep: () => void; onDiscard: () => void }) {
  const keep = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    keep.current?.focus();
    return () => {
      if (before && before.isConnected) before.focus();
    };
  }, []);
  return (
    <div className="modal-ask" role="alertdialog" aria-label="고친 내용을 버릴까요?">
      <p>고친 내용을 버릴까요?</p>
      <div className="row">
        <button ref={keep} type="button" className="btn" onClick={onKeep}>
          계속 고치기
        </button>
        <button type="button" className="btn danger" onClick={onDiscard}>
          버리고 닫기
        </button>
      </div>
    </div>
  );
}
