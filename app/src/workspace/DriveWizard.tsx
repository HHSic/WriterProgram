import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import type { DriveAccount, DriveInfo, DriveProvider, DriveRegistration } from '../api/types';
import { Modal } from '../components/Modal';
import { errorText } from '../lib/format';

type Step = 'choose' | 'guide' | 'registration' | 'login' | 'done';

const guides: Record<DriveProvider, { idLabel: string; steps: string[] }> = {
  google: {
    idLabel: '클라이언트 ID',
    steps: [
      'Google Cloud에서 프로젝트를 만들고, API 및 서비스 → 라이브러리에서 Google Drive API를 사용 설정하세요.',
      'Google 인증 플랫폼에서 앱 이름·지원 이메일을 입력하고, 사용자 유형을 외부로 설정하세요. 테스트 중이면 로그인할 계정을 테스트 사용자에 추가하세요.',
      '데이터 액세스에 https://www.googleapis.com/auth/drive.file, openid, email, profile 범위를 추가하세요.',
      '사용자 인증 정보에서 OAuth 클라이언트 ID를 만들고, 애플리케이션 유형을 데스크톱 앱으로 선택하세요. 웹 애플리케이션 유형은 사용할 수 없습니다.',
      '발급된 클라이언트 ID와 클라이언트 보안 비밀을 다음 화면에 붙여 넣으세요. 테스트 상태에서는 Google 정책에 따라 7일마다 다시 로그인해야 할 수 있습니다.',
    ],
  },
  onedrive: {
    idLabel: '애플리케이션(클라이언트) ID',
    steps: [
      'Microsoft Entra 관리 센터에서 앱 등록 → 새 등록을 선택하세요.',
      '지원 계정 유형을 모든 조직 디렉터리의 계정 및 개인 Microsoft 계정으로 선택하세요.',
      '인증에서 모바일 및 데스크톱 애플리케이션 플랫폼을 추가하고, 리디렉션 URI에 http://localhost를 등록하세요.',
      'API 권한에서 Microsoft Graph의 위임된 권한 Files.ReadWrite.AppFolder, User.Read, offline_access를 추가하세요. 조직 정책에 따라 관리자 동의가 필요할 수 있습니다.',
      '개요에 있는 애플리케이션(클라이언트) ID를 복사하세요. 클라이언트 비밀은 만들 필요가 없습니다.',
    ],
  },
  dropbox: {
    idLabel: 'App key',
    steps: [
      'Dropbox App Console에서 Create app을 선택하고, Scoped access → App folder로 앱을 만드세요.',
      'Permissions에서 account_info.read, files.metadata.read, files.content.read, files.content.write, sharing.read, sharing.write를 선택하고 Submit을 누르세요.',
      'Settings의 OAuth 2 Redirect URIs에 아래의 돌아오는 주소를 등록하세요. 포트와 마지막 /까지 같아야 합니다.',
      'Allow public clients (Implicit Grant & PKCE)를 Allow로 설정하세요.',
      'Settings의 App key를 복사하세요. App secret은 필요하지 않습니다. 개발 상태에서는 연결 사용자 수에 제한이 있습니다.',
    ],
  },
};

