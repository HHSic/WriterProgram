// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { WebPane } from './WebPane';
const mock=vi.hoisted(()=>({ menu:null as null|{left:number;top:number;right:number;bottom:number}, state:{overview:{project:{id:'test'}},dialog:null as null|object,busy:null}, api:{isDesktop:true,onBrowserPage:vi.fn(()=>()=>{}),browserOpen:vi.fn(async()=>{}),browserBounds:vi.fn(async()=>{}),browserClose:vi.fn(async()=>{}),browserPreview:vi.fn(async()=> 'data:image/png;base64,dGVzdA==')} }));
vi.mock('../api',()=>({api:mock.api}));
vi.mock('../components/Menu',()=>({useMenuBounds:()=>mock.menu}));
vi.mock('../store',()=>({useApp:(selector:(s:typeof mock.state)=>unknown)=>selector(mock.state),webPages:()=>({}),rememberWebPage:vi.fn(),clipPage:vi.fn()}));
let root:Root,host:HTMLDivElement;
beforeEach(()=>{
  vi.clearAllMocks(); mock.menu=null; mock.state.dialog=null;
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  globalThis.ResizeObserver=class {observe(){} disconnect(){} unobserve(){}};
  vi.spyOn(HTMLElement.prototype,'getBoundingClientRect').mockReturnValue({left:500,top:100,right:1000,bottom:600,width:500,height:500,x:500,y:100,toJSON(){return{};}});
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
});
afterEach(async()=>{await act(async()=>root.unmount());host.remove();vi.restoreAllMocks();});
async function render(active=true){await act(async()=>{root.render(<WebPane id="test" active={active}/>);await Promise.resolve();});}
it('keeps the native web page visible for menus outside its rectangle',async()=>{
  await render(); expect(mock.api.browserOpen).toHaveBeenCalledTimes(1);
  mock.menu={left:100,top:100,right:350,bottom:550};await render();
  expect(mock.api.browserBounds).toHaveBeenLastCalledWith('web-test',expect.anything(),true);
  expect(mock.api.browserClose).not.toHaveBeenCalled();
});
it('keeps the captured page behind overlapping menus and dialogs, then resumes the same webview',async()=>{
  await render(); mock.menu={left:750,top:50,right:1050,bottom:200};await render();
  expect(mock.api.browserBounds).toHaveBeenLastCalledWith('web-test',expect.anything(),false);
  expect(host.querySelector('img')?.getAttribute('src')).toBe('data:image/png;base64,dGVzdA==');
  mock.menu=null;mock.state.dialog={kind:'view'};await render();
  expect(mock.api.browserBounds).toHaveBeenLastCalledWith('web-test',expect.anything(),false);
  mock.state.dialog=null;await render();
  expect(mock.api.browserBounds).toHaveBeenLastCalledWith('web-test',expect.anything(),true);
  expect(mock.api.browserOpen).toHaveBeenCalledTimes(1);
});
