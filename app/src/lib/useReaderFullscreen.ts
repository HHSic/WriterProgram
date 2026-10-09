import { useEffect, useRef, useState, type RefObject } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Restore only fullscreen entered by this preview. Browser Esc is observed too. */
export function useReaderFullscreen(anchor: RefObject<HTMLElement | null>) {
  const [fullscreen, setFullscreen] = useState(false);
  const [error, setError] = useState('');
  const owned = useRef(false);
  const busy = useRef(false);
  const alive = useRef(true);
  const native = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  useEffect(() => {
    alive.current = true;
    const changed = () => {
      const active = !!document.fullscreenElement && !!anchor.current?.closest('.reader-modal')?.contains(document.fullscreenElement);
      if (!active && !native) owned.current = false;
      setFullscreen(active);
    };
    document.addEventListener('fullscreenchange', changed);
    return () => {
      alive.current = false;
      document.removeEventListener('fullscreenchange', changed);
      if (owned.current) {
        if (native) void getCurrentWindow().setFullscreen(false).catch(() => {});
        else if (document.fullscreenElement) void document.exitFullscreen().catch(() => {});
      }
    };
  }, [anchor, native]);
  const toggle = async () => {
    if (busy.current) return;
    busy.current = true;
    setError('');
    try {
      if (native) {
        const appWindow = getCurrentWindow();
        if (fullscreen) { if (owned.current) await appWindow.setFullscreen(false); owned.current = false; }
        else { owned.current = !(await appWindow.isFullscreen()); await appWindow.setFullscreen(true); }
      } else if (fullscreen) {
        if (document.fullscreenElement) await document.exitFullscreen();
        owned.current = false;
      } else {
        const panel = anchor.current?.closest<HTMLElement>('.reader-modal');
        if (!panel?.requestFullscreen) throw new Error('이 환경에서 전체화면을 사용할 수 없습니다.');
        await panel.requestFullscreen();
        owned.current = true;
      }
      if (alive.current) setFullscreen(!fullscreen);
      else if (owned.current) {
        if (native) await getCurrentWindow().setFullscreen(false);
        else if (document.fullscreenElement) await document.exitFullscreen();
        owned.current = false;
      }
    } catch (e) { if (alive.current) setError(`전체화면 전환 실패: ${e instanceof Error ? e.message : String(e)}`); }
    finally { busy.current = false; }
  };
  return { fullscreen, error, toggle };
}
