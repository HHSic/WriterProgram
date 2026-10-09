import { useEffect, useState } from 'react';
import { api } from '../api';
import type { DriveInfo, DriveProvider, ShareLink } from '../api/types';
import { Modal } from '../components/Modal';
import { errorText, timeLabel } from '../lib/format';
import { allManuscript, closeDialog, openDialog, openWeb, saveEverything, useApp } from '../store';

export function ShareDialog() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const docs = allManuscript(ov);
  const [picked, setPicked] = useState(() => new Set(docs.some((d) => d.id === activeDocId) ? [activeDocId!] : docs.map((d) => d.id)));
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [provider, setProvider] = useState<DriveProvider | ''>('');
  const [links, setLinks] = useState<ShareLink[]>([]);
  const [created, setCreated] = useState<ShareLink | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState('');

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [status, links] = await Promise.all([api.driveStatus(), api.shareList(ov.root)]);
        if (!alive) return;
        const connected = status.drives.filter((d) => d.registered && d.account);
        setDrives(connected);
        setProvider(connected[0]?.provider ?? '');
        setLinks(links);
      } catch (e) { if (alive) setError(errorText(e)); }
      finally { if (alive) setLoading(false); }
    })();
    return () => { alive = false; };
  }, [ov.root]);

  const publish = async () => {
    if (!provider) return;
    setBusy(true);
    setError('');
    setCopied('');
    try {
      if (!await saveEverything()) throw new Error('원고를 저장하지 못했습니다. 저장 문제를 해결한 뒤 공유하세요.');
      const ids = docs.filter((d) => picked.has(d.id)).map((d) => d.id);
      const link = await api.shareCreate(ov.root, ids, provider);
      setCreated(link);
      setLinks((links) => [link, ...links]);
    } catch (e) { setError(errorText(e)); }
    finally { setBusy(false); }
  };

  const copy = async (link: ShareLink) => {
    try {
      await navigator.clipboard.writeText(link.url);
      setCopied(link.id);
      setError('');
    } catch { setError('링크를 복사하지 못했습니다. 주소를 선택해 직접 복사하세요.'); }
  };

  const revoke = async (link: ShareLink) => {
    setBusy(true);
    setError('');
    try {
      await api.shareRevoke(ov.root, link.id);
      setLinks((links) => links.filter((l) => l.id !== link.id));
      setCreated((last) => last?.id === link.id ? null : last);
    } catch (e) { setError(errorText(e)); }
    finally { setBusy(false); }
  };

  return (
    <Modal title="링크로 공유" width={620} onClose={() => { if (!busy) closeDialog(); }} footer={
      <>
        <button type="button" className="btn" disabled={busy} onClick={closeDialog}>닫기</button>
        <button type="button" className="btn primary" disabled={loading || busy || !provider || !picked.size || !api.isDesktop} onClick={() => void publish()}>
          {busy ? '처리 중…' : '링크 만들기'}
        </button>
      </>
    }>
      <div className="form share-dialog">
        <p className="dialog-text">연결한 드라이브에 선택한 원고의 문서 사본을 올립니다. 링크를 가진 사람은 드라이브 미리보기에서 읽거나 내려받을 수 있습니다. 수정한 원고는 새 링크로 공유하세요.</p>
        {!api.isDesktop && <p className="hint">온라인 공유는 설치형 앱에서 쓸 수 있습니다.</p>}
        {loading && <p role="status">공유 연결을 확인하는 중…</p>}
        {error && <p className="warn-text" role="alert">{error}</p>}
        {!loading && !drives.length && api.isDesktop && <div className="row wrap">
          <span className="hint">공유하려면 앱에서 드라이브 계정을 먼저 연결하세요.</span>
          <button type="button" className="btn" onClick={() => openDialog({kind: 'drives'})}>드라이브 연결…</button>
        </div>}
        <fieldset className="share-selection" disabled={busy}>
          <legend>공유할 원고</legend>
          <div className="row">
            <button type="button" className="btn small" onClick={() => setPicked(new Set(docs.map((d) => d.id)))}>모두 선택</button>
            <button type="button" className="btn small" onClick={() => setPicked(new Set())}>선택 해제</button>
            <span className="hint">{picked.size}개 선택</span>
          </div>
          <ul className="share-chapters">
            {docs.map((doc, i) => <li key={doc.id}><label>
              <input type="checkbox" checked={picked.has(doc.id)} onChange={(e) => setPicked((previous) => {
                const next = new Set(previous);
                if (e.target.checked) next.add(doc.id); else next.delete(doc.id);
                return next;
              })} />
              <span>{i + 1}. {doc.title || '제목 없음'}</span>
            </label></li>)}
          </ul>
          {!docs.length && <p className="hint">공유할 원고가 없습니다.</p>}
          <label className="field"><span className="field-label">공유할 드라이브</span>
            <select value={provider} disabled={!drives.length} onChange={(e) => setProvider(e.target.value as DriveProvider)}>
              {!drives.length && <option value="">연결된 드라이브 없음</option>}
              {drives.map((d) => <option key={d.provider} value={d.provider}>{d.label} · {d.account?.email}</option>)}
            </select>
          </label>
        </fieldset>
        {created && <section className="share-result" aria-label="새 공유 링크" aria-live="polite">
          <strong>공유 링크가 만들어졌습니다</strong>
          <input aria-label="공유 링크 주소" readOnly value={created.url} onFocus={(e) => e.target.select()} />
          <button type="button" className="btn primary" onClick={() => void copy(created)}>{copied === created.id ? '복사됨' : '링크 복사'}</button>
        </section>}
        <section aria-label="공유 링크 관리">
          <h3>공유 중인 링크</h3>
          <p className="hint">이 기기에서 만든 링크입니다. 공유 종료 시 드라이브의 공유용 사본을 지웁니다. 내려받은 사본은 회수할 수 없습니다.</p>
          <ul className="share-links">
            {links.map((link) => <li key={link.id}>
              <div><strong>{link.title}</strong><span className="meta">원고 {link.chapterCount}개 · {timeLabel(link.createdAt)} 공유 · {link.account}</span></div>
              <div className="row wrap">
                <button type="button" className="btn small" disabled={busy} onClick={() => void copy(link)}>{copied === link.id ? '복사됨' : '복사'}</button>
                <button type="button" className="btn small" disabled={busy} onClick={() => { closeDialog(); void openWeb(link.url); }}>열기</button>
                <button type="button" className="btn small" disabled={busy || !drives.some((d) => d.provider === link.provider && d.account?.email.toLowerCase() === link.account.toLowerCase())} onClick={() => void revoke(link)}>공유 종료</button>
              </div>
            </li>)}
          </ul>
          {!links.length && <p className="hint">공유 중인 링크가 없습니다.</p>}
        </section>
      </div>
    </Modal>
  );
}
