import { useState } from 'react';
import { useEditorState } from '@tiptap/react';
import type { Editor } from '@tiptap/core';
import { openMenu } from '../components/Menu';
import { FONT_LABEL, type BodyFont } from '../lib/view';
import { MARK_BUTTONS, activeMarks, toggleMark, type MarkKey } from '../editor/markButtons';
import { addTextNote, openDialog, openFind, saveNow, setView, splitView, unsplit, toggleFocusMode, updateProject, useApp } from '../store';
import { formatMenu, marginMenu } from './EditToolbar';
import { synopsisCommand } from './BlockCommands';
import { openSymbols } from './Symbols';

type Tab = '편집' | '서식' | '보기' | '쪽' | '검토';
const TABS: Tab[] = ['편집','서식','보기','쪽','검토'];
export function Ribbon() {
  const editor = useApp(s => s.editor);
  const view = useApp(s => s.view);
  const [tab,setTab] = useState<Tab>('편집');
  const [collapsed,setCollapsed] = useState(false);
  return <section className={`ribbon${collapsed ? ' collapsed' : ''}`} aria-label="문서 도구">
    <div className="ribbon-tabs" role="tablist" aria-label="도구 종류">{TABS.map(t => <button key={t} role="tab" id={`ribbon-${t}`} aria-controls="ribbon-tools" aria-selected={t===tab} onClick={() => { setTab(t); setCollapsed(false); }}>{t}</button>)}<span className="grow"/><button className="ribbon-collapse" aria-label={collapsed ? '도구 펼치기' : '도구 접기'} aria-expanded={!collapsed} onClick={() => setCollapsed(!collapsed)}>{collapsed ? '⌄' : '⌃'}</button></div>
    {!collapsed && <div id="ribbon-tools" role="tabpanel" aria-labelledby={`ribbon-${tab}`} className="ribbon-tools">
      {editor ? <EditorTools editor={editor} tab={tab}/> : <span className="hint">문서를 열면 편집 도구를 사용할 수 있습니다.</span>}
      {tab==='보기' && <><Group label="작성 화면">
        <Button label="쪽 보기" glyph="▤" active={view.pages} run={() => setView({pages:!view.pages})}/>
        <Button label="눈금자" glyph="┠" active={view.ruler} run={() => setView({ruler:!view.ruler})}/>
        <Button label="문단 부호" glyph="¶" active={view.showMarks} run={() => setView({showMarks:!view.showMarks})}/>
        <Button label="보기 설정" glyph="⚙" run={() => openDialog({kind:'view'})}/>
      </Group><Group label="창·읽기">
        <Button label="나눠 보기" glyph="◫" run={() => { if(useApp.getState().panes.length>1) void unsplit(); else splitView('row'); }}/>
        <Button label="집중 모드" glyph="⛶" run={toggleFocusMode}/>
        <Button label="책 미리보기" glyph="▣" run={() => openDialog({kind:'reader'})}/>
      </Group></>}
      {(tab==='서식'||tab==='보기') && <Group label="화면 글꼴·크기">
        <select aria-label="화면 글꼴" value={view.font} onChange={e => setView({font:e.target.value as BodyFont})}>{Object.entries(FONT_LABEL).map(([id,name]) => <option key={id} value={id}>{name}</option>)}</select>
        <label>크기 <input type="number" min={1} step={1} aria-label="화면 글자 크기" value={view.fontSize} onChange={e => { const n=e.target.valueAsNumber; if(Number.isFinite(n)&&n>=1) setView({fontSize:n}); }}/><span>px</span></label>
        <label>자간 <input type="number" aria-label="화면 자간" value={view.letterSpacing} onChange={e => { const n=e.target.valueAsNumber; if(Number.isFinite(n)) setView({letterSpacing:n}); }}/>%</label>
      </Group>}
      {tab==='쪽' && <Group label="용지·여백">
        <Button label="편집 용지" glyph="▯" run={() => openDialog({kind:'project',tab:'format'})}/>
        <Button label="세로" glyph="▯" run={() => { const f=useApp.getState().overview!.project.manuscriptFormat; void updateProject({manuscriptFormat:{...f,paper:{...f.paper,widthMm:Math.min(f.paper.widthMm,f.paper.heightMm),heightMm:Math.max(f.paper.widthMm,f.paper.heightMm)}}}); }}/>
        <Button label="가로" glyph="▭" run={() => { const f=useApp.getState().overview!.project.manuscriptFormat; void updateProject({manuscriptFormat:{...f,paper:{...f.paper,widthMm:Math.max(f.paper.widthMm,f.paper.heightMm),heightMm:Math.min(f.paper.widthMm,f.paper.heightMm)}}}); }}/>
        <Button label="쪽 여백" glyph="▧" run={() => openDialog({kind:'project',tab:'format'})}/>
        <Button label="머리말·꼬리말" glyph="☰" run={() => openDialog({kind:'project',tab:'format'})}/>
      </Group>}
      {tab==='검토' && <><Group label="문서 이력">
        <Button label="문서 이력 관리" glyph="◷" run={() => useApp.setState({rightOpen:true,rightTab:'records',previewCardId:null})}/>
        <Button label="교정본 주고받기" glyph="⇄" run={() => openDialog({kind:'exchanges'})}/>
        <Button label="창작 기록" glyph="▥" run={() => openDialog({kind:'proof'})}/>
      </Group><Group label="원고 분석"><Button label="인물·장소 찾기" glyph="⌕" run={() => openDialog({kind:'discover'})}/></Group></>}
    </div>}
  </section>;
}
function Group({label,children}:{label:string;children:React.ReactNode}) { return <div className="ribbon-group"><div className="ribbon-group-tools">{children}</div><span className="ribbon-group-label">{label}</span></div>; }
function Button({label,glyph,run,disabled=false,active}:{label:string;glyph:React.ReactNode;run:()=>void;disabled?:boolean;active?:boolean}) { return <button className="ribbon-button" aria-label={label} aria-pressed={active} disabled={disabled} onMouseDown={e=>e.preventDefault()} onClick={run}><span className="ribbon-glyph" aria-hidden="true">{glyph}</span><span>{label}</span></button>; }
function EditorTools({editor,tab}:{editor:Editor;tab:Tab}) {
  const state = useEditorState({editor,selector:({editor:e}) => ({editable:e.isEditable,undo:e.can().undo(),redo:e.can().redo(),heading:e.getAttributes('paragraph').heading,marks:activeMarks(e),color:e.getAttributes('textColor').color,align:e.getAttributes('paragraph').align,pageBreak:e.getAttributes('paragraph').pageBreakBefore})});
  const chain=()=>editor.chain().focus();
  return <>
    {tab==='편집' && <><Group label="저장·편집">
      <Button label="저장" glyph="▣" run={()=>void saveNow()}/>
      <Button label="되돌리기" glyph="↶" disabled={!state.editable||!state.undo} run={()=>{chain().undo().run();}}/>
      <Button label="다시 하기" glyph="↷" disabled={!state.editable||!state.redo} run={()=>{chain().redo().run();}}/>
      <Button label="찾기·바꾸기" glyph="⌕" run={()=>openFind({scope:'doc',focus:'find'})}/>
    </Group><Group label="입력">
      <Button label="문자표" glyph="Ω" disabled={!state.editable} run={openSymbols}/>
      <Button label="장면 나눔" glyph="◇" disabled={!state.editable} run={()=>{chain().insertSceneBreak().run();}}/>
      <Button label="시놉시스 추가" glyph="≡" disabled={!state.editable} run={()=>synopsisCommand(editor,'add')}/>
    </Group></>}
    {(tab==='서식'||tab==='편집') && <><Group label="문단 스타일"><select aria-label="문단 스타일" disabled={!state.editable} value={state.heading??0} onChange={e=>{chain().setParagraphHeading(Number(e.target.value)||null).run();}}><option value="0">일반 본문</option><option value="1">제목</option><option value="2">부제목</option><option value="3">소제목</option></select></Group>
    <Group label="글자 꾸밈">{(['bold','italic','underline','strike','dot'] as MarkKey[]).map(k => <Button key={k} label={MARK_BUTTONS[k].name} glyph={MARK_BUTTONS[k].glyph} active={editor.isActive(k)} disabled={!state.editable} run={()=>toggleMark(editor,k)}/>)}
      <label className="ribbon-color">글자색<input type="color" aria-label="선택 글자색" disabled={!state.editable} value={editor.getAttributes('textColor').color??'#000000'} onChange={e=>{chain().setTextColor(e.target.value).run();}}/></label>
      <Button label="색 초기화" glyph="가↺" disabled={!state.editable} run={()=>{chain().unsetTextColor().run();}}/>
    </Group><Group label="문단 모양">
      {(['left','center','right','justify'] as const).map((a,i)=><Button key={a} label={['왼쪽 정렬','가운데 정렬','오른쪽 정렬','양쪽 정렬'][i]} glyph={['≡','☰','≡','▤'][i]} active={editor.isActive('paragraph',{align:a})} disabled={!state.editable} run={()=>{chain().setParagraphAlign(a).run();}}/>)}
      <button className="ribbon-button" disabled={!state.editable} onMouseDown={e=>e.preventDefault()} onClick={e=>openMenu(e,marginMenu(editor),{title:'들여쓰기·문단 여백'})}>들여쓰기·여백</button>
      <button className="ribbon-button" disabled={!state.editable} onMouseDown={e=>e.preventDefault()} onClick={e=>openMenu(e,formatMenu(editor),{title:'서식'})}>서식 더보기</button>
    </Group></>}
    {tab==='쪽' && <Group label="쪽 나누기"><Button label="쪽 나눔" glyph="쪽↵" disabled={!state.editable} run={()=>{chain().insertPageBreak().run();}}/><Button label="쪽 나눔 지우기" glyph="쪽×" disabled={!state.editable||!editor.isActive('paragraph',{pageBreakBefore:true})} run={()=>{chain().removePageBreak().run();}}/></Group>}
    {tab==='검토' && <Group label="메모"><Button label="새 메모" glyph="▱" disabled={!state.editable} run={()=>void addTextNote()}/><Button label="메모 보기" glyph="☷" run={()=>useApp.setState({rightOpen:true,rightTab:'notes',previewCardId:null})}/></Group>}
  </>;
}
