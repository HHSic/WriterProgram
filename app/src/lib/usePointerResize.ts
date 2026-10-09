import { useEffect, useRef, type PointerEvent as ReactPointerEvent } from 'react';

/** A resize drag survives leaving the handle and always releases on cancel/unmount. */
export function usePointerResize() {
  const cleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => cleanup.current?.(), []);
  return (event: ReactPointerEvent<HTMLElement>, move: (e: PointerEvent) => void) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    cleanup.current?.();
    const id = event.pointerId;
    const onMove = (e: PointerEvent) => { if (e.pointerId === id) { e.preventDefault(); move(e); } };
    const end = (e?: PointerEvent) => { if (e && e.pointerId !== id) return; stop(); };
    const stop = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      window.removeEventListener('blur', stop);
      document.documentElement.classList.remove('resizing-panes');
      cleanup.current = null;
    };
    cleanup.current = stop;
    document.documentElement.classList.add('resizing-panes');
    window.addEventListener('pointermove', onMove, { passive: false });
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    window.addEventListener('blur', stop);
  };
}
