// 둘 다 보기: two versions of a document side by side, changed paragraphs
// marked, with the choices that fit: this device's text against another
// device's (a save that met another device's edits), a document against a
// copy a sync program left, or a chapter against its rescue copy (비상 보관:
// text that could not be saved into the project last time).

import { Fragment, useEffect, useMemo, useState } from 'react';
import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { CopyAction, CopyInfo, RescueFile, SnapshotInfo } from '../api/types';
import { Modal } from '../components/Modal';
import { blocksFromJSON } from '../editor/counts';
import { peerOf } from '../editor/shared';
import { diffParagraphs, placesChanged, type Piece, type Row } from '../lib/diff';
import { errorText, num, timeLabel } from '../lib/format';
import { UNTITLED } from '../lib/labels';
import { closeDialog, findDoc, keepMine, resolveCopy, restoreRecord, setRescueAside, takeRescue, takeTheirs, useApp } from '../store';
import { copyWhere } from './Copies';

/** Runs of unchanged paragraphs longer than this are folded. */
const FOLD_AFTER = 4;

function paragraphs(body: JSONContent, sceneBreak: string): string[] {
  return blocksFromJSON(body).map((b) => (b.scene ? sceneBreak : b.lines.join('\n')));
}

function chars(texts: string[]): number {
  return texts.reduce((n, t) => n + Array.from(t).length, 0);
}

