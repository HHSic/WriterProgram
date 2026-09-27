// Editor schema: paragraphs, line breaks, scene breaks and a few marks. It
// must stay in line with crates/core/src/markup.rs, which writes the files.

import { Mark, Node, mergeAttributes, type Extensions } from '@tiptap/core';
import { Placeholder } from '@tiptap/extensions';
import StarterKit from '@tiptap/starter-kit';

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    sceneBreak: {
      insertSceneBreak: () => ReturnType;
    };
    dot: {
      toggleDot: () => ReturnType;
    };
  }
}

/** A paragraph with only one of these, followed by Enter, becomes a scene break. */
const SCENE_MARKERS = ['***', '* * *', '◆', '◇', '◆◆◆', '◇◇◇', '＊＊＊', '###', '---'];

export const SceneBreak = Node.create<{ symbol: string }>({
  name: 'sceneBreak',
  group: 'block',
  atom: true,
  selectable: true,

  addOptions() {
    return { symbol: '◆' };
  },

  parseHTML() {
    return [{ tag: 'div[data-scene-break]' }, { tag: 'hr' }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(HTMLAttributes, {
        'data-scene-break': '',
        class: 'scene-break',
        contenteditable: 'false',
        'aria-label': '장면 나눔',
      }),
      ['span', { class: 'scene-symbol' }, this.options.symbol],
    ];
  },

  addCommands() {
    return {
      insertSceneBreak:
        () =>
        ({ chain }) =>
          chain().insertContent({ type: this.name }).run(),
    };
  },

  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        if (!empty || $from.parent.type.name !== 'paragraph') return false;
        if ($from.parentOffset !== $from.parent.content.size) return false;
        const text = $from.parent.textContent.trim();
        if (!SCENE_MARKERS.includes(text) && text !== this.options.symbol) return false;
        const start = $from.before();
        return editor
          .chain()
          .insertContentAt({ from: start, to: $from.after() }, [{ type: this.name }, { type: 'paragraph' }])
          .setTextSelection(start + 2)
          .run();
      },
    };
  },
});

/** 방점: dots above the letters. */
export const Dot = Mark.create({
  name: 'dot',

  parseHTML() {
    return [{ tag: 'span.dot' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', mergeAttributes(HTMLAttributes, { class: 'dot' }), 0];
  },

  addCommands() {
    return {
      toggleDot:
        () =>
        ({ commands }) =>
          commands.toggleMark(this.name),
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-Shift-d': () => this.editor.commands.toggleDot(),
    };
  },
});

export function manuscriptExtensions(sceneSymbol: string): Extensions {
  return [
    StarterKit.configure({
      heading: false,
      blockquote: false,
      bulletList: false,
      orderedList: false,
      listItem: false,
      listKeymap: false,
      codeBlock: false,
      code: false,
      horizontalRule: false,
      link: false,
    }),
    SceneBreak.configure({ symbol: sceneSymbol }),
    Dot,
    Placeholder.configure({
      placeholder: ({ editor }) => (editor.isEmpty ? '여기에 쓰기 시작하세요' : ''),
    }),
  ];
}
