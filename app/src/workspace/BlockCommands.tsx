import { useEffect, useState } from 'react';
import type { Editor } from '@tiptap/core';
import { MARK_BUTTONS, toggleMark, type MarkKey } from '../editor/markButtons';
import { openMenu, type MenuItem } from '../components/Menu';
import { formatMenu, marginMenu } from './EditToolbar';
import { addTextNote, toastError } from '../store';

export function synopsisCommand(editor: Editor, action: 'add'|'remove') {
  editor.view.dom.dispatchEvent(new CustomEvent('writer:synopsis', { detail: action }));
}
export function selectionMenu(editor: Editor): MenuItem[] {
  const copy = async (cut: boolean) => {
    const { from, to } = editor.state.selection;
    const original = editor.state.doc;
    try {
      await navigator.clipboard.writeText(editor.state.doc.textBetween(from, to, '\n'));
      if (cut && !editor.isDestroyed && editor.isEditable && editor.state.doc.eq(original)) editor.chain().focus().deleteRange({ from, to }).run();
    } catch(e) { toastError('클립보드에 복사하지 못함', e); }
  };
  return [
    { label:'복사 (Ctrl+C)', onSelect: () => void copy(false) },
    { label:'잘라내기 (Ctrl+X)', disabled: !editor.isEditable, onSelect: () => void copy(true) },
    ...(['bold','italic','underline','strike','dot'] as MarkKey[]).map(key => ({ label: MARK_BUTTONS[key].name, checked: editor.isActive(key), onSelect: () => toggleMark(editor,key) })),
    ...formatMenu(editor), ...marginMenu(editor),
    { label: '선택한 글에 메모', onSelect: () => void addTextNote() },
  ];
}
export function openSelectionMenu(e: React.MouseEvent, editor: Editor) {
  if (!editor.isEditable || editor.state.selection.empty || !(e.target as HTMLElement).closest('.ProseMirror')) return;
  openMenu(e, selectionMenu(editor), { title: '선택 영역 서식' });
}
interface Command { label: string; keywords: string; run: (editor: Editor) => void }
export const BLOCK_COMMANDS: Command[] = [
  ...([1,2,3,null] as const).map((level,i) => ({ label: ['제목','부제목','소제목','일반 본문'][i], keywords: ['h1 title','h2 subtitle','h3 heading','text 본문'][i], run: (e:Editor) => { e.chain().focus().setParagraphHeading(level).run(); } })),
  { label:'시놉시스 추가', keywords:'synopsis 요약', run:e => synopsisCommand(e,'add') },
  { label:'시놉시스 삭제', keywords:'synopsis 제거', run:e => synopsisCommand(e,'remove') },
  { label:'장면 나눔', keywords:'scene', run:e => { e.chain().focus().insertSceneBreak().run(); } },
  { label:'쪽 나눔', keywords:'page', run:e => { e.chain().focus().insertPageBreak().run(); } },
  { label:'가운데 정렬', keywords:'center 중앙', run:e => { e.chain().focus().setParagraphAlign('center').run(); } },
];
export function slashQuery(editor: Editor) {
  const { $from, empty } = editor.state.selection;
  if (!empty || !editor.isEditable || $from.parent.type.name !== 'paragraph') return null;
  const text = $from.parent.textBetween(0, $from.parentOffset, '\n');
  if (!/^\/[^/\n]{0,30}$/u.test(text)) return null;
  return { from: $from.start(), to: $from.pos, query: text.slice(1) };
}
export function SlashCommands({ editor }: { editor: Editor }) {
  const [value, setValue] = useState<ReturnType<typeof slashQuery>>(null);
  const [index, setIndex] = useState(0);
  const [dismissed, setDismissed] = useState<number | null>(null);
  const [coords,setCoords] = useState({ left:0, top:0 });
  useEffect(() => {
    const update = () => {
      const next = editor.view.composing ? null : slashQuery(editor);
      setValue(next); setIndex(0);
      if (!next) setDismissed(null);
      else { const r = editor.view.coordsAtPos(next.to); setCoords({ left: Math.max(8,Math.min(r.left,window.innerWidth-280)), top:Math.max(8,Math.min(r.bottom+6,window.innerHeight-340)) }); }
    };
    editor.on('transaction', update); editor.view.dom.addEventListener('compositionend', update);
    const scroll = editor.view.dom.closest('.doc-scroll'); scroll?.addEventListener('scroll', update);
    return () => { editor.off('transaction',update); editor.view.dom.removeEventListener('compositionend',update); scroll?.removeEventListener('scroll',update); };
  },[editor]);
  const items = BLOCK_COMMANDS.filter(c => `${c.label} ${c.keywords}`.toLowerCase().includes(value?.query.toLowerCase() ?? ''));
  const visible = value && value.from !== dismissed;
  const apply = (command: Command) => {
    if (!value || editor.view.composing) return;
    editor.chain().focus().deleteRange({ from:value.from,to:value.to }).run();
    command.run(editor); setDismissed(value.from); setValue(null);
  };
  useEffect(() => {
    if (!visible) return;
    const key = (e:KeyboardEvent) => {
      if (e.isComposing || e.keyCode === 229 || editor.view.composing || !editor.isFocused) return;
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); setDismissed(value.from); }
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); e.stopPropagation(); setIndex(i => items.length ? (i + (e.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length : 0); }
      if ((e.key === 'Enter' || e.key === 'Tab') && items[index]) { e.preventDefault(); e.stopPropagation(); apply(items[index]); }
    };
    window.addEventListener('keydown',key,true);
    return () => window.removeEventListener('keydown',key,true);
  });
  if (!visible) return null;
  return <div className="slash-menu" role="listbox" aria-label="블록 명령어" style={{ left:coords.left,top:coords.top }}>
    {items.length ? items.map((c,i) => <button key={c.label} role="option" aria-selected={i===index} onMouseDown={e => e.preventDefault()} onClick={() => apply(c)}>{c.label}</button>) : <p>맞는 명령어가 없습니다.</p>}
  </div>;
}