export function CompareDialog({ docId, copy, rescue }: { docId: string; copy?: CopyInfo; rescue?: RescueFile }) {
  const ov = useApp((s) => s.overview)!;
  const conflict = useApp((s) => s.conflicts[docId]);
  const [texts, setTexts] = useState<{ left: string[]; right: string[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const title = findDoc(ov, docId)?.doc.title || UNTITLED;
  const scene = ov.project.sceneBreak;

  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        // Left: this device's text (the editor's, when it is open).
        const left = peerOf(docId)?.getJSON() ?? (await api.docLoad(ov.root, docId)).body;
        // Right: the copy, the rescue copy, or what another device saved.
        const right = copy
          ? (await api.copyLoad(ov.root, copy.section, copy.file)).body
          : rescue
            ? await api.rescueLoad(rescue.path)
            : (await api.docLoad(ov.root, docId)).body;
        if (alive) setTexts({ left: paragraphs(left, scene), right: paragraphs(right, scene) });
      } catch (e) {
        if (alive) setError(errorText(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [ov.root, docId, copy, rescue, scene]);

  const rows = useMemo(() => (texts ? diffParagraphs(texts.left, texts.right) : null), [texts]);
  const places = rows ? placesChanged(rows) : 0;
  const leftLabel = copy || rescue ? '지금 글' : '이 기기';
  const rightLabel = copy ? `사본 · ${copyWhere(copy)}` : rescue ? `비상 보관 · ${timeLabel(rescue.saved)}` : '다른 기기';

  const run = async (fn: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    await fn();
    setBusy(false);
    closeDialog();
  };
  const copyAction = (action: CopyAction) => () => run(() => resolveCopy(copy!, action));

  const footer = rescue ? (
    <>
      <button type="button" className="btn" onClick={closeDialog}>
        닫기
      </button>
      <span className="grow" />
      <button type="button" className="btn" disabled={busy} onClick={() => void run(() => setRescueAside(rescue))}>
        지금 글 두기
      </button>
      <button type="button" className="btn primary" disabled={busy} onClick={() => void run(() => takeRescue(rescue))}>
        비상 보관 글로 바꾸기
      </button>
    </>
  ) : copy ? (
    <>
      <button type="button" className="btn" onClick={closeDialog}>
        닫기
      </button>
      <span className="grow" />
      <button type="button" className="btn" disabled={busy} onClick={copyAction('discard')}>
        지금 글 두기 (사본 버리기)
      </button>
      {copy.section !== 'notes' && (
        <button type="button" className="btn" disabled={busy} onClick={copyAction('keepBoth')}>
          둘 다 두기
        </button>
      )}
      <button type="button" className="btn primary" disabled={busy} onClick={copyAction('take')}>
        사본으로 바꾸기
      </button>
    </>
  ) : (
    <>
      <button type="button" className="btn" onClick={closeDialog}>
        닫기
      </button>
      <span className="grow" />
      <button type="button" className="btn" disabled={busy || !conflict} onClick={() => void run(() => keepMine(docId))}>
        이 기기 것으로 저장
      </button>
      <button type="button" className="btn primary" disabled={busy || !conflict} onClick={() => void run(() => takeTheirs(docId))}>
        다른 기기 것 불러오기
      </button>
    </>
  );

  return (
    <Modal title={`둘 다 보기 · ${title}`} onClose={closeDialog} width={1040} footer={footer}>
      {error && <p className="warn-text">비교할 글을 읽지 못함 · {error}</p>}
      {texts && rows && (
        <>
          <p className="compare-summary">
            {places === 0 ? '두 글이 같습니다.' : `달라진 곳 ${num(places)}군데`} · {leftLabel} {num(chars(texts.left))}자 ·{' '}
            {copy ? '사본' : rescue ? '비상 보관' : rightLabel} {num(chars(texts.right))}자
            {!copy && !rescue && <span className="hint">고르지 않은 쪽 글도 기록에 남습니다.</span>}
            {copy && <span className="hint">사본으로 바꾸면 지금 글은 기록에 남습니다.</span>}
            {rescue && <span className="hint">비상 보관 글로 바꾸면 지금 글은 기록에 남습니다. 지금 글을 두어도 비상 보관 파일은 지우지 않습니다.</span>}
          </p>
          <div className="compare" role="table" aria-label="두 글 비교">
            <div className="compare-head" role="row">
              <span role="columnheader">{leftLabel}</span>
              <span role="columnheader">{rightLabel}</span>
            </div>
            <CompareRows rows={rows} />
          </div>
        </>
      )}
    </Modal>
  );
}

/** Current editor text or two retained records, with the same paragraph diff. */
export function HistoryCompareDialog({docId,recordId}:{docId:string;recordId:string}) {
  const ov=useApp(s=>s.overview)!;
  const cannotRestore=useApp(s=>!!s.conflicts[docId]||!!findDoc(s.overview!,docId)?.doc.locked);
  const [records,setRecords]=useState<SnapshotInfo[]|null>(null);
  const [leftId,setLeftId]=useState('current');
  const [rightId,setRightId]=useState(recordId);
  const [texts,setTexts]=useState<{left:string[];right:string[]}|null>(null);
  const [error,setError]=useState<string|null>(null);
  useEffect(()=>{
    let alive=true;
    api.snapshotList(ov.root,docId).then(list=>{if(alive)setRecords(list);},e=>{if(alive)setError(errorText(e));});
    return()=>{alive=false;};
  },[ov.root,docId]);
  useEffect(()=>{
    if(!records)return;
    let alive=true;
    setTexts(null);setError(null);
    void (async()=>{
      try {
        if(!records.some(r=>r.id===rightId)||(leftId!=='current'&&!records.some(r=>r.id===leftId))) throw Error('기록을 찾을 수 없습니다.');
        const left=leftId==='current' ? peerOf(docId)?.getJSON()??(await api.docLoad(ov.root,docId)).body : (await api.snapshotLoad(ov.root,docId,leftId)).body;
        const right=(await api.snapshotLoad(ov.root,docId,rightId)).body;
        if(alive)setTexts({left:paragraphs(left,ov.project.sceneBreak),right:paragraphs(right,ov.project.sceneBreak)});
      } catch(e){if(alive)setError(errorText(e));}
    })();
    return()=>{alive=false;};
  },[ov.root,ov.project.sceneBreak,docId,records,leftId,rightId]);
  const rows=useMemo(()=>texts?diffParagraphs(texts.left,texts.right):null,[texts]);
  const label=(r:SnapshotInfo)=>`${timeLabel(r.at)} · ${r.name||'이름 없는 기록'}`;
  const right=records?.find(r=>r.id===rightId);
  const leftRecord=records?.find(r=>r.id===leftId);
  const leftLabel=leftId==='current'?'지금 글':leftRecord?label(leftRecord):'선택한 기준 이력';
  return <Modal title={`문서 이력 비교 · ${findDoc(ov,docId)?.doc.title||UNTITLED}`} onClose={closeDialog} width={1040} footer={<>
    <button type="button" className="btn" onClick={closeDialog}>닫기</button><span className="grow"/>
    <button type="button" className="btn primary" disabled={cannotRestore||!texts||!right||!!error} onClick={()=>right&&restoreRecord(docId,right)}>선택한 이력으로 복원</button>
  </>}>
    <div className="history-choices">
      <label>기준 원고<select aria-label="기준 원고" value={leftId} onChange={e=>setLeftId(e.target.value)}><option value="current">지금 글</option>{records?.map(r=><option key={r.id} value={r.id}>{label(r)}</option>)}</select></label>
      <label>비교할 이력<select aria-label="비교할 이력" value={rightId} onChange={e=>setRightId(e.target.value)}>{records?.map(r=><option key={r.id} value={r.id}>{label(r)}</option>)}</select></label>
    </div>
    {error&&<p className="warn-text" role="alert">기록을 읽지 못함 · {error}</p>}
    {!texts&&!error&&<p role="status">기록을 불러오는 중…</p>}
    {texts&&rows&&<>
      <p className="compare-summary">{placesChanged(rows)===0?'두 글이 같습니다.':`달라진 곳 ${num(placesChanged(rows))}군데`} · 기준 {num(chars(texts.left))}자 · 이력 {num(chars(texts.right))}자</p>
      <div className="compare" role="table" aria-label="문서 이력 비교"><div className="compare-head" role="row"><span role="columnheader">{leftLabel}</span><span role="columnheader">{right?label(right):'선택한 이력'}</span></div><CompareRows key={`${leftId}/${rightId}`} rows={rows}/></div>
    </>}
    <p className="hint">복원하면 지금 글을 ‘되돌리기 전’ 기록으로 먼저 보관합니다. 비교는 본문과 문단을 기준으로 표시합니다.</p>
  </Modal>;
}

function Pieces({ pieces, side }: { pieces: Piece[]; side: 'left' | 'right' }) {
  return (
    <>
      {pieces.map((p, i) =>
        p.changed ? (
          <mark key={i} className={`diff-${side}`}>
            {p.text}
          </mark>
        ) : (
          <Fragment key={i}>{p.text}</Fragment>
        ),
      )}
    </>
  );
}

function CompareRows({ rows }: { rows: Row[] }) {
  const [opened, setOpened] = useState<Set<number>>(new Set());
  // Long runs of the same paragraphs fold down to their first and last one.
  const items: ({ row: Row; index: number } | { fold: number; count: number })[] = [];
  let i = 0;
  while (i < rows.length) {
    if (rows[i].kind !== 'same') {
      items.push({ row: rows[i], index: i });
      i++;
      continue;
    }
    let j = i;
    while (j < rows.length && rows[j].kind === 'same') j++;
    const run = j - i;
    if (run > FOLD_AFTER && !opened.has(i)) {
      items.push({ row: rows[i], index: i });
      items.push({ fold: i, count: run - 2 });
      items.push({ row: rows[j - 1], index: j - 1 });
    } else {
      for (let k = i; k < j; k++) items.push({ row: rows[k], index: k });
    }
    i = j;
  }

  return (
    <>
      {items.map((item) => {
        if ('fold' in item) {
          return (
            <button
              key={`fold-${item.fold}`}
              type="button"
              className="compare-fold"
              onClick={() => setOpened((prev) => new Set(prev).add(item.fold))}
            >
              같은 문단 {num(item.count)}개 펼치기
            </button>
          );
        }
        const { row, index } = item;
        switch (row.kind) {
          case 'same':
            return (
              <div key={index} className="compare-row same" role="row">
                <p role="cell">{row.text}</p>
                <p role="cell">{row.text}</p>
              </div>
            );
          case 'changed':
            return (
              <div key={index} className="compare-row changed" role="row">
                <p role="cell">
                  <Pieces pieces={row.left} side="left" />
                </p>
                <p role="cell">
                  <Pieces pieces={row.right} side="right" />
                </p>
              </div>
            );
          case 'removed':
            return (
              <div key={index} className="compare-row removed" role="row">
                <p role="cell">
                  <mark className="diff-left">{row.text}</mark>
                </p>
                <p role="cell" className="compare-empty" aria-label="없음" />
              </div>
            );
          case 'added':
            return (
              <div key={index} className="compare-row added" role="row">
                <p role="cell" className="compare-empty" aria-label="없음" />
                <p role="cell">
                  <mark className="diff-right">{row.text}</mark>
                </p>
              </div>
            );
        }
      })}
    </>
  );
}
