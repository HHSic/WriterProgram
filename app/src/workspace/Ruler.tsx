// 눈금자 over the manuscript, as in 한글: the text column in characters, and
// the shape of the paragraph the cursor is in. Drag the marks to change it
// (every selected paragraph follows):
//   ▭ bottom left   왼쪽 여백 (the paragraph as a whole)
//   ▽ top left      첫 줄 들여쓰기
//   △ bottom left   내어쓰기 (the lines after the first)
//   ◁ right         오른쪽 여백
// Double-click 첫 줄 to go back to the manuscript format's indent.

import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { useEditorState, type Editor } from '@tiptap/react';
import { indentKindAt } from '../editor/indent';
import { MAX_FIRST_LINE, MAX_MARGIN } from '../editor/paragraph';
import { useApp } from '../store';

type Mark = 'left' | 'first' | 'hang' | 'right';

const LABEL: Record<Mark, string> = {
  left: '왼쪽 여백',
  first: '첫 줄 들여쓰기',
  hang: '내어쓰기',
  right: '오른쪽 여백',
};

interface Column {
  /** Where the text column starts, from the ruler's left edge (px). */
  x: number;
  width: number;
  /** One character, letter spacing included (px). */
  char: number;
}

function measure(editor: Editor, bar: HTMLElement): Column | null {
  if (editor.isDestroyed) return null;
  const dom = editor.view.dom as HTMLElement;
  const style = getComputedStyle(dom);
  const size = parseFloat(style.fontSize);
  const tracking = parseFloat(style.letterSpacing) || 0;
  const r = dom.getBoundingClientRect();
  const b = bar.getBoundingClientRect();
  if (!size || !r.width) return null;
  return { x: r.left - b.left, width: r.width, char: size + tracking };
}

