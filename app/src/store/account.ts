import { create } from 'zustand';
import { api } from '../api';
import type { AccountLogin, AccountStatus, PersonalSettings, SettingsSnapshot } from '../api/types';
import { applyAccessorySettings, cleanSettings, currentSettings, hasSettings, mergeSettings, settingsDiff } from '../lib/preferences';
import { PREFERENCES_CHANGED } from '../lib/preferenceEvents';
import { saveView } from '../lib/view';
import { errorText } from '../lib/format';
import { useApp } from './state';

type SyncState = 'idle' | 'loading' | 'saved' | 'pending' | 'error';
export const useAccount = create<{
  status: AccountStatus; working: boolean; syncState: SyncState; error: string; notice: string;
}>(()=>({status:{url:null,account:null},working:false,syncState:'idle',error:'',notice:''}));
const GUEST = 'wp.account.guest';
const LAST = 'wp.account.last';
const CACHE = 'wp.account.preferences.';
let epoch=0, activeKey:string|null=null, applying=false, running=false;
let base: SettingsSnapshot={revision:0,settings:{},updatedAt:null};
let pending:PersonalSettings={}, previous:PersonalSettings={};
let timer:ReturnType<typeof setTimeout>|undefined;
let installed=false;
const now=()=>currentSettings(useApp.getState().view);
const keyOf=(status:AccountStatus)=>status.url&&status.account?JSON.stringify([status.url,status.account.id]):null;
function read(key:string):unknown {try{return JSON.parse(localStorage.getItem(key)||'null');}catch{return null;}}
function write(key:string,value:unknown){try{localStorage.setItem(key,JSON.stringify(value));}catch{/* Continue in memory. */}}
function remove(key:string){try{localStorage.removeItem(key);}catch{/* Continue in memory. */}}
function guest():PersonalSettings{return cleanSettings(read(GUEST)||now());}
function cache() {if(activeKey)write(CACHE+activeKey,{snapshot:base,pending});}
function apply(settings:PersonalSettings) {
  applying=true;
  try {
    const clean=cleanSettings(settings);
    const view={...useApp.getState().view,...clean.view};
    saveView(view);useApp.setState({view});
    applyAccessorySettings(clean);
    previous=now();
  } finally {applying=false;}
}
function schedule() {
  clearTimeout(timer);
  timer=setTimeout(()=>void syncAccountSettings(),1000);
}
function changed() {
  if(applying)return;
  const current=now(), patch=settingsDiff(previous,current);
  previous=current;
  if(!hasSettings(patch))return;
  if(activeKey) {
    pending=mergeSettings(pending,patch);cache();
    useAccount.setState({syncState:'pending'});
    schedule();
  } else if(!read(LAST))write(GUEST,current);
}
function dropSubmitted(submitted:PersonalSettings) {
  for(const section of ['view','ribbon','book'] as const) {
    const live=pending[section] as Record<string,unknown>|undefined;
    for(const [key,value] of Object.entries(submitted[section]||{}))if(live?.[key]===value)delete live[key];
  }
}
/** Read before writing; field patches and revisions preserve changes from other devices. */
export async function syncAccountSettings() {
  const key=activeKey, version=epoch;
  if(!key||running)return;
  running=true;useAccount.setState({syncState:'loading',error:''});
  const current=()=>version===epoch&&key===activeKey;
  try {
    const remote=await api.accountSettingsRead(key);
    if(!current())return;
    base={...remote,settings:cleanSettings(remote.settings)};
    if(remote.revision===0&&!hasSettings(base.settings))pending=mergeSettings(now(),pending);
    apply(mergeSettings(mergeSettings(guest(),base.settings),pending));
    for(let attempt=0;hasSettings(pending)&&attempt<3;attempt++) {
      const sent=structuredClone(pending);
      const result=await api.accountSettingsWrite(key,base.revision,sent);
      if(!current())return;
      base={...result.snapshot,settings:cleanSettings(result.snapshot.settings)};
      if(!result.conflict)dropSubmitted(sent);
      apply(mergeSettings(mergeSettings(guest(),base.settings),pending));
      cache();
    }
    useAccount.setState({syncState:hasSettings(pending)?'pending':'saved',error:''});
    if(hasSettings(pending))schedule();
  } catch(error) {
    if(current()){cache();useAccount.setState({syncState:'error',error:'설정을 동기화하지 못했습니다. 이 기기의 변경은 보관됩니다. '+errorText(error)});}
  } finally {if(current())running=false;}
}
function adopt(status:AccountStatus) {
  epoch++;clearTimeout(timer);running=false;
  useAccount.setState({status,error:'',notice:'',syncState:'idle'});
  activeKey=keyOf(status);pending={};base={revision:0,settings:{},updatedAt:null};
  if(!activeKey) {
    if(read(LAST)){apply(guest());remove(LAST);}
    previous=now();write(GUEST,previous);return;
  }
  const cached=read(CACHE+activeKey) as {snapshot?:SettingsSnapshot;pending?:PersonalSettings}|null;
  if(cached?.snapshot&&Number.isSafeInteger(cached.snapshot.revision)&&cached.snapshot.revision>=0) {
    base={...cached.snapshot,settings:cleanSettings(cached.snapshot.settings)};
  }
  pending=cleanSettings(cached?.pending);
  apply(mergeSettings(mergeSettings(guest(),base.settings),pending));
  write(LAST,activeKey);cache();
  void syncAccountSettings();
}
export async function loadAccount() {
  const version=++epoch;
  useAccount.setState({working:true,error:'',notice:''});
  try {
    const status=await api.accountStatus();
    if(version===epoch)adopt(status);
  } catch(error) {
    if(version===epoch)useAccount.setState({error:errorText(error)});
  } finally {if(version===epoch||version+1===epoch)useAccount.setState({working:false});}
}
export async function configureAccount(url:string,publishableKey:string):Promise<boolean> {
  useAccount.setState({working:true,error:'',notice:''});
  try {adopt(await api.accountConfigure(url,publishableKey));return true;}
  catch(error){useAccount.setState({error:errorText(error)});return false;}
  finally{useAccount.setState({working:false});}
}
export async function loginAccount(request:AccountLogin):Promise<boolean> {
  const version=epoch;
  useAccount.setState({working:true,error:'',notice:''});
  try {
    const result=await api.accountLogin(request);
    if(version!==epoch)return false;
    if(result.confirmationRequired){useAccount.setState({notice:'가입 확인 메일을 확인하세요. 메일 인증 후 로그인할 수 있습니다.'});return false;}
    if(!result.account)throw new Error('로그인한 계정을 확인하지 못했습니다.');
    adopt({...useAccount.getState().status,account:result.account});return true;
  } catch(error){if(version===epoch)useAccount.setState({error:errorText(error)});return false;}
  finally{useAccount.setState({working:false});}
}
export async function logoutAccount():Promise<boolean> {
  useAccount.setState({working:true,error:'',notice:''});
  try {
    cache();
    await api.accountLogout();
    adopt({...useAccount.getState().status,account:null});return true;
  } catch(error){useAccount.setState({error:errorText(error)});return false;}
  finally{useAccount.setState({working:false});}
}
/** App-wide, independent of opening/closing a manuscript. */
export function installAccountSettings() {
  if(installed)return ()=>{};
  installed=true;
  previous=now();
  if(!read(GUEST)&&!read(LAST))write(GUEST,previous);
  const unsubscribe=useApp.subscribe((state,old)=>{if(state.view!==old.view)changed();});
  window.addEventListener(PREFERENCES_CHANGED,changed);
  window.addEventListener('focus',syncOnFocus);
  const interval=setInterval(()=>void syncAccountSettings(),60_000);
  void loadAccount();
  return ()=>{
    installed=false;epoch++;clearTimeout(timer);clearInterval(interval);
    cache();unsubscribe();window.removeEventListener(PREFERENCES_CHANGED,changed);window.removeEventListener('focus',syncOnFocus);
    activeKey=null;running=false;
  };
}
function syncOnFocus(){void syncAccountSettings();}
