import { FontSizeInput } from '../components/FontSizeInput';
import { validColor } from '../editor/formatting';
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { DocData } from '../api/types';
import { Modal } from '../components/Modal';
import { BOOK_GAP, bookColumns, bookLayout, loadBookPreferences, saveBookPreferences, type BookPreferences } from '../lib/book';
import { UNTITLED, docNumber } from '../lib/labels';
import { allManuscript, closeDialog, saveEverything, useApp } from '../store';

export function ReaderDialog() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const chapters = allManuscript(ov);
  const [chapterId, setChapterId] = useState(() => chapters.find((c) => c.id === activeDocId)?.id ?? chapters[0]?.id ?? '');
  const chapterIndex = chapters.findIndex((c) => c.id === chapterId);
  const [preferences, setPreferences] = useState(loadBookPreferences);
  const [loaded, setLoaded] = useState<{ id: string; data: DocData } | null>(null);
  const [error, setError] = useState('');
  const [retry, setRetry] = useState(0);
  const [enterAtEnd, setEnterAtEnd] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => { saveBookPreferences(preferences); }, [preferences]);
  useEffect(() => {
    if (!chapterId) return;
    let alive = true;
    setLoaded(null);
    setLoading(true);
    setError('');
    void (async () => {
      try {
        if (!(await saveEverything())) throw new Error('원고를 저장하지 못했습니다. 저장 상태를 확인한 뒤 다시 시도하세요.');
        if (!alive) return;
        const data = await api.docLoad(ov.root, chapterId);
        if (alive) setLoaded({ id: chapterId, data });
      } catch (e) {
        if (alive) setError(e instanceof Error ? e.message : String(e));
      } finally { if (alive) setLoading(false); }
    })();
    return () => { alive = false; };
  }, [ov.root, chapterId, retry]);

  const goChapter = (index: number, end = false) => {
    if (!chapters[index]) return;
    setEnterAtEnd(end);
    setChapterId(chapters[index].id);
  };
  const update = (patch: Partial<BookPreferences>) => setPreferences((old) => ({ ...old, ...patch }));

  return (
    <Modal title="책 미리보기" className="reader-modal" width={1160} onClose={closeDialog}
      footer={<><span className="hint grow">원고를 고치려면 편집 화면으로 돌아가세요.</span><button type="button" className="btn" onClick={closeDialog}>편집으로 돌아가기</button></>}>
      <div className="reader-controls">
        <div className="segmented" role="group" aria-label="읽기 화면">
          {([['auto', '자동'], ['phone', '휴대폰'], ['pc', 'PC']] as const).map(([id, label]) => (
            <button type="button" key={id} className={`btn${preferences.device === id ? ' on' : ''}`}
              aria-pressed={preferences.device === id} onClick={() => update({ device: id })}>{label}</button>
          ))}
        </div>
        <label className="reader-chapter-select">목차
          <select aria-label="읽을 원고" value={chapterId} disabled={!chapters.length}
            onChange={(e) => goChapter(chapters.findIndex((c) => c.id === e.target.value))}>
            {chapters.map((c, i) => <option key={c.id} value={c.id}>{docNumber(ov.project.kind, i + 1)} · {c.title || UNTITLED}</option>)}
          </select>
        </label>
        <div className="reader-type-controls" role="group" aria-label="읽기 글자 크기">
          <button type="button" className="btn" aria-label="글자 작게" disabled={preferences.fontSize <= 1}
            onClick={() => update({ fontSize: Math.max(1, preferences.fontSize - 2) })}>가−</button>
          <FontSizeInput label="읽기 글자 크기(px)" value={preferences.fontSize} onChange={(fontSize) => update({ fontSize })} /><span>px</span>
          <button type="button" className="btn" aria-label="글자 크게"
            onClick={() => update({ fontSize: preferences.fontSize + 2 })}>가+</button>
        </div>
        <label>배경 <select aria-label="읽기 배경" value={preferences.theme}
          onChange={(e) => update({ theme: e.target.value as BookPreferences['theme'] })}>
          <option value="paper">종이</option><option value="light">밝게</option><option value="dark">어둡게</option>
        </select></label>
      </div>
      <div className="reader-project-title">{ov.project.title}</div>
      {!chapters.length ? <p className="reader-message">읽을 원고가 없습니다. 원고를 먼저 만들어 주세요.</p>
        : error ? <div className="reader-message" role="alert"><p>{error}</p><button type="button" className="btn" onClick={() => setRetry((n) => n + 1)}>다시 시도</button></div>
        : loading || !loaded || loaded.id !== chapterId ? <p className="reader-message" role="status">원고를 불러오는 중…</p>
        : <BookPages key={loaded.id} body={loaded.data.body} preferences={preferences} enterAtEnd={enterAtEnd}
          heading={`${docNumber(ov.project.kind, chapterIndex + 1)} · ${loaded.data.meta.title || UNTITLED}`}
          hasPrevious={chapterIndex > 0} hasNext={chapterIndex < chapters.length - 1}
          onPrevious={() => goChapter(chapterIndex - 1, true)} onNext={() => goChapter(chapterIndex + 1)} />}
    </Modal>
  );
}

