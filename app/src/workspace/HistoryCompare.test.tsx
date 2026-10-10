// @vitest-environment jsdom
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import type {Overview,SnapshotInfo} from '../api/types';
import {HistoryCompareDialog} from './Compare';

const mock=vi.hoisted(()=>({
  state:{overview:null as Overview|null,conflicts:{} as Record<string,unknown>},
  api:{snapshotList:vi.fn(),snapshotLoad:vi.fn(),docLoad:vi.fn(),snapshotRestore:vi.fn()},
  peer:vi.fn(),restore:vi.fn(),close:vi.fn(),
}));
vi.mock('../api',()=>({api:mock.api}));
vi.mock('../editor/shared',()=>({peerOf:mock.peer}));
vi.mock('../store',()=>({useApp:(select:(s:typeof mock.state)=>unknown)=>select(mock.state),findDoc:()=>({doc:mock.state.overview!.parts[0].docs[0]}),closeDialog:mock.close,restoreRecord:mock.restore}));
const body=(...lines:string[])=>({type:'doc',content:lines.map(text=>({type:'paragraph',content:[{type:'text',text}]}))});
const before={id:'before',at:'2026-10-10T05:00:00Z',name:'초고'} as SnapshotInfo;
const after={id:'after',at:'2026-10-10T06:00:00Z',name:'수정본'} as SnapshotInfo;
let root:Root;
beforeEach(()=>{
  vi.clearAllMocks();Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  mock.state.overview={root:'/test/project',project:{sceneBreak:'***'},parts:[{docs:[{id:'chapter',title:'첫 장',locked:false}]}]} as unknown as Overview;
  mock.state.conflicts={};
  mock.api.snapshotList.mockResolvedValue([after,before]);
  mock.api.snapshotLoad.mockImplementation(async(_root,_doc,id)=>({body:id==='before'?body('초고의 문장','지운 문단'):body('수정한 문장','새로운 문단')}));
  mock.api.docLoad.mockResolvedValue({body:body('디스크에 저장된 글')});
  mock.peer.mockReturnValue({getJSON:()=>body('아직 저장 전인 문장','새로운 문단')});
  const host=document.createElement('div');document.body.append(host);root=createRoot(host);
});
afterEach(()=>{act(()=>root.unmount());document.body.replaceChildren();});
async function show(){await act(async()=>{root.render(<HistoryCompareDialog docId="chapter" recordId="before"/>);});}
async function choose(label:string,value:string){await act(async()=>{const select=document.querySelector<HTMLSelectElement>(`[aria-label="${label}"]`)!;select.value=value;select.dispatchEvent(new Event('change',{bubbles:true}));});}
function restoreButton(){return [...document.querySelectorAll('button')].find(b=>b.textContent==='선택한 이력으로 복원')!;}

it('compares the unsaved editor text without saving or restoring it',async()=>{
  await show();
  expect(document.querySelector('[role="table"]')?.textContent).toContain('아직 저장 전인 문장');
  expect(document.body.textContent).not.toContain('디스크에 저장된 글');
  expect(document.querySelectorAll('mark').length).toBeGreaterThan(0);
  expect(mock.api.docLoad).not.toHaveBeenCalled();expect(mock.api.snapshotRestore).not.toHaveBeenCalled();
});
it('compares two historical texts and restores only the selected right record',async()=>{
  await show();await choose('기준 원고','after');
  expect(mock.api.snapshotLoad).toHaveBeenCalledWith('/test/project','chapter','after');
  expect(document.querySelector('[role="table"]')?.textContent).toContain('수정한 문장');
  await choose('비교할 이력','after');
  expect(document.querySelector('.compare-summary')?.textContent).toContain('두 글이 같습니다.');
  await act(async()=>restoreButton().click());
  expect(mock.restore).toHaveBeenCalledWith('chapter',after);
});
it('falls back to the saved document when no editor is open',async()=>{
  mock.peer.mockReturnValue(null);await show();
  expect(mock.api.docLoad).toHaveBeenCalledWith('/test/project','chapter');
  expect(document.querySelector('[role="table"]')?.textContent).toContain('디스크에 저장된 글');
});
it('shows load failures without offering a stale restore',async()=>{
  await show();mock.api.snapshotLoad.mockRejectedValueOnce(Error('이력 파일을 읽을 수 없음'));
  await choose('비교할 이력','after');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('이력 파일을 읽을 수 없음');
  expect(document.querySelector('[role="table"]')).toBeNull();expect(restoreButton().disabled).toBe(true);
});
it('ignores a delayed comparison after the writer chooses another record',async()=>{
  let finish!:(value:unknown)=>void;
  mock.api.snapshotLoad.mockImplementationOnce(()=>new Promise(r=>{finish=r;}));
  await show();await choose('비교할 이력','after');
  await act(async()=>finish({body:body('지난 요청의 본문')}));
  expect(document.querySelector('[role="table"]')?.textContent).toContain('수정한 문장');
  expect(document.body.textContent).not.toContain('지난 요청의 본문');
});
it('allows comparison of locked documents and disables restoration',async()=>{
  mock.state.overview!.parts[0].docs[0].locked=true;await show();
  expect(document.querySelector('[role="table"]')).not.toBeNull();expect(restoreButton().disabled).toBe(true);
});
