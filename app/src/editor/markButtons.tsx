// The text marks the floating bar and the 편집 도구줄 offer as buttons: 굵게,
// 기울임, 밑줄, 취소선, 방점.

import type { ChainedCommands, Editor } from '@tiptap/core';
import type { ReactNode } from 'react';

export type MarkKey = 'bold' | 'italic' | 'underline' | 'strike' | 'dot';

export interface MarkButton {
  key: MarkKey;
  /** The name on its own, as the 편집 도구줄 shows it. */
  name: string;
  /** Its keyboard shortcut, shown in the floating bar. */
  keys: string;
  glyph: ReactNode;
  toggle: (chain: ChainedCommands) => ChainedCommands;
}

export const MARK_BUTTONS: Record<MarkKey, MarkButton> = {
  bold: { key: 'bold', name: '굵게', keys: 'Ctrl+B', glyph: <b>가</b>, toggle: (c) => c.toggleBold() },
  italic: { key: 'italic', name: '기울임', keys: 'Ctrl+I', glyph: <i>가</i>, toggle: (c) => c.toggleItalic() },
  underline: { key: 'underline', name: '밑줄', keys: 'Ctrl+U', glyph: <u>가</u>, toggle: (c) => c.toggleUnderline() },
  strike: { key: 'strike', name: '취소선', keys: 'Ctrl+Shift+S', glyph: <s>가</s>, toggle: (c) => c.toggleStrike() },
  dot: { key: 'dot', name: '방점', keys: 'Ctrl+Shift+D', glyph: <span className="dot">가</span>, toggle: (c) => c.toggleDot() },
};

/** Which marks the selection has, for a `useEditorState` selector. */
export function activeMarks(e: Editor): Record<MarkKey, boolean> {
  return {
    bold: e.isActive('bold'),
    italic: e.isActive('italic'),
    underline: e.isActive('underline'),
    strike: e.isActive('strike'),
    dot: e.isActive('dot'),
  };
}

/** Turns a mark on or off over the selection, keeping the focus in the text. */
export function toggleMark(editor: Editor, key: MarkKey) {
  MARK_BUTTONS[key].toggle(editor.chain().focus()).run();
}
