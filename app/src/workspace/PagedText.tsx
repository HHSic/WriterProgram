// One editable document flows through CSS page columns. Paging changes no
// document nodes, selection history, or IME input; blank lines use real space.
import { useLayoutEffect, useRef, useState } from 'react';
import { EditorContent, type Editor } from '@tiptap/react';
import type { ManuscriptFormat } from '../api/types';
import type { ViewSettings } from '../lib/view';
import { PAGE_GAP, pageCount, pageGeometry } from '../lib/pagination';

export function PagedText({ editor, format, view }: { editor: Editor; format: ManuscriptFormat; view: ViewSettings }) {
  const sheet = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const [pages, setPages] = useState(1);
  const [page, setPage] = useState(0);
  const step = useRef(1);

  useLayoutEffect(() => {
    const paper = sheet.current;
    const box = viewport.current;
    if (!paper || !box) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const measure = () => {
      if (!alive || editor.isDestroyed || !paper.clientWidth || editor.view.composing) return;
      const geometry = pageGeometry(paper.clientWidth, format);
      paper.style.setProperty('--sheet-left', `${geometry.left}px`);
      paper.style.setProperty('--sheet-right', `${geometry.right}px`);
      paper.style.setProperty('--sheet-top', `${geometry.top}px`);
      paper.style.setProperty('--sheet-bottom', `${geometry.bottom}px`);
      paper.style.setProperty('--sheet-body-height', `${geometry.height}px`);
      paper.style.setProperty('--sheet-body-width', `${geometry.width}px`);
      const content = editor.view.dom;
      const left = content.getBoundingClientRect().left;
      const rects = [...(content.lastElementChild?.getClientRects() ?? [])];
      const extent = rects.length ? Math.max(...rects.map((r) => r.right - left)) : content.scrollWidth;
      const count = pageCount(geometry.width, extent);
      const oldStep = step.current;
      const at = Math.min(count - 1, Math.max(0, Math.round(box.scrollLeft / oldStep)));
      step.current = geometry.width + PAGE_GAP;
      box.dataset.pageStep = String(step.current);
      setPages(count); setPage(at);
      if (Math.abs(oldStep - step.current) > 0.5 || box.scrollLeft > (count - 1) * step.current + 1) box.scrollLeft = at * step.current;
    };
    const schedule = () => { clearTimeout(timer); timer = setTimeout(measure, 80); };
    measure();
    editor.on('update', schedule);
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(schedule);
    observer?.observe(paper);
    paper.addEventListener('compositionend', schedule);
    window.addEventListener('resize', schedule);
    void document.fonts?.ready.then(schedule);
    return () => {
      alive = false; clearTimeout(timer); observer?.disconnect();
      editor.off('update', schedule);
      paper.removeEventListener('compositionend', schedule);
      window.removeEventListener('resize', schedule);
    };
  }, [editor, format, view.font, view.fontSize, view.lineHeight, view.letterSpacing, view.paragraphGap, view.indent, view.width]);

  const turn = (index: number) => {
    const next = Math.max(0, Math.min(pages - 1, index));
    viewport.current?.scrollTo({ left: next * step.current, behavior: 'instant' });
    setPage(next);
  };
  return <>
    <div ref={sheet} className="paged-sheet">
      <div ref={viewport} className="paged-viewport" data-page-step={step.current} onScroll={(e) => {
        setPage(Math.min(pages - 1, Math.max(0, Math.round(e.currentTarget.scrollLeft / step.current))));
      }}>
        <EditorContent editor={editor} />
      </div>
      <span className="sheet-page-number" aria-hidden="true">{page + 1}</span>
    </div>
    <nav className="page-navigation" aria-label="편집 페이지">
      <button type="button" className="btn small" disabled={page === 0} onClick={() => turn(page - 1)}>이전 쪽</button>
      <span role="status">{page + 1} / {pages}쪽</span>
      <button type="button" className="btn small" disabled={page >= pages - 1} onClick={() => turn(page + 1)}>다음 쪽</button>
      <span className="hint">쪽 나눔: Ctrl+Enter</span>
    </nav>
  </>;
}
