import type { Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';

/** First visible character after a scene boundary, skipping blank paragraphs. */
export function sceneTextPosition(doc: PmNode, start: number): number {
  let found: number | null = null;
  let ended = false;
  doc.forEach((node, offset) => {
    if (found !== null || ended || offset < start) return;
    if (node.type.name === 'sceneBreak') { ended = true; return; }
    node.descendants((child, position) => {
      if (found !== null) return false;
      if (child.isText) {
        const at = child.text?.search(/\S/u) ?? -1;
        if (at >= 0) found = offset + 1 + position + at;
      }
      return true;
    });
  });
  return found ?? Math.min(start + 1, doc.content.size);
}

/** Focus without Tiptap's deferred centering; scroll only this editor's view. */
export function goToScene(editor: Editor, start: number) {
  if (editor.isDestroyed) return;
  const pos = sceneTextPosition(editor.state.doc, start);
  editor.commands.setTextSelection(pos);
  editor.view.focus();
  const scroller = editor.view.dom.closest<HTMLElement>('.doc-scroll');
  if (!scroller) return;
  let caret = editor.view.coordsAtPos(pos);
  const page = editor.view.dom.closest<HTMLElement>('.paged-viewport');
  if (page) {
    const step = Number(page.dataset.pageStep) || page.clientWidth + 48;
    const column = Math.max(0, Math.floor((caret.left - page.getBoundingClientRect().left + page.scrollLeft) / step));
    page.scrollLeft = column * step;
    caret = editor.view.coordsAtPos(pos);
  }
  // Even the final scene needs enough space below it to reach the top.
  scroller.style.setProperty('--scene-scroll-room', `${scroller.clientHeight}px`);
  scroller.scrollTo({ top: Math.max(0, scroller.scrollTop + caret.top - scroller.getBoundingClientRect().top - scroller.clientTop), behavior: 'instant' });
}
