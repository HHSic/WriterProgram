import { useEffect, useState } from 'react';
import { useApp } from '../store';
import { Icon } from './Icon';

export function ToastHost() {
  const toast = useApp((s) => s.toast);
  const [hover, setHover] = useState(false);

  useEffect(() => {
    if (!toast || hover) return;
    // Longer when there is something to do (되돌리기) or read (an error).
    const ms = toast.tone === 'error' || toast.action ? 9000 : 5000;
    const timer = setTimeout(() => useApp.setState({ toast: null }), ms);
    return () => clearTimeout(timer);
  }, [toast, hover]);

  if (!toast) return null;
  return (
    <div
      className={`toast${toast.tone === 'error' ? ' error' : ''}`}
      role="status"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
    >
      <span>{toast.text}</span>
      {toast.action && (
        <button
          type="button"
          className="toast-action"
          onClick={() => {
            toast.action!.run();
            useApp.setState({ toast: null });
          }}
        >
          {toast.action.label}
        </button>
      )}
      <button type="button" className="icon-btn" aria-label="닫기" onClick={() => useApp.setState({ toast: null })}>
        <Icon name="close" size={14} />
      </button>
    </div>
  );
}
