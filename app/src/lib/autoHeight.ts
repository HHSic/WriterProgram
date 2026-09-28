// Text boxes that grow with their text (synopsis, card description, notes).
// They are measured again when their width changes, e.g. when the window or
// a split pane is resized, so the height never stays at an old width's.

import { useLayoutEffect, type RefObject } from 'react';

export function useAutoHeight(ref: RefObject<HTMLTextAreaElement | null>, value: string, min = 0) {
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const fit = () => {
      el.style.height = 'auto';
      el.style.height = `${Math.max(el.scrollHeight, min)}px`;
    };
    fit();
    let width = el.clientWidth;
    const observer = new ResizeObserver(() => {
      if (el.clientWidth === width) return;
      width = el.clientWidth;
      fit();
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, value, min]);
}
