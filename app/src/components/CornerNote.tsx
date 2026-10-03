// A small note in the bottom-right corner, apart from the toast in the
// middle: quiet news that needs no attention now (a date proof came) or a
// question that can wait (받을까요?). One at a time; a newer one replaces it.

import { useEffect, useState } from 'react';
import { useApp } from '../store';
import { Icon } from './Icon';

export function CornerNote() {
  const note = useApp((s) => s.cornerNote);
  const [hover, setHover] = useState(false);

  useEffect(() => {
    if (!note || hover || note.stay) return;
    const timer = setTimeout(() => useApp.setState({ cornerNote: null }), note.ms ?? 2500);
    return () => clearTimeout(timer);
  }, [note, hover]);

  if (!note) return null;
  const close = () => {
    useApp.setState({ cornerNote: null });
    note.onClose?.();
  };
  return (
    <div className="corner-note" role="status" aria-live="polite" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}>
      <span className="corner-note-text">{note.text}</span>
      {note.actions && note.actions.length > 0 && (
        <span className="corner-note-actions">
          {note.actions.map((a) => (
            <button
              key={a.label}
              type="button"
              className="btn small"
              onClick={() => {
                useApp.setState({ cornerNote: null });
                a.run();
              }}
            >
              {a.label}
            </button>
          ))}
        </span>
      )}
      <button type="button" className="icon-btn" aria-label="닫기" onClick={close}>
        <Icon name="close" size={12} />
      </button>
    </div>
  );
}
