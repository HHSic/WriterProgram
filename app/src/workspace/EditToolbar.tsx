// 편집 도구줄: everything the keyboard shortcuts and the floating bar do, as
// buttons under the page. Shown on touch screens (보기 설정 can show it
// always or never); on a phone it rides above the on-screen keyboard.

import { useEditorState } from '@tiptap/react';
import type { Editor } from '@tiptap/core';
import type { ReactNode } from 'react';
import { Icon } from '../components/Icon';
import { openMenu, type MenuItem } from '../components/Menu';
import { firstLineAt, marginsAt } from '../editor/paragraph';
import { addTextNote, openFind } from '../store';
import { openSymbols } from './Symbols';

/** 문단 여백 for the paragraphs the cursor or selection is in. */
export function marginMenu(editor: Editor): MenuItem[] {
  const now = marginsAt(editor.state);
  const first = firstLineAt(editor.state);
  const run = (f: (chain: ReturnType<Editor['chain']>) => ReturnType<Editor['chain']>) => () => {
    f(editor.chain().focus()).run();
  };
  return [
    { heading: now.left || now.right ? `지금 문단: 왼쪽 ${now.left}자 · 오른쪽 ${now.right}자` : '지금 문단: 여백 없음' },
    { label: '왼쪽 한 자 들이기 (Ctrl+])', onSelect: run((c) => c.shiftMargins(1)) },
    { label: '왼쪽 한 자 내기 (Ctrl+[)', disabled: now.left === 0, onSelect: run((c) => c.shiftMargins(-1)) },
    { label: '양쪽 한 자씩 들이기', onSelect: run((c) => c.shiftMargins(1, ['left', 'right'])) },
    {
      label: '양쪽 한 자씩 내기',
      disabled: now.left === 0 && now.right === 0,
      onSelect: run((c) => c.shiftMargins(-1, ['left', 'right'])),
    },
    { heading: first === null ? '첫 줄: 서식대로' : first >= 0 ? `첫 줄: 들여쓰기 ${first}자` : `첫 줄: 내어쓰기 ${-first}자` },
    { label: '첫 줄 서식대로', checked: first === null, onSelect: run((c) => c.setFirstLine(null)) },
    { label: '첫 줄 들여쓰기 1자', checked: first === 1, onSelect: run((c) => c.setFirstLine(1)) },
    { label: '첫 줄 들여쓰기 2자', checked: first === 2, onSelect: run((c) => c.setFirstLine(2)) },
    { label: '내어쓰기 1자', checked: first === -1, onSelect: run((c) => c.setFirstLine(-1)) },
    { label: '내어쓰기 2자', checked: first === -2, onSelect: run((c) => c.setFirstLine(-2)) },
    { label: '첫 줄 들이지 않기', checked: first === 0, onSelect: run((c) => c.setFirstLine(0)) },
    { separator: true },
    {
      label: '여백 없애기',
      disabled: now.left === 0 && now.right === 0,
      onSelect: run((c) => c.setMargins({ left: 0, right: 0 })),
    },
  ];
}

export function EditToolbar({ editor }: { editor: Editor }) {
  const state = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      editable: e.isEditable,
      bold: e.isActive('bold'),
      italic: e.isActive('italic'),
      underline: e.isActive('underline'),
      strike: e.isActive('strike'),
      dot: e.isActive('dot'),
      undo: e.can().undo(),
      redo: e.can().redo(),
    }),
  });

  const tool = (
    key: string,
    label: string,
    glyph: ReactNode,
    run: () => void,
    opts: { on?: boolean; disabled?: boolean } = {},
  ) => (
    <button
      key={key}
      type="button"
      className={`tool${opts.on ? ' on' : ''}`}
      aria-label={label}
      aria-pressed={opts.on}
      title={label}
      disabled={!state.editable || opts.disabled}
      // Keep the cursor (and the phone's keyboard) in the text.
      onMouseDown={(e) => e.preventDefault()}
      onClick={run}
    >
      {glyph}
    </button>
  );

  const chain = () => editor.chain().focus();

  return (
    <div className="edit-toolbar" role="toolbar" aria-label="편집 도구줄">
      {tool('undo', '되돌리기 (Ctrl+Z)', <Icon name="undo" size={17} />, () => chain().undo().run(), { disabled: !state.undo })}
      {tool('redo', '다시 하기 (Ctrl+Y)', <Icon name="redo" size={17} />, () => chain().redo().run(), { disabled: !state.redo })}
      <span className="tool-sep" aria-hidden="true" />
      {tool('bold', '굵게', <b>가</b>, () => chain().toggleBold().run(), { on: state.bold })}
      {tool('dot', '방점', <span className="dot">가</span>, () => chain().toggleDot().run(), { on: state.dot })}
      {tool('italic', '기울임', <i>가</i>, () => chain().toggleItalic().run(), { on: state.italic })}
      {tool('underline', '밑줄', <u>가</u>, () => chain().toggleUnderline().run(), { on: state.underline })}
      {tool('strike', '취소선', <s>가</s>, () => chain().toggleStrike().run(), { on: state.strike })}
      <span className="tool-sep" aria-hidden="true" />
      {tool('memo', '고른 글에 메모 달기', <Icon name="note" size={17} />, () => void addTextNote())}
      <button
        type="button"
        className="tool"
        aria-label="문단 여백"
        title="문단 여백"
        disabled={!state.editable}
        onMouseDown={(e) => e.preventDefault()}
        onClick={(e) => openMenu(e, marginMenu(editor), { title: '문단 여백' })}
      >
        <Icon name="indent" size={17} />
      </button>
      {tool('symbol', '문자표', <Icon name="symbol" size={17} />, () => openSymbols())}
      {tool('scene', '장면 나눔 넣기', <Icon name="diamond" size={16} />, () => chain().insertSceneBreak().run())}
      <span className="tool-sep" aria-hidden="true" />
      <button
        type="button"
        className="tool"
        aria-label="찾기"
        title="찾기"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => openFind({ scope: 'doc', focus: 'find' })}
      >
        <Icon name="search" size={17} />
      </button>
    </div>
  );
}
