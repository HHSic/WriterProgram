// 문단 모양 and special spaces in the manuscript editor.
//
// A paragraph can be set in from the left and right as a whole, counted in
// characters (letters, poems, a 상태창), and can have its own first line:
// in (들여쓰기), out (내어쓰기, the other lines set in) or flush. The file
// keeps it as `<p data-left="2" data-right="1" data-indent="-1">`
// (crates/core/src/markup.rs ParaAttrs). A paragraph without its own first
// line follows the indent and its rules (editor/indent.ts).
// 묶음 빈칸 (U+00A0) keeps two words on one line; 고정폭 빈칸 (U+2002) keeps
// its width when lines are justified.

import { Extension, type CommandProps } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import type { EditorState } from '@tiptap/pm/state';

/** Widest margin, in characters (same as ParaAttrs::MAX). */
export const MAX_MARGIN = 20;
/** Deepest first line either way, in characters (ParaAttrs::MAX_INDENT). */
export const MAX_FIRST_LINE = 10;

export const NO_BREAK_SPACE = ' ';
export const FIXED_SPACE = ' ';
export const FULL_WIDTH_SPACE = '　';

export type Side = 'left' | 'right';

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    paragraphMargins: {
      /** Adds `delta` characters to the chosen sides of the selected paragraphs. */
      shiftMargins: (delta: number, sides?: Side[]) => ReturnType;
      /** Sets the margins of the selected paragraphs. */
      setMargins: (margins: Partial<Record<Side, number>>) => ReturnType;
      /**
       * Sets the first line of the selected paragraphs: characters in
       * (negative: 내어쓰기), or null to follow the indent and its rules.
       */
      setFirstLine: (chars: number | null) => ReturnType;
    };
    specialSpaces: {
      insertSpace: (space: string) => ReturnType;
    };
  }
}

function clamp(n: number): number {
  return Math.max(0, Math.min(MAX_MARGIN, Math.round(Number.isFinite(n) ? n : 0)));
}

function clampFirst(n: unknown): number | null {
  if (n === null || n === undefined || n === '') return null;
  const v = Number(n);
  if (!Number.isFinite(v)) return null;
  return Math.max(-MAX_FIRST_LINE, Math.min(MAX_FIRST_LINE, Math.round(v)));
}

const firstLineAttribute = {
  default: null,
  keepOnSplit: true,
  parseHTML: (el: HTMLElement) => clampFirst(el.getAttribute('data-indent')),
  renderHTML: (attrs: Record<string, unknown>) => {
    const n = clampFirst(attrs.indent);
    return n === null ? {} : { 'data-indent': String(n), style: `--pi: ${n}` };
  },
};

function marginAttribute(side: Side) {
  const data = `data-${side}`;
  const variable = side === 'left' ? '--pl' : '--pr';
  return {
    default: 0,
    keepOnSplit: true,
    parseHTML: (el: HTMLElement) => clamp(Number(el.getAttribute(data))),
    renderHTML: (attrs: Record<string, unknown>) => {
      const n = clamp(Number(attrs[side]));
      return n ? { [data]: String(n), style: `${variable}: ${n}` } : {};
    },
  };
}

export const ParagraphMargins = Extension.create({
  name: 'paragraphMargins',

  addGlobalAttributes() {
    return [
      {
        types: ['paragraph'],
        attributes: { left: marginAttribute('left'), right: marginAttribute('right'), indent: firstLineAttribute },
      },
    ];
  },

  addCommands() {
    const change =
      (next: (attrs: Record<Side, number>) => Record<Side, number>) =>
      ({ tr, dispatch }: CommandProps) => {
        const { from, to } = tr.selection;
        let changed = false;
        tr.doc.nodesBetween(from, to, (node: PmNode, pos: number) => {
          if (node.type.name !== 'paragraph') return true;
          const now = { left: clamp(Number(node.attrs.left)), right: clamp(Number(node.attrs.right)) };
          const want = next(now);
          if (want.left !== now.left || want.right !== now.right) {
            changed = true;
            if (dispatch) tr.setNodeMarkup(pos, undefined, { ...node.attrs, ...want });
          }
          return false;
        });
        return changed;
      };
    return {
      shiftMargins:
        (delta, sides = ['left']) =>
          change((m) => ({
            left: sides.includes('left') ? clamp(m.left + delta) : m.left,
            right: sides.includes('right') ? clamp(m.right + delta) : m.right,
          })),
      setMargins: (margins) =>
        change((m) => ({ left: clamp(margins.left ?? m.left), right: clamp(margins.right ?? m.right) })),
      setFirstLine:
        (chars) =>
        ({ tr, dispatch }: CommandProps) => {
          const want = clampFirst(chars);
          const { from, to } = tr.selection;
          let changed = false;
          tr.doc.nodesBetween(from, to, (node: PmNode, pos: number) => {
            if (node.type.name !== 'paragraph') return true;
            if (clampFirst(node.attrs.indent) !== want) {
              changed = true;
              if (dispatch) tr.setNodeMarkup(pos, undefined, { ...node.attrs, indent: want });
            }
            return false;
          });
          return changed;
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-]': () => this.editor.commands.shiftMargins(1),
      'Mod-[': () => this.editor.commands.shiftMargins(-1),
    };
  },
});

export const SpecialSpaces = Extension.create({
  name: 'specialSpaces',

  addCommands() {
    return {
      insertSpace:
        (space) =>
        ({ tr, dispatch }) => {
          if (dispatch) tr.insertText(space);
          return true;
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      // 묶음 빈칸: Word's Ctrl+Shift+Space, and 한글's Alt+Space when Windows lets it through.
      'Mod-Shift-Space': () => this.editor.commands.insertSpace(NO_BREAK_SPACE),
      'Alt-Space': () => this.editor.commands.insertSpace(NO_BREAK_SPACE),
      // 고정폭 빈칸
      'Alt-Shift-Space': () => this.editor.commands.insertSpace(FIXED_SPACE),
    };
  },
});

/** Margins of the paragraph the cursor is in. */
export function marginsAt(state: EditorState): Record<Side, number> {
  const node = state.selection.$from.parent;
  if (node.type.name !== 'paragraph') return { left: 0, right: 0 };
  return { left: clamp(Number(node.attrs.left)), right: clamp(Number(node.attrs.right)) };
}

/** The own first line of the paragraph the cursor is in (null: follows the indent). */
export function firstLineAt(state: EditorState): number | null {
  const node = state.selection.$from.parent;
  return node.type.name === 'paragraph' ? clampFirst(node.attrs.indent) : null;
}
