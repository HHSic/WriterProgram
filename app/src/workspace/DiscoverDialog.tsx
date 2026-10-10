import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import type { Discovered, DiscoveredKind } from '../api/types';
import { Modal } from '../components/Modal';
import { closeDialog, refreshOverview, refreshCardCounts, saveEverything, showToast, toastError, useApp, jumpTo } from '../store';

export function DiscoverDialog() {
  const root = useApp(s => s.overview!.root);
  const docId = useApp(s => s.activeDocId);
  const [scope, setScope] = useState<'all' | 'doc'>(docId ? 'doc' : 'all');
  const [found, setFound] = useState<Discovered | null>(null);
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState<Map<string, DiscoveredKind>>(new Map());
  const alive = useRef(true);
  useEffect(() => () => { alive.current = false; }, []);
  const scan = async () => {
    setBusy(true);
    try {
      if (!(await saveEverything())) return;
      const result = await api.discoverScan(root, scope === 'doc' && docId ? [docId] : undefined);
      if (alive.current) { setFound(result); setSelected(new Map()); }
    } catch (e) { toastError('후보를 찾지 못함', e); }
    finally { if (alive.current) setBusy(false); }
  };
  const add = async () => {
    setBusy(true);
    try {
      const ids = await api.discoverAdd(root, [...selected].map(([name, kind]) => ({ name, kind })));
      if (useApp.getState().overview?.root !== root) return;
      await refreshOverview(); await refreshCardCounts(); showToast({text:`설정집에 ${ids.length}개 추가됨`}); closeDialog();
    } catch(e) { toastError('설정 카드를 추가하지 못함', e); }
    finally { if (alive.current) setBusy(false); }
  };
  return <Modal title="인물·장소 자동 찾기" width={700} onClose={closeDialog} footer={<>
    <button className="btn" disabled={busy} onClick={closeDialog}>닫기</button>
    <button className="btn primary" disabled={busy || !selected.size} onClick={() => void add()}>선택한 {selected.size}개 등록</button>
  </>}>
    <p className="hint">원고의 이름과 문맥을 기기 안에서 살펴 후보를 찾습니다. 결과를 확인하고 등록할 이름을 골라 주세요.</p>
    <div className="row"><select aria-label="찾을 범위" disabled={busy} value={scope} onChange={e => { setScope(e.target.value as 'all'|'doc'); setFound(null); setSelected(new Map()); }}>
      {docId && <option value="doc">현재 문서</option>}<option value="all">전체 원고</option>
    </select><button className="btn" disabled={busy} onClick={() => void scan()}>{busy ? '처리 중…' : '후보 찾기'}</button></div>
    {found && <><p role="status">후보 {found.candidates.length}개{found.skipped ? ` · 읽지 못한 문서 ${found.skipped}개` : ''}{found.truncated ? ' · 최대 500개까지 표시' : ''}</p>
      {!found.candidates.length && <p>찾은 후보가 없습니다. 설정집에서 이름을 직접 등록할 수도 있습니다.</p>}
      <button className="btn small" disabled={busy} onClick={() => setSelected(new Map(found.candidates.filter(c => !c.existing).map(c => [c.name, c.kind])))}>새 후보 모두 선택</button>
      <button className="btn small" disabled={busy} onClick={() => setSelected(new Map())}>선택 해제</button>
      <ul className="discover-list">{found.candidates.map(c => <li key={c.name}>
        <label><input type="checkbox" disabled={busy || !!c.existing} checked={selected.has(c.name)} onChange={e => setSelected(old => { const next = new Map(old); if(e.target.checked) next.set(c.name, c.kind); else next.delete(c.name); return next; })}/><strong>{c.name}</strong> · {c.count}회 {c.existing ? ' · 등록됨' : ''}</label>
        <select aria-label={`${c.name} 분류`} disabled={busy || !selected.has(c.name)} value={selected.get(c.name) ?? c.kind} onChange={e => setSelected(old => new Map(old).set(c.name, e.target.value as DiscoveredKind))}><option value="person">인물</option><option value="place">장소</option></select>
        {c.samples.map((s,i) => <button key={i} className="discover-sample" onClick={() => { closeDialog(); jumpTo({ docId:s.docId, ...s.hit }); }}>{s.hit.before}<b>{s.hit.text}</b>{s.hit.after}</button>)}
      </li>)}</ul></>}
  </Modal>;
}
