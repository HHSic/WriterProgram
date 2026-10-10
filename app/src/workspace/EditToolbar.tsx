// 편집 도구줄: everything the keyboard shortcuts and the floating bar do, as
// buttons under the page. Shown on touch screens (보기 설정 can show it
// always or never); on a phone it rides above the on-screen keyboard.

import { useEditorState } from '@tiptap/react';
import type { Editor } from '@tiptap/core';
import type { ReactNode } from 'react';
import { Icon } from '../components/Icon';
import { openMenu, type MenuItem } from '../components/Menu';
import { MARK_BUTTONS, activeMarks, toggleMark, type MarkKey } from '../editor/markButtons';
import { TEXT_COLORS, type ParagraphAlign } from '../editor/formatting';
import { firstLineAt, marginsAt } from '../editor/paragraph';
import { keysText } from '../lib/shortcuts';
import { addTextNote, openFind, toggleReading, useApp } from '../store';
import { openSymbols } from './Symbols';

/** The marks in the 편집 도구줄, in its order (방점 next to 굵게). */
const TOOLBAR_MARKS: MarkKey[] = ['bold', 'dot', 'italic', 'underline', 'strike'];

/** 문단 여백 for the paragraphs the cursor or selection is in. */
export function marginMenu(editor: Editor): MenuItem[] {
  const now = marginsAt(editor.state);
  const first = firstLineAt(editor.state);
  const run = (f: (chain: ReturnType<Editor['chain']>) => ReturnType<Editor['chain']>) => () => {
    f(editor.chain().focus()).run();
  };
  return [
    { heading: now.left || now.right ? `지금 문단: 왼쪽 ${now.left}자 · 오른쪽 ${now.right}자` : '지금 문단: 여백 없음' },
    { label: `왼쪽 한 자 들이기 (${keysText('marginIn')})`, onSelect: run((c) => c.shiftMargins(1)) },
    { label: `왼쪽 한 자 내기 (${keysText('marginOut')})`, disabled: now.left === 0, onSelect: run((c) => c.shiftMargins(-1)) },
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

export function formatMenu(editor: Editor): MenuItem[] {
  const chain = () => editor.chain().focus();
  return [
    { heading: '문단 스타일' },
    ...([null, 1, 2, 3] as const).map((heading, i) => ({ label: ['일반 본문', '제목', '부제목', '소제목'][i], checked: editor.isActive('paragraph', { heading }), onSelect: () => { chain().setParagraphHeading(heading).run(); } })),
    { heading: '문단 정렬' },
    ...([['left', '왼쪽'], ['center', '가운데'], ['right', '오른쪽'], ['justify', '양쪽']] as [ParagraphAlign, string][]).map(([align, label]) => ({
      label: `${label} 정렬`, checked: editor.isActive('paragraph', { align }),
      onSelect: () => { chain().setParagraphAlign(align).run(); },
    })),
    { heading: '글자색' },
    { label: '기본 글자색', onSelect: () => { chain().unsetTextColor().run(); } },
    ...TEXT_COLORS.map((color, i) => ({ label: ['빨강', '주황', '초록', '파랑', '보라', '갈색', '회색'][i],
      checked: editor.isActive('textColor', { color }), onSelect: () => { chain().setTextColor(color).run(); } })),
    { separator: true },
    { label: `쪽 나눔 넣기 (${keysText('pageBreak')})`, onSelect: () => { chain().insertPageBreak().run(); } },
    { label: '이 문단 앞 쪽 나눔 없애기', disabled: !editor.isActive('paragraph', { pageBreakBefore: true }), onSelect: () => { chain().removePageBreak().run(); } },
  ];
}

export function EditToolbar({ editor }: { editor: Editor }) {
  const state = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      editable: e.isEditable,
      ...activeMarks(e),
      centered: e.isActive('paragraph', { align: 'center' }),
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
  const reading = useApp((s) => s.reading?.editor === editor && s.reading.status !== 'noVoice');

  return (
    <div className="edit-toolbar" role="toolbar" aria-label="편집 도구줄">
      {tool('undo', `되돌리기 (${keysText('undo')})`, <Icon name="undo" size={17} />, () => chain().undo().run(), { disabled: !state.undo })}
      {tool('redo', `다시 하기 (${keysText('redo')})`, <Icon name="redo" size={17} />, () => chain().redo().run(), { disabled: !state.redo })}
      <span className="tool-sep" aria-hidden="true" />
      {TOOLBAR_MARKS.map((key) => {
        const b = MARK_BUTTONS[key];
        return tool(key, b.name, b.glyph, () => toggleMark(editor, key), { on: state[key] });
      })}
      <span className="tool-sep" aria-hidden="true" />
      {tool('center', '가운데 정렬 (Ctrl+E)', <span>≡</span>, () => chain().setParagraphAlign(state.centered ? null : 'center').run(), { on: state.centered })}
      <label className="tool color-tool" title="글자색 고르기">
        <span>가</span><input type="color" aria-label="글자색 고르기" disabled={!state.editable}
          value={editor.getAttributes('textColor').color ?? '#C0392B'} onChange={(e) => chain().setTextColor(e.target.value).run()} />
      </label>
      {tool('colorReset', '기본 글자색으로', <span>가↺</span>, () => chain().unsetTextColor().run())}
      {tool('pageBreak', '쪽 나눔 넣기 (Ctrl+Enter)', <span>쪽↵</span>, () => chain().insertPageBreak().run())}
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
      <button
        type="button"
        className={`tool${reading ? ' on' : ''}`}
        aria-label={reading ? '소리 내어 읽기 멈춤' : '소리 내어 읽기'}
        aria-pressed={reading}
        title={reading ? '소리 내어 읽기 멈춤' : '소리 내어 읽기'}
        onMouseDown={(e) => e.preventDefault()}
        onClick={toggleReading}
      >
        <Icon name="speaker" size={17} />
      </button>
    </div>
  );
}
