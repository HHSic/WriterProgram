import { useEffect, useState } from 'react';
import { api } from '../api';
import type { AccountLogin } from '../api/types';
import { Modal } from '../components/Modal';
import { closeDialog } from '../store';
import { configureAccount, loginAccount, logoutAccount, syncAccountSettings, useAccount } from '../store/account';

export function AccountDialog() {
  const {status,working,syncState,error,notice}=useAccount();
  const [setup,setSetup]=useState(!status.url);
  const [url,setUrl]=useState(status.url||'');
  const [key,setKey]=useState('');
  const [email,setEmail]=useState('');
  const [password,setPassword]=useState('');
  const [domain,setDomain]=useState('');
  const [signup,setSignup]=useState(false);
  useEffect(()=>{if(status.url){setUrl(status.url);setSetup(false);}},[status.url]);
  const login=async(request:AccountLogin)=>{await loginAccount(request);setPassword('');};
  const cancel=()=>void api.accountCancel();
  return <Modal title="계정·설정 동기화" width={520} onClose={()=>{if(working)cancel();closeDialog();}}>
    <div className="form">
      <p className="hint">로그인하면 글꼴·크기·자간·화면 색·도구줄·책 미리보기 설정을 자동으로 불러옵니다.</p>
      {!api.isDesktop&&<p className="hint">브라우저 미리보기에서는 예시 계정으로 동작합니다. 실제 로그인은 설치형 앱에서 사용하세요.</p>}
      {status.account?<section className="form">
        <strong>{status.account.name||status.account.email}</strong><span>{status.account.email}</span>
        <span role="status">{syncState==='saved'?'개인 설정 저장됨':syncState==='loading'?'설정을 동기화하는 중…':syncState==='pending'?'설정 변경을 저장할 예정입니다.':syncState==='error'?'동기화가 필요합니다.':'설정 자동 동기화'}</span>
        <div className="row"><button className="btn" disabled={working||syncState==='loading'} onClick={()=>void syncAccountSettings()}>지금 동기화</button><button className="btn" disabled={working} onClick={()=>void logoutAccount()}>로그아웃</button></div>
      </section>:setup?<form className="form" onSubmit={e=>{e.preventDefault();void configureAccount(url,key).then(ok=>{if(ok){setKey('');setSetup(false);}});}}>
        <label className="field"><span>Supabase 프로젝트 URL</span><input type="url" value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://프로젝트.supabase.co" required disabled={working}/></label>
        <label className="field"><span>Publishable key / anon 공개 키</span><input value={key} onChange={e=>setKey(e.target.value)} autoComplete="off" spellCheck={false} required disabled={working}/></label>
        <p className="hint">앱 운영자가 제공한 연결 정보를 사용하세요. service_role·secret 키는 입력하지 않습니다.</p>
        <div className="row"><button className="btn primary" disabled={working}>서버 연결 저장</button>{status.url&&<button type="button" className="btn" disabled={working} onClick={()=>setSetup(false)}>돌아가기</button>}</div>
      </form>:<>
        <div className="row"><button className="btn" disabled={working} onClick={()=>void login({kind:'oauth',provider:'google'})}>Google로 로그인</button><button className="btn" disabled={working} onClick={()=>void login({kind:'oauth',provider:'kakao'})}>Kakao로 로그인</button></div>
        <form className="form" onSubmit={e=>{e.preventDefault();void login({kind:signup?'signup':'password',email,password});}}>
          <strong>{signup?'앱 계정 만들기':'앱 계정 로그인'}</strong>
          <label className="field"><span>이메일</span><input type="email" value={email} onChange={e=>setEmail(e.target.value)} autoComplete="username" required disabled={working}/></label>
          <label className="field"><span>비밀번호</span><input type="password" value={password} onChange={e=>setPassword(e.target.value)} autoComplete={signup?'new-password':'current-password'} minLength={signup?8:undefined} required disabled={working}/></label>
          <div className="row"><button className="btn primary" disabled={working}>{signup?'가입':'로그인'}</button><button type="button" className="btn" disabled={working} onClick={()=>{setSignup(!signup);setPassword('');}}>{signup?'로그인으로 돌아가기':'새 계정 만들기'}</button></div>
        </form>
        <form className="form" onSubmit={e=>{e.preventDefault();void login({kind:'sso',domain});}}>
          <label className="field"><span>회사·학교 SSO 도메인</span><input value={domain} onChange={e=>setDomain(e.target.value)} placeholder="company.com" required disabled={working}/></label>
          <button className="btn" disabled={working}>SSO로 로그인</button><p className="hint">서버에 연결된 SAML SSO를 사용합니다. Supabase Pro 이상에서 사용할 수 있습니다.</p>
        </form>
        <button className="btn ghost" disabled={working} onClick={()=>setSetup(true)}>서버 연결 설정</button>
      </>}
      {working&&<div className="row"><span role="status">처리하는 중…</span>{!setup&&!status.account&&<button type="button" className="btn" onClick={cancel}>로그인 취소</button>}</div>}
      {notice&&<p className="hint" role="status">{notice}</p>}
      {error&&<p className="hint error-text" role="alert">{error}</p>}
    </div>
  </Modal>;
}