export function Ruler({ editor, planning }: { editor: Editor; planning: boolean }) {
  const indent = useApp((s) => s.view.indent);
  const rules = useApp((s) => s.overview!.project.manuscriptFormat.indentRules);
  const bar = useRef<HTMLDivElement>(null);
  const [col, setCol] = useState<Column | null>(null);
  const [drag, setDrag] = useState<{ mark: Mark; at: number } | null>(null);

  const shape = useEditorState({
    editor,
    selector: ({ editor: e }) => {
      const { $from } = e.state.selection;
      const node = $from.parent;
      if (node.type.name !== 'paragraph' || $from.depth < 1) return null;
      const own = node.attrs.indent === null || node.attrs.indent === undefined ? null : Number(node.attrs.indent);
      const kind = own === null && !planning ? indentKindAt(e.state.doc, rules ?? null, $from.index(0)) : null;
      return {
        left: Number(node.attrs.left) || 0,
        right: Number(node.attrs.right) || 0,
        own,
        // What the first line does now, own or from the format.
        first: own ?? (kind === 'flush' || kind === 'hang' ? 0 : indent),
        hangFromRules: kind === 'hang' ? indent : 0,
        editable: e.isEditable,
      };
    },
  });

  useEffect(() => {
    const el = bar.current;
    if (!el) return;
    const update = () => setCol(measure(editor, el));
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    observer.observe(editor.view.dom);
    return () => observer.disconnect();
  }, [editor]);

  if (!col) return <div ref={bar} className="ruler" aria-hidden="true" />;

  const chars = Math.floor(col.width / col.char);
  const s = shape ?? { left: 0, right: 0, own: null, first: indent, hangFromRules: 0, editable: false };
  const left = s.left + s.hangFromRules;
  const pos: Record<Mark, number> = {
    left,
    first: left + Math.max(0, s.first),
    hang: left + Math.max(0, -s.first),
    right: chars - s.right,
  };
  const px = (c: number) => col.x + c * col.char;

  const start = (mark: Mark) => (e: ReactPointerEvent) => {
    if (!s.editable || !shape) return;
    e.preventDefault();
    e.stopPropagation();
    const target = e.currentTarget as HTMLElement;
    try {
      target.setPointerCapture(e.pointerId);
    } catch {
      // A pointer the browser no longer knows: the drag still follows the events.
    }
    const at = (clientX: number) => {
      const b = bar.current!.getBoundingClientRect();
      return Math.round((clientX - b.left - col.x) / col.char);
    };
    setDrag({ mark, at: pos[mark] });
    const move = (ev: PointerEvent) => setDrag({ mark, at: at(ev.clientX) });
    const up = (ev: PointerEvent) => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', up);
      target.removeEventListener('pointercancel', up);
      setDrag(null);
      if (ev.type === 'pointercancel') return;
      const c = at(ev.clientX);
      const chain = editor.chain().focus();
      switch (mark) {
        case 'left':
          chain.setMargins({ left: Math.max(0, Math.min(MAX_MARGIN, c - s.hangFromRules)) }).run();
          break;
        case 'right':
          chain.setMargins({ right: Math.max(0, Math.min(MAX_MARGIN, chars - c)) }).run();
          break;
        case 'first':
          chain.setFirstLine(Math.max(0, Math.min(MAX_FIRST_LINE, c - left))).run();
          break;
        case 'hang':
          chain.setFirstLine(-Math.max(0, Math.min(MAX_FIRST_LINE, c - left))).run();
          break;
      }
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', up);
    target.addEventListener('pointercancel', up);
  };

  const ticks = [];
  for (let c = 0; c <= chars; c++) {
    const major = c % 5 === 0;
    ticks.push(
      <span key={c} className={`ruler-tick${major ? ' major' : ''}`} style={{ left: px(c) }}>
        {c % 10 === 0 && c > 0 ? <i>{c}</i> : null}
      </span>,
    );
  }

  const guide = drag ? px(drag.at) : null;
  const dragLabel = drag
    ? drag.mark === 'right'
      ? `${LABEL.right} ${Math.max(0, chars - drag.at)}자`
      : drag.mark === 'left'
        ? `${LABEL.left} ${Math.max(0, drag.at - s.hangFromRules)}자`
        : `${LABEL[drag.mark]} ${Math.max(0, drag.at - left)}자`
    : null;
  const firstTitle =
    s.own === null
      ? `첫 줄: 서식대로 ${s.first}자 · 끌어서 이 문단만 바꾸기`
      : `첫 줄: 이 문단 ${s.first >= 0 ? `들여쓰기 ${s.first}자` : `내어쓰기 ${-s.first}자`} · 두 번 눌러 서식대로`;

  return (
    <div ref={bar} className={`ruler${s.editable ? '' : ' locked'}`} role="group" aria-label="눈금자">
      <div className="ruler-column" style={{ left: col.x, width: col.width }} />
      <div className="ruler-margin" style={{ left: col.x, width: pos.left * col.char }} />
      <div className="ruler-margin" style={{ left: px(pos.right), width: s.right * col.char }} />
      {ticks}
      {shape && (
        <>
          <button
            type="button"
            className={`ruler-mark first${s.own === null ? ' auto' : ''}`}
            style={{ left: px(pos.first) }}
            title={firstTitle}
            aria-label={firstTitle}
            onPointerDown={start('first')}
            onDoubleClick={() => editor.chain().focus().setFirstLine(null).run()}
          />
          <button
            type="button"
            className="ruler-mark hang"
            style={{ left: px(pos.hang) }}
            title={`나머지 줄 · 끌어서 내어쓰기`}
            aria-label="내어쓰기"
            onPointerDown={start('hang')}
          />
          <button
            type="button"
            className="ruler-mark left"
            style={{ left: px(pos.left) }}
            title={`왼쪽 여백 ${s.left}자 · 끌어서 문단 통째로 들이기`}
            aria-label="왼쪽 여백"
            onPointerDown={start('left')}
          />
          <button
            type="button"
            className="ruler-mark right"
            style={{ left: px(pos.right) }}
            title={`오른쪽 여백 ${s.right}자`}
            aria-label="오른쪽 여백"
            onPointerDown={start('right')}
          />
        </>
      )}
      {guide !== null && (
        <>
          <div className="ruler-guide" style={{ left: guide }} />
          <div className="ruler-bubble" style={{ left: guide }}>
            {dragLabel}
          </div>
        </>
      )}
    </div>
  );
}
