// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { AccountDialog } from './AccountDialog';

const mock=vi.hoisted(()=>({
  state:{status:{url:'https://example.supabase.co' as string|null,account:null as null|{id:string,email:string,name:string}},working:false,syncState:'idle',error:'',notice:''},
  configure:vi.fn(),login:vi.fn(),logout:vi.fn(),sync:vi.fn(),cancel:vi.fn(),close:vi.fn(),
}));
vi.mock('../store/account',()=>({useAccount:()=>mock.state,configureAccount:mock.configure,loginAccount:mock.login,logoutAccount:mock.logout,syncAccountSettings:mock.sync}));
vi.mock('../api',()=>({api:{isDesktop:true,accountCancel:mock.cancel}}));
vi.mock('../store',()=>({closeDialog:mock.close}));
(globalThis as {IS_REACT_ACT_ENVIRONMENT?:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
let root:Root;
beforeEach(()=>{
  vi.clearAllMocks();Object.assign(mock.state,{status:{url:'https://example.supabase.co',account:null},working:false,syncState:'idle',error:'',notice:''});
  mock.login.mockResolvedValue(false);mock.configure.mockResolvedValue(true);
  const host=document.createElement('div');document.body.append(host);root=createRoot(host);
});
afterEach(()=>{act(()=>root.unmount());document.body.replaceChildren();});
async function show(){await act(async()=>root.render(<AccountDialog/>));}
async function click(text:string){const button=[...document.querySelectorAll('button')].find(b=>b.textContent===text)!;await act(async()=>button.click());}
async function input(label:string,value:string){await act(async()=>{
  const node=[...document.querySelectorAll('label')].find(l=>l.textContent?.includes(label))!.querySelector('input')!;
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(node,value);
  node.dispatchEvent(new Event('input',{bubbles:true}));
});}
it('offers registered OAuth and SSO login, and submits app credentials without saving them',async()=>{
  await show();await click('Google로 로그인');expect(mock.login).toHaveBeenLastCalledWith({kind:'oauth',provider:'google'});
  await click('Kakao로 로그인');expect(mock.login).toHaveBeenLastCalledWith({kind:'oauth',provider:'kakao'});
  await input('이메일','writer@example.com');await input('비밀번호','example-password');await click('로그인');
  expect(mock.login).toHaveBeenLastCalledWith({kind:'password',email:'writer@example.com',password:'example-password'});
  expect(document.querySelector<HTMLInputElement>('input[type=password]')?.value).toBe('');
  await input('회사·학교 SSO 도메인','school.example');await click('SSO로 로그인');
  expect(mock.login).toHaveBeenLastCalledWith({kind:'sso',domain:'school.example'});
});
it('saves public connection settings and shows signup confirmation as information',async()=>{
  await show();await click('서버 연결 설정');
  await input('Supabase 프로젝트 URL','https://new.supabase.co');await input('Publishable','sb_publishable_example');await click('서버 연결 저장');
  expect(mock.configure).toHaveBeenCalledWith('https://new.supabase.co','sb_publishable_example');
  await click('새 계정 만들기');await input('이메일','writer@example.com');await input('비밀번호','example-password');await click('가입');
  expect(mock.login).toHaveBeenLastCalledWith({kind:'signup',email:'writer@example.com',password:'example-password'});
  mock.state.notice='메일 인증 후 로그인할 수 있습니다.';await show();
  expect(document.querySelector('[role=status]')?.textContent).toContain('메일 인증');expect(document.querySelector('[role=alert]')).toBeNull();
});
it('shows settings status for the current account and allows cancellation while signing in',async()=>{
  mock.state.status.account={id:'u1',email:'writer@example.com',name:'작가'};mock.state.syncState='saved';await show();
  expect(document.body.textContent).toContain('개인 설정 저장됨');await click('지금 동기화');expect(mock.sync).toHaveBeenCalledOnce();
  await click('로그아웃');expect(mock.logout).toHaveBeenCalledOnce();
  mock.state.status.account=null;mock.state.working=true;await show();await click('로그인 취소');expect(mock.cancel).toHaveBeenCalledOnce();
});
it('switches to login when startup loads a configured server after the dialog opens',async()=>{
  mock.state.status.url=null;mock.state.working=true;await show();
  expect(document.body.textContent).toContain('Supabase 프로젝트 URL');
  mock.state.status.url='https://configured.supabase.co';mock.state.working=false;await show();
  expect(document.body.textContent).toContain('Google로 로그인');expect(document.body.textContent).not.toContain('Supabase 프로젝트 URL');
});