export function DriveWizard({ drives, initial, onClose }: {
  drives: DriveInfo[];
  initial: DriveInfo | null;
  onClose: () => void;
}) {
  const [selected, setSelected] = useState(initial);
  const [step, setStep] = useState<Step>(initial ? initial.registered ? 'login' : 'guide' : 'choose');
  const [registration, setRegistration] = useState<DriveRegistration>({ clientId: '' });
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState<'load' | 'save' | 'login' | null>(null);
  const [error, setError] = useState('');
  const [account, setAccount] = useState<DriveAccount | null>(null);
  const pending = useRef<typeof busy>(null);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (pending.current === 'login') void api.driveCancel().catch(() => {});
    };
  }, []);

  const close = () => {
    // A local settings write finishes before the wizard can be dismissed.
    if (pending.current === 'save' || pending.current === 'load') return;
    onClose();
  };
  const start = (provider: DriveInfo) => {
    setSelected(provider);
    setRegistration({ clientId: '' });
    setError('');
    setDirty(false);
    setStep(provider.registered ? 'login' : 'guide');
  };
  const loadRegistration = async () => {
    if (!selected || pending.current) return;
    if (dirty) { setStep('registration'); return; }
    pending.current = 'load';
    setBusy('load');
    setError('');
    try {
      const saved = await api.driveRegistrationGet(selected.provider);
      if (!alive.current) return;
      setRegistration(saved ?? { clientId: '' });
      setDirty(false);
      setStep('registration');
    } catch (e) {
      if (alive.current) setError(errorText(e));
    } finally {
      pending.current = null;
      if (alive.current) setBusy(null);
    }
  };
  const save = async () => {
    if (!selected || pending.current) return;
    pending.current = 'save';
    setBusy('save');
    setError('');
    try {
      await api.driveRegistrationSave(selected.provider, {
        ...registration,
        clientId: registration.clientId.trim(),
        clientSecret: selected.provider === 'google' ? registration.clientSecret?.trim() : null,
      });
      if (!alive.current) return;
      setSelected({ ...selected, registered: true });
      setDirty(false);
      setStep('login');
    } catch (e) {
      if (alive.current) setError(errorText(e));
    } finally {
      pending.current = null;
      if (alive.current) setBusy(null);
    }
  };
  const login = async () => {
    if (!selected || pending.current) return;
    pending.current = 'login';
    setBusy('login');
    setError('');
    try {
      const result = await api.driveConnect(selected.provider);
      if (!alive.current) return;
      setAccount(result);
      setStep('done');
    } catch (e) {
      if (alive.current) setError(errorText(e));
    } finally {
      pending.current = null;
      if (alive.current) setBusy(null);
    }
  };
  const cancel = async () => {
    try { await api.driveCancel(); }
    catch (e) { if (alive.current) setError(errorText(e)); }
  };
  const openConsole = async () => {
    if (!selected) return;
    setError('');
    try { await api.driveRegistrationOpen(selected.provider); }
    catch (e) { if (alive.current) setError(errorText(e)); }
  };
  const update = (patch: Partial<DriveRegistration>) => {
    setRegistration((value) => ({ ...value, ...patch }));
    setDirty(true);
    setError('');
  };
  const validText = (text?: string | null) => !!text?.trim() && text.trim().length <= 2048 && !/\s/.test(text.trim());
  const valid = validText(registration.clientId) && (selected?.provider !== 'google' || validText(registration.clientSecret));
  const number = step === 'choose' ? 1 : step === 'guide' || step === 'registration' ? 2 : step === 'login' ? 3 : 4;

  return (
    <Modal title="드라이브 연결 마법사" onClose={close} width={620} dirty={dirty}
      footer={<>
        {step === 'done' ? <button type="button" className="btn primary" onClick={close}>완료</button> : <>
          {step !== 'choose' && !(step === 'guide' && dirty) && busy !== 'login' && <button type="button" className="btn" disabled={busy !== null}
            onClick={() => { setError(''); setStep(step === 'registration' ? 'guide' : 'choose'); }}>이전</button>}
          {step === 'guide' && <button type="button" className="btn primary" disabled={busy !== null} onClick={() => void loadRegistration()}>등록 정보 입력</button>}
          {step === 'registration' && <button type="button" className="btn primary" disabled={!valid || busy !== null} onClick={() => void save()}>{busy === 'save' ? '저장 중…' : '저장하고 다음'}</button>}
          {step === 'login' && (busy === 'login'
            ? <button type="button" className="btn" onClick={() => void cancel()}>로그인 취소</button>
            : <button type="button" className="btn primary" onClick={() => void login()}>브라우저에서 로그인</button>)}
        </>}
      </>}>
      <div className="form drive-wizard">
        <ol className="drive-wizard-steps" aria-label="연결 단계">
          {['드라이브 선택', '앱 등록', '계정 로그인', '완료'].map((label, i) =>
            <li key={label} aria-current={number === i + 1 ? 'step' : undefined}>{i + 1}. {label}</li>)}
        </ol>
        {!api.isDesktop && <p className="hint">브라우저 미리보기입니다. 연결과 저장은 예시이며 실제 드라이브에 연결되지 않습니다.</p>}
        {step === 'choose' && <>
          <p className="dialog-text">작품을 맞추거나 공유 링크를 만들 드라이브를 선택하세요.</p>
          <ul className="drive-list">
            {drives.map((drive) => <li key={drive.provider} className="drive-item">
              <div className="grow"><strong>{drive.label}</strong><span className="meta">{drive.account ? `${drive.account.email} · 연결됨` : drive.registered ? '앱 등록됨 · 바로 로그인할 수 있습니다' : '처음 연결 · 앱 등록부터 안내합니다'}</span></div>
              <button type="button" className="btn small primary" disabled={!!drive.account} onClick={() => start(drive)}>선택</button>
            </li>)}
          </ul>
        </>}
        {selected && step === 'guide' && <>
          <h3>{selected.label} 앱 등록 안내</h3>
          <p className="dialog-text">처음 한 번, 드라이브 회사에 앱을 등록해야 합니다. 아래 설정으로 등록하고 발급된 값을 이 앱에 입력하세요.</p>
          <button type="button" className="btn" disabled={busy !== null} onClick={() => void openConsole()}>등록 페이지 열기</button>
          <ol className="drive-wizard-guide">{guides[selected.provider].steps.map((text) => <li key={text}>{text}</li>)}</ol>
          {selected.provider === 'dropbox' && <label className="field"><span className="field-label">돌아오는 주소 (Redirect URI)</span>
            <input readOnly value={`http://localhost:${registration.redirectPort ?? 53682}/`} onFocus={(e) => e.currentTarget.select()} /></label>}
        </>}
        {selected && step === 'registration' && <>
          <h3>{selected.label} 등록 정보</h3>
          <p className="dialog-text">등록 페이지에서 받은 값을 입력하세요. 저장하면 이 기기에 적용되어 바로 로그인할 수 있습니다.</p>
          <label className="field"><span className="field-label">{guides[selected.provider].idLabel}</span>
            <input autoFocus value={registration.clientId} maxLength={2048} autoComplete="off" spellCheck={false} disabled={busy !== null} onChange={(e) => update({ clientId: e.target.value })} /></label>
          {selected.provider === 'google' && <label className="field"><span className="field-label">클라이언트 보안 비밀</span>
            <input type="password" value={registration.clientSecret ?? ''} maxLength={2048} autoComplete="new-password" disabled={busy !== null} onChange={(e) => update({ clientSecret: e.target.value })} />
            <small className="hint">Google 계정 비밀번호가 아닙니다. 데스크톱 앱 등록에서 받은 값입니다.</small>
          </label>}
          <small className="hint">등록 정보는 이 기기에만 저장합니다. 계정 로그인 정보는 운영체제의 자격 증명 저장소에 보관합니다.</small>
        </>}
        {selected && step === 'login' && <>
          <h3>{selected.label} 계정 연결</h3>
          <p className="dialog-text">{busy === 'login' ? '브라우저에서 계정을 선택하고 권한을 허용하세요. 로그인 완료를 기다리고 있습니다.' : '앱 등록이 준비되었습니다. 브라우저에서 계정을 선택하고 이 앱의 접근을 허용하세요.'}</p>
          <ul className="drive-notes"><li>계정 비밀번호는 드라이브 회사의 로그인 페이지에만 입력하세요.</li><li>이 앱이 만든 파일을 맞추고 공유하는 권한을 요청합니다.</li></ul>
          {busy !== 'login' && <button type="button" className="btn" disabled={busy !== null} onClick={() => void loadRegistration()}>등록 정보 수정</button>}
        </>}
        {selected && step === 'done' && <>
          <h3>{selected.label} 연결 완료</h3>
          <p className="dialog-text" role="status">{account?.name} · {account?.email}</p>
          <p className="dialog-text">작품 설정의 ‘드라이브와 맞추기’에서 동기화를 시작하거나, ‘링크로 공유’에서 공유 링크를 만들 수 있습니다.</p>
        </>}
        {error && <p className="warn-text" role="alert">{error}</p>}
      </div>
    </Modal>
  );
}
