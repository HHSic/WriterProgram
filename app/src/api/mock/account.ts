import type { AccountIdentity, Backend, SettingsSnapshot } from '../types';
import { mergeSettings } from '../../lib/preferences';

let url:string|null=null;
let account:AccountIdentity|null=null;
const settings=new Map<string,SettingsSnapshot>();
const currentKey=()=>JSON.stringify([url,account?.id]);
export const accountMethods = {
  async accountStatus(){return {url,account};},
  async accountConfigure(value, _key){if(account)throw '로그아웃 후 서버 연결을 바꿔 주세요.';url=value.replace(/\/$/,'');return {url,account};},
  async accountLogin(request){
    if(!url)throw '계정 서버 연결을 먼저 설정하세요.';
    if(request.kind==='signup')return {account:null,confirmationRequired:true};
    const email=request.kind==='password'?request.email:'preview@example.com';
    account={id:'preview-'+email.toLowerCase(),email,name:'미리보기 계정'};
    return {account,confirmationRequired:false};
  },
  async accountCancel(){},
  async accountLogout(){account=null;},
  async accountSettingsRead(key){
    if(!account||key!==currentKey())throw '계정이 바뀌었습니다. 다시 로그인하세요.';
    return structuredClone(settings.get(key)||{revision:0,settings:{},updatedAt:null});
  },
  async accountSettingsWrite(key,revision,patch){
    if(!account||key!==currentKey())throw '계정이 바뀌었습니다. 다시 로그인하세요.';
    const old=settings.get(key)||{revision:0,settings:{},updatedAt:null};
    if(old.revision!==revision)return {conflict:true,snapshot:structuredClone(old)};
    const snapshot={revision:old.revision+1,settings:mergeSettings(old.settings,patch),updatedAt:new Date().toISOString()};
    settings.set(key,snapshot);return {conflict:false,snapshot:structuredClone(snapshot)};
  },
} satisfies Partial<Backend>;
