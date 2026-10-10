import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';

/** Keep the caret on a complete page when typing moves into another column. */
export const PageScroll = Extension.create({
  name: 'pageScroll',
  priority: 110,
  addProseMirrorPlugins() {
    return [new Plugin({ props: { handleScrollToSelection(view) {
      const page = view.dom.closest<HTMLElement>('.paged-viewport');
      if (!page) return false;
      let caret = view.coordsAtPos(view.state.selection.head);
      if (page.dataset.pageStep) {
      const step = Number(page.dataset.pageStep) || page.clientWidth + 48;
      const column = Math.max(0, Math.floor((caret.left - page.getBoundingClientRect().left + page.scrollLeft) / step));
      page.scrollLeft = column * step;
      }
      caret = view.coordsAtPos(view.state.selection.head);
      const scroll = view.dom.closest<HTMLElement>('.doc-scroll');
      if (scroll) {
        const box = scroll.getBoundingClientRect();
        if (caret.top < box.top + 12) scroll.scrollTop += caret.top - box.top - 12;
        else if (caret.bottom > box.bottom - 64) scroll.scrollTop += caret.bottom - box.bottom + 64;
      }
      return true;
    } } })];
  },
});
