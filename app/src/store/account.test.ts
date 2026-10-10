// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { AccountStatus, PersonalSettings, SettingsSnapshot } from '../api/types';
import { DEFAULT_VIEW } from '../lib/view';
import { loadRibbon, mergeSettings } from '../lib/preferences';
import { loadBookPreferences } from '../lib/book';

const mock=vi.hoisted(()=>({api:{accountStatus:vi.fn(),accountConfigure:vi.fn(),accountLogin:vi.fn(),accountLogout:vi.fn(),accountSettingsRead:vi.fn(),accountSettingsWrite:vi.fn()}}));
vi.mock('../api',()=>mock);
const {installAccountSettings,loginAccount,logoutAccount,syncAccountSettings,useAccount}=await import('./account');
const {useApp}=await import('./state');
const A:AccountStatus={url:'https://test.supabase.co',account:{id:'a',email:'a@example.com',name:'A'}};
const B:AccountStatus={...A,account:{id:'b',email:'b@example.com',name:'B'}};
let stop:(()=>void)|undefined;
let server:SettingsSnapshot;
const settle=async()=>{for(let i=0;i<15;i++)await Promise.resolve();};
function deferred<T>() {let resolve!:(value:T)=>void;const promise=new Promise<T>(r=>{resolve=r;});return {promise,resolve};}
beforeEach(()=>{
  vi.useFakeTimers();vi.clearAllMocks();localStorage.clear();
  useApp.setState({view:{...DEFAULT_VIEW}});
  useAccount.setState({status:{url:null,account:null},working:false,syncState:'idle',error:'',notice:''});
  server={revision:1,settings:{view:{fontSize:27,theme:'dark'},ribbon:{tab:'보기',collapsed:false},book:{fontSize:1200,pageWidth:210,pageHeight:297}},updatedAt:null};
  mock.api.accountStatus.mockResolvedValue(A);
  mock.api.accountLogin.mockResolvedValue({account:B.account,confirmationRequired:false});
  mock.api.accountLogout.mockResolvedValue(undefined);
  mock.api.accountSettingsRead.mockImplementation(async()=>structuredClone(server));
  mock.api.accountSettingsWrite.mockImplementation(async(_key:string,_revision:number,patch:PersonalSettings)=>{
    server={...server,revision:server.revision+1,settings:mergeSettings(server.settings,patch)};
    return {conflict:false,snapshot:structuredClone(server)};
  });
});
afterEach(()=>{stop?.();stop=undefined;vi.useRealTimers();});
async function boot(){stop=installAccountSettings();await settle();}
it('automatically applies private settings, including ribbon and reader preferences',async()=>{
  await boot();
  expect(useApp.getState().view).toMatchObject({fontSize:27,theme:'dark'});
  expect(loadRibbon()).toEqual({tab:'보기',collapsed:false});
  expect(loadBookPreferences()).toMatchObject({fontSize:1200,pageWidth:210,pageHeight:297,device:'auto'});
  expect(mock.api.accountSettingsRead).toHaveBeenCalledWith(JSON.stringify([A.url,'a']));
  expect(mock.api.accountSettingsWrite).not.toHaveBeenCalled();
});
it('restores guest settings on logout and ignores a previous account response',async()=>{
  const old=deferred<SettingsSnapshot>();mock.api.accountSettingsRead.mockReturnValueOnce(old.promise);
  await boot();
  await logoutAccount();
  expect(useApp.getState().view.fontSize).toBe(DEFAULT_VIEW.fontSize);
  server={revision:1,settings:{view:{fontSize:42}},updatedAt:null};
  await loginAccount({kind:'password',email:'b@example.com',password:'fake-password'});await settle();
  old.resolve({revision:8,settings:{view:{fontSize:99}},updatedAt:null});await settle();
  expect(useApp.getState().view.fontSize).toBe(42);
  expect(useAccount.getState().status.account?.id).toBe('b');
});
it('retains offline changes across restart and uploads them after reading remote settings',async()=>{
  mock.api.accountSettingsRead.mockRejectedValue(new Error('offline'));await boot();
  useApp.setState({view:{...useApp.getState().view,fontSize:35}});
  await vi.advanceTimersByTimeAsync(1000);
  expect(useAccount.getState().syncState).toBe('error');
  stop?.();stop=undefined;
  mock.api.accountSettingsRead.mockImplementation(async()=>structuredClone(server));
  await boot();
  expect(useApp.getState().view.fontSize).toBe(35);
  expect(server.settings.view?.fontSize).toBe(35);
});
it('preserves edits made while a write is in flight',async()=>{
  await boot();const slow=deferred<{conflict:boolean;snapshot:SettingsSnapshot}>();
  mock.api.accountSettingsWrite.mockReturnValueOnce(slow.promise);
  useApp.setState({view:{...useApp.getState().view,fontSize:31}});
  await vi.advanceTimersByTimeAsync(1000);
  useApp.setState({view:{...useApp.getState().view,fontSize:44}});
  server={...server,revision:2,settings:mergeSettings(server.settings,{view:{fontSize:31}})};
  slow.resolve({conflict:false,snapshot:structuredClone(server)});await settle();
  expect(useApp.getState().view.fontSize).toBe(44);
  expect(server.settings.view?.fontSize).toBe(44);
});
it('retries revision conflicts while preserving another device’s fields',async()=>{
  await boot();
  mock.api.accountSettingsWrite.mockImplementationOnce(async()=>{
    server={revision:2,settings:{view:{fontSize:27,theme:'light'}},updatedAt:null};
    return {conflict:true,snapshot:structuredClone(server)};
  });
  useApp.setState({view:{...useApp.getState().view,fontSize:32}});
  await syncAccountSettings();
  expect(server.settings.view).toMatchObject({fontSize:32,theme:'light'});
  expect(mock.api.accountSettingsWrite.mock.calls.map(call=>call[1])).toEqual([1,2]);
});
it('ignores unknown and device-specific data downloaded from the server',async()=>{
  server.settings={view:{fontSize:1200,readVoice:'foreign voice',apiKey:'secret'},book:{device:'pc'}} as PersonalSettings;
  useApp.setState({view:{...DEFAULT_VIEW,readVoice:'local voice'}});
  await boot();
  expect(useApp.getState().view.fontSize).toBe(1200);
  expect(useApp.getState().view.readVoice).toBe('local voice');
  expect(loadBookPreferences().device).toBe('auto');
  expect(useApp.getState().view).not.toHaveProperty('apiKey');
});
it('restores guest settings when the previous login no longer exists',async()=>{
  localStorage.setItem('wp.account.guest',JSON.stringify({view:{fontSize:19}}));
  localStorage.setItem('wp.account.last',JSON.stringify([A.url,'a']));
  useApp.setState({view:{...DEFAULT_VIEW,fontSize:70}});
  mock.api.accountStatus.mockResolvedValue({...A,account:null});
  await boot();
  expect(useApp.getState().view.fontSize).toBe(19);
  expect(localStorage.getItem('wp.account.last')).toBeNull();
});
