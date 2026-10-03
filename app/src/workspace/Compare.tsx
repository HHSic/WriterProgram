// 둘 다 보기: two versions of a document side by side, changed paragraphs
// marked, with the choices that fit: this device's text against another
// device's (a save that met another device's edits), a document against a
// copy a sync program left, or a chapter against its rescue copy (비상 보관:
// text that could not be saved into the project last time).

import { Fragment, useEffect, useMemo, useState } from 'react';
import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { CopyAction, CopyInfo, RescueFile } from '../api/types';
import { Modal } from '../components/Modal';
import { blocksFromJSON } from '../editor/counts';
import { peerOf } from '../editor/shared';
import { diffParagraphs, placesChanged, type Piece, type Row } from '../lib/diff';
import { errorText, num, timeLabel } from '../lib/format';
import { UNTITLED } from '../lib/labels';
import { closeDialog, findDoc, keepMine, resolveCopy, setRescueAside, takeRescue, takeTheirs, useApp } from '../store';
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
