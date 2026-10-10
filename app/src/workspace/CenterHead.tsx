import { Icon } from '../components/Icon';
import { openMenu, type MenuItem } from '../components/Menu';
import { UNTITLED, docNumber } from '../lib/labels';
import { keysText } from '../lib/shortcuts';
import { findDoc, openDialog, openWeb, saveNow, saveProjectFileAs, setFocusMode, splitView, setView, toggleReading, unsplit, useApp, webPages } from '../store';
import { SaveStatus } from './SaveStatus';
import { api } from '../api';

/** Document location and save status; commands live below the ribbon tabs. */
export function CenterHead() {
  const ov = useApp(s=>s.overview)!;
  const activeDocId=useApp(s=>s.activeDocId);
  const activeCardId=useApp(s=>s.activeCardId);
  const editor=useApp(s=>s.editor);
  const conflict=useApp(s=>s.activeDocId?s.conflicts[s.activeDocId]:undefined);
  const rightOpen=useApp(s=>s.rightOpen);
  const target=useApp(s=>s.activeTarget);
  const reviewFile=useApp(s=>s.activeTarget?.kind==='review'?s.reviewFiles[s.activeTarget.id]:undefined);
  const split=useApp(s=>s.panes.length>1?s.split:null);
  const pages=useApp(s=>s.view.pages);
  const reading=useApp(s=>s.reading!==null&&s.reading.status!=='noVoice'&&s.reading.editor===s.editor);
  const card=activeCardId?ov.cards.find(c=>c.id===activeCardId):undefined;
  const place=activeDocId?findDoc(ov,activeDocId):null;
  const fileItems=():MenuItem[]=>[
    {label:'작품 파일로 저장 (.wpm)…',disabled:!api.isDesktop,onSelect:()=>void saveProjectFileAs()},
    {label:'지금 저장',onSelect:()=>void saveNow()},
    ...(ov.projectFile?[{label:'작품 파일 위치 열기',onSelect:()=>void api.reveal(ov.projectFile!)}]:[]),
  ];
  const moreItems=():MenuItem[]=>[
    ...fileItems(),
    {label:'원고 가져오기…',onSelect:()=>openDialog({kind:'import'})},
    {label:'내보내기…',onSelect:()=>openDialog({kind:'export'})},
    {separator:true},
    {label:'책 미리보기',onSelect:()=>openDialog({kind:'reader'})},
    {label:'링크로 공유',onSelect:()=>openDialog({kind:'share'})},
    {label:'쪽 보기',checked:pages,disabled:!editor,onSelect:()=>setView({pages:!pages})},
    {label:reading?'소리 내어 읽기 멈춤':'소리 내어 읽기',disabled:!editor,hint:keysText('readAloud'),onSelect:toggleReading},
    {separator:true},
    {label:'좌우로 나눠 보기',checked:split==='row',onSelect:()=>splitView('row')},
    {label:'위아래로 나눠 보기',checked:split==='column',onSelect:()=>splitView('column')},
    {label:'나눠 보기 닫기',disabled:split===null,onSelect:()=>void unsplit()},
    {label:'웹 보기',onSelect:()=>void openWeb()},
    {label:'집중 모드',hint:keysText('focusMode'),onSelect:()=>setFocusMode(true)},
    {label:'보기 설정',onSelect:()=>openDialog({kind:'view'})},
    {label:'작품 설정',onSelect:()=>openDialog({kind:'project'})},
  ];
  return <div className="col-head center-head">
    <nav className="crumbs" aria-label="현재 위치">
      {target?.kind==='notes'&&<strong>메모함</strong>}
      {target?.kind==='review'&&<><span>교정본 검토</span><Icon name="chevronRight" size={12}/><strong>{reviewFile||'받은 교정본'}</strong></>}
      {target?.kind==='web'&&<><span>웹</span><Icon name="chevronRight" size={12}/><strong>{webPages(ov.project.id)[target.id]?.title||'새 페이지'}</strong></>}
      {target?.kind==='table'&&<><span>개요 표</span><Icon name="chevronRight" size={12}/><strong>{ov.parts.find(p=>p.id===target.id)?.title}</strong></>}
      {activeCardId&&<><span>설정집</span><Icon name="chevronRight" size={12}/><span>{ov.cardTypes.find(t=>t.id===card?.cardType)?.name}</span><Icon name="chevronRight" size={12}/><strong>{card?.name||'이름 없음'}</strong></>}
      {place&&<><span>{place.part?place.part.title:'기획'}</span><Icon name="chevronRight" size={12}/><strong>{place.number!==null&&`${docNumber(ov.project.kind,place.number)} · `}{place.doc.title||UNTITLED}</strong></>}
    </nav>
    <div className="grow"/>
    <div className="center-actions">
      {conflict?<span className="save-state conflict" role="status" title="다른 기기에서 고친 내용과 달라 저장을 멈춤"><span className="dot-mark"/>다른 기기에서 바뀜</span>:<SaveStatus/>}
      <button type="button" className={`icon-btn${rightOpen?' on':''}`} aria-label={rightOpen?'오른쪽 패널 닫기':'오른쪽 패널 열기'} aria-pressed={rightOpen} title="오른쪽 패널" onClick={()=>useApp.setState({rightOpen:!rightOpen})}><Icon name="panel" size={18}/></button>
      <button type="button" className="icon-btn" aria-label="작업 메뉴" title="저장·읽기·창·설정" onClick={e=>openMenu(e,moreItems(),{title:'작업 메뉴'})}><Icon name="more"/></button>
    </div>
  </div>;
}
