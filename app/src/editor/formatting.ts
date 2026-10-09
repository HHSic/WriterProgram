// Visible manuscript formatting, persisted by the Rust markup reader/writer.
import { commands as coreCommands, Extension, Mark, mergeAttributes } from '@tiptap/core';
import { editorKeys } from '../lib/shortcuts';

export type ParagraphAlign = 'left' | 'center' | 'right' | 'justify';
export const TEXT_COLORS = ['#C0392B', '#C46A10', '#23854B', '#2365B8', '#7240A0', '#7F5262', '#666666'];
export const validColor = (value: unknown): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value);
const validAlign = (value: unknown) => ['left', 'center', 'right', 'justify'].includes(String(value));

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    paragraphFormat: {
      setParagraphAlign: (align: ParagraphAlign | null) => ReturnType;
      insertPageBreak: () => ReturnType;
      removePageBreak: () => ReturnType;
    };
    textColor: {
      setTextColor: (color: string) => ReturnType;
      unsetTextColor: () => ReturnType;
    };
  }
}

export const ParagraphFormat = Extension.create({
  name: 'paragraphFormat',
  addGlobalAttributes() {
    return [{ types: ['paragraph'], attributes: {
      align: { default: null, keepOnSplit: true,
        parseHTML: (el: HTMLElement) => validAlign(el.dataset.align) ? el.dataset.align : null,
        renderHTML: (attrs: Record<string, unknown>) => validAlign(attrs.align) ? { 'data-align': attrs.align, style: `text-align: ${attrs.align}` } : {} },
      pageBreakBefore: { default: false, keepOnSplit: false,
        parseHTML: (el: HTMLElement) => el.dataset.pageBreak === '1',
        renderHTML: (attrs: Record<string, unknown>) => attrs.pageBreakBefore ? { 'data-page-break': '1' } : {} },
    } }];
  },
  addCommands() {
    return {
      // Tiptap's middle-of-paragraph split copies attributes even when
      // keepOnSplit is false. A page boundary belongs only to the first part.
      splitBlock: (options) => (props) => {
        const hadBreak = props.tr.selection.$from.parent.attrs.pageBreakBefore === true;
        if (!coreCommands.splitBlock(options)(props)) return false;
        const from = props.tr.selection.$from;
        if (props.dispatch && hadBreak && from.depth && from.parent.type.name === 'paragraph') {
          props.tr.setNodeMarkup(from.before(), undefined, { ...from.parent.attrs, pageBreakBefore: false });
        }
        return true;
      },
      setParagraphAlign: (align) => ({ tr, dispatch }) => {
        if (align !== null && !validAlign(align)) return false;
        tr.doc.nodesBetween(tr.selection.from, tr.selection.to, (node, pos) => {
          if (node.type.name !== 'paragraph') return true;
          if (dispatch) tr.setNodeMarkup(pos, undefined, { ...node.attrs, align });
          return false;
        });
        return true;
      },
      insertPageBreak: () => ({ chain, state }) => {
        const empty = state.selection.empty && state.selection.$from.parent.type.name === 'paragraph' && state.selection.$from.parent.content.size === 0;
        const commands = chain();
        if (!empty) commands.splitBlock();
        return commands.updateAttributes('paragraph', { pageBreakBefore: true }).run();
      },
      removePageBreak: () => ({ commands }) => commands.updateAttributes('paragraph', { pageBreakBefore: false }),
    };
  },
  addKeyboardShortcuts() {
    return {
      ...editorKeys('alignCenter', () => this.editor.commands.setParagraphAlign('center')),
      ...editorKeys('alignLeft', () => this.editor.commands.setParagraphAlign('left')),
      ...editorKeys('alignRight', () => this.editor.commands.setParagraphAlign('right')),
      ...editorKeys('pageBreak', () => this.editor.commands.insertPageBreak()),
    };
  },
});

export const TextColor = Mark.create({
  name: 'textColor',
  addAttributes() {
    return { color: { default: '#C0392B', parseHTML: (el: HTMLElement) => validColor(el.dataset.color) ? el.dataset.color : '#C0392B' } };
  },
  parseHTML() { return [{ tag: 'span[data-color]', getAttrs: (el) => validColor((el as HTMLElement).dataset.color) ? null : false }]; },
  renderHTML({ HTMLAttributes }) {
    const color = validColor(HTMLAttributes.color) ? HTMLAttributes.color.toUpperCase() : '#000000';
    return ['span', mergeAttributes({ 'data-color': color, style: `color: ${color}` }), 0];
  },
  addCommands() {
    return { setTextColor: (color) => ({ commands }) => validColor(color) && commands.setMark(this.name, { color: color.toUpperCase() }),
      unsetTextColor: () => ({ commands }) => commands.unsetMark(this.name) };
  },
});