/** Only known text marks are rendered. Manuscript text never becomes HTML. */
function inline(node: JSONContent, key: number): ReactNode {
  if (node.type === 'hardBreak') return <br key={key} />;
  if (node.type !== 'text') return null;
  let text: ReactNode = node.text ?? '';
  for (const mark of node.marks ?? []) {
    if (mark.type === 'bold') text = <strong>{text}</strong>;
    else if (mark.type === 'italic') text = <em>{text}</em>;
    else if (mark.type === 'strike') text = <s>{text}</s>;
    else if (mark.type === 'underline') text = <u>{text}</u>;
    else if (mark.type === 'textColor' && validColor(mark.attrs?.color)) text = <span style={{ color: mark.attrs.color }}>{text}</span>;
    else if (mark.type === 'dot') text = <span className="dot">{text}</span>;
  }
  return <span key={key}>{text}</span>;
}

function BookPages({ body, heading, preferences, enterAtEnd, hasPrevious, hasNext, onPrevious, onNext }: {
  body: JSONContent; heading: string; preferences: BookPreferences; enterAtEnd: boolean;
  hasPrevious: boolean; hasNext: boolean; onPrevious: () => void; onNext: () => void;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const article = useRef<HTMLElement>(null);
  const [layout, setLayout] = useState({ columns: 1, pages: 1, spreads: 1, step: 1 });
  const [spread, setSpread] = useState(0);
  const reading = useRef({ layout, spread });
  reading.current = { layout, spread };
  const firstLayout = useRef(true);
  const pointer = useRef<{ x: number; y: number } | null>(null);

  useLayoutEffect(() => {
    const view = viewport.current;
    const content = article.current;
    if (!view || !content) return;
    let alive = true;
    const measure = () => {
      if (!alive || !view.clientWidth) return;
      const width = view.clientWidth;
      const columns = bookColumns(width, preferences.device);
      content.style.columnWidth = `${(width - BOOK_GAP * (columns - 1)) / columns}px`;
      // The final paragraph may fragment over several columns. Its final
      // rectangle gives the actual last page, even for an odd desktop spread.
      const left = content.getBoundingClientRect().left;
      const rects = [...(content.lastElementChild?.getClientRects() ?? [])];
      const extent = rects.length ? Math.max(...rects.map((r) => r.right - left)) : content.scrollWidth;
      const next = bookLayout(width, extent, columns);
      const old = reading.current;
      const page = old.spread * old.layout.columns;
      const position = firstLayout.current && enterAtEnd ? next.spreads - 1
        : Math.min(next.spreads - 1, Math.floor(page / columns));
      firstLayout.current = false;
      reading.current = { layout: next, spread: position };
      setLayout(next);
      setSpread(position);
    };
    measure();
    const observer = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(measure) : null;
    observer?.observe(view);
    window.addEventListener('resize', measure);
    void document.fonts?.ready.then(measure);
    return () => { alive = false; observer?.disconnect(); window.removeEventListener('resize', measure); };
  }, [preferences.device, preferences.fontSize, enterAtEnd]);

  const previous = () => { if (spread > 0) setSpread(spread - 1); else if (hasPrevious) onPrevious(); };
  const next = () => { if (spread < layout.spreads - 1) setSpread(spread + 1); else if (hasNext) onNext(); };
  const actions = useRef({ previous, next });
  actions.current = { previous, next };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.keyCode === 229 || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.target instanceof Element && e.target.closest('input, select, textarea, button, [contenteditable="true"]')) return;
      if (!['ArrowLeft', 'ArrowRight', 'PageUp', 'PageDown'].includes(e.key)) return;
      e.preventDefault();
      e.stopPropagation();
      if (e.key === 'ArrowLeft' || e.key === 'PageUp') actions.current.previous();
      else actions.current.next();
    };
    window.addEventListener('keydown', key, true);
    return () => window.removeEventListener('keydown', key, true);
  }, []);

  const firstPage = spread * layout.columns + 1;
  const lastPage = Math.min(layout.pages, firstPage + layout.columns - 1);
  return (
    <div className={`reader-book reader-${preferences.device}`} data-reader-theme={preferences.theme}
      style={{ '--reader-font-size': `${preferences.fontSize}px` } as CSSProperties}>
      <div className="reader-paper" data-columns={layout.columns}>
        <div className="reader-viewport" tabIndex={0} aria-label="책 본문" ref={viewport}
          onClick={() => viewport.current?.focus()}
          onPointerDown={(e) => { if (e.pointerType === 'touch' || e.pointerType === 'pen') pointer.current = { x: e.clientX, y: e.clientY }; }}
          onPointerCancel={() => { pointer.current = null; }}
          onPointerUp={(e) => {
            const start = pointer.current;
            pointer.current = null;
            if (!start || window.getSelection()?.toString()) return;
            const dx = e.clientX - start.x;
            const dy = e.clientY - start.y;
            if (Math.abs(dx) >= 50 && Math.abs(dx) > Math.abs(dy) * 1.5) { if (dx < 0) next(); else previous(); }
          }}>
          <article className="reader-pages" ref={article} style={{ transform: `translateX(${-spread * layout.step}px)` }}>
            <h3>{heading}</h3>
            {(body.content ?? []).map((node, i) => node.type === 'sceneBreak'
              ? <div className="reader-scene" key={i} role="separator" aria-label="장면 나눔">***</div>
              : <p key={i} style={{ textAlign: ['left', 'center', 'right', 'justify'].includes(node.attrs?.align) ? node.attrs?.align : undefined,
                breakBefore: node.attrs?.pageBreakBefore ? 'column' : undefined,
                marginLeft: `${Math.max(0, Math.min(20, Number(node.attrs?.left) || 0))}em`,
                marginRight: `${Math.max(0, Math.min(20, Number(node.attrs?.right) || 0))}em`,
                textIndent: node.attrs?.align === 'center' || node.attrs?.align === 'right' ? 0 : node.attrs?.indent == null ? undefined : `${Math.max(-10, Math.min(10, Number(node.attrs.indent) || 0))}em`,
                paddingLeft: Number(node.attrs?.indent) < 0 ? `${Math.min(10, -Number(node.attrs?.indent))}em` : undefined,
              }}>{node.content?.length ? node.content.map(inline) : '\u00a0'}</p>)}
          </article>
        </div>
      </div>
      <nav className="reader-navigation" aria-label="책 넘기기">
        <button type="button" className="btn" disabled={spread === 0 && !hasPrevious} onClick={previous}>이전</button>
        <span role="status" aria-live="polite">{firstPage === lastPage ? firstPage : `${firstPage}–${lastPage}`} / {layout.pages}쪽</span>
        <button type="button" className="btn" disabled={spread === layout.spreads - 1 && !hasNext} onClick={next}>다음</button>
      </nav>
      <p className="reader-key-hint">← → 키로 넘기기 · 터치 화면에서는 좌우로 밀기</p>
    </div>
  );
}
