// 창작 과정 증명서 만들기 (docs/creation-proof.md §5): what to cover, whether
// to show draft-and-final excerpts, and times as dates only; then a folder
// to put the certificate (HTML) and its proof file (증명자료.json) in.

import { useEffect, useMemo, useState } from 'react';
import { api } from '../api';
import type { ProofOptions } from '../api/types';
import { Modal } from '../components/Modal';
import { UNTITLED, docNumber } from '../lib/labels';
import { allManuscript, closeDialog, saveEverything, showToast, toastError, useApp } from '../store';

type Range = 'all' | 'chapters' | 'period';

const RANGES: { id: Range; label: string }[] = [
  { id: 'all', label: '작품 전체' },
  { id: 'chapters', label: '회차 고르기' },
  { id: 'period', label: '기간' },
];

export function ProofDialog() {
  const ov = useApp((s) => s.overview)!;
  const kind = ov.project.kind;
  const chapters = allManuscript(ov).map((doc, i) => ({ id: doc.id, label: `${docNumber(kind, i + 1)} ${doc.title}`.trim() }));
  const [range, setRange] = useState<Range>('all');
  const [picked, setPicked] = useState<string[]>([]);
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [showText, setShowText] = useState(false);
  const [excerpts, setExcerpts] = useState<string[]>([]);
  const [datesOnly, setDatesOnly] = useState(true);
  const [preview, setPreview] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const options: ProofOptions = useMemo(
    () => ({
      docs: range === 'chapters' ? picked : null,
      from: range === 'period' && from ? from : null,
      to: range === 'period' && to ? to : null,
      excerpts: showText ? excerpts : [],
      datesOnly,
    }),
    [range, picked, from, to, showText, excerpts, datesOnly],
  );
  const ready = range !== 'chapters' || picked.length > 0;
  const covered = range === 'chapters' ? chapters.filter((c) => picked.includes(c.id)) : chapters;
  // Chapters that cannot be read are left out and listed as 빠진 회차; say so before making.
  const leftOut = range === 'chapters' ? [] : ov.unreadable.filter((u) => u.section === 'manuscript');

  useEffect(() => {
    if (!ready) {
      setPreview(null);
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      api.proofPreview(ov.root, options).then(
        (text) => live && setPreview(text),
        () => live && setPreview(null),
      );
    }, 250);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [ov.root, options, ready]);

  const toggle = (list: string[], id: string, on: boolean) => (on ? [...list, id] : list.filter((x) => x !== id));

  const make = async () => {
    if (!ready || busy) return;
    setBusy(true);
    try {
      if (!(await saveEverything())) return;
      const folder = await api.pickFolder('증명서를 저장할 폴더');
      if (!folder) return;
      const written = await api.proofMake(ov.root, options, folder);
      closeDialog();
      showToast({
        text: written.ok
          ? '창작 과정 증명서를 만들었습니다.'
          : '창작 과정 증명서를 만들었지만, 기록에 맞지 않는 곳이 있습니다. 증명서의 확인 결과를 보세요.',
        tone: written.ok ? undefined : 'error',
        action: api.isDesktop ? { label: '폴더 열기', run: () => void api.reveal(written.html) } : undefined,
      });
    } catch (e) {
      toastError('증명서를 만들지 못함', e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="창작 과정 증명서 만들기"
      onClose={closeDialog}
      width={560}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="button" className="btn primary" disabled={!ready || busy} onClick={() => void make()}>
            {busy ? '만드는 중…' : '폴더 골라 만들기'}
          </button>
        </>
      }
    >
      <div className="form">
        <p className="dialog-text">
          창작 일지와 날짜 증명으로, 이 작품을 언제 어떻게 쓰고 고쳤는지 보여 주는 증명서를 만듭니다. 사람이 읽는 증명서(HTML, 인쇄하면
          PDF)와 누구나 검증할 수 있는 증명 자료 파일을 함께 저장합니다.
        </p>

        <fieldset className="field">
          <legend className="field-label">범위</legend>
          <div className="segmented">
            {RANGES.map((r) => (
              <label key={r.id} className={range === r.id ? 'on' : ''}>
                <input type="radio" checked={range === r.id} onChange={() => setRange(r.id)} />
                {r.label}
              </label>
            ))}
          </div>
          {range === 'chapters' && (
            <div className="proof-chapters">
              {chapters.map((c) => (
                <label key={c.id} className="check">
                  <input
                    type="checkbox"
                    checked={picked.includes(c.id)}
                    onChange={(e) => setPicked(toggle(picked, c.id, e.target.checked))}
                  />
                  {c.label}
                </label>
              ))}
            </div>
          )}
          {range === 'period' && (
            <div className="row wrap">
              <input type="date" aria-label="시작하는 날" value={from} onChange={(e) => setFrom(e.target.value)} />
              <span>~</span>
              <input type="date" aria-label="끝나는 날" value={to} onChange={(e) => setTo(e.target.value)} />
            </div>
          )}
        </fieldset>

        <fieldset className="field">
          <legend className="field-label">원고 공개</legend>
          <label className="check">
            <input type="checkbox" checked={showText} onChange={(e) => setShowText(e.target.checked)} />
            고른 회차의 초고와 지금 글 비교를 넣기
          </label>
          <small className="hint">
            넣지 않으면 원고 내용 없이 쓰고 고친 흐름과 지문만 담습니다. 넣으면 고른 회차의 고친 부분이 증명서에 그대로 보입니다.
          </small>
          {showText && (
            <div className="proof-chapters">
              {covered.map((c) => (
                <label key={c.id} className="check">
                  <input
                    type="checkbox"
                    checked={excerpts.includes(c.id)}
                    onChange={(e) => setExcerpts(toggle(excerpts, c.id, e.target.checked))}
                  />
                  {c.label}
                </label>
              ))}
            </div>
          )}
        </fieldset>

        <fieldset className="field">
          <legend className="field-label">시각</legend>
          <label className="check">
            <input type="checkbox" checked={datesOnly} onChange={(e) => setDatesOnly(e.target.checked)} />
            증명서에는 날짜까지만 보이기
          </label>
          <small className="hint">
            몇 시에 썼는지는 생활 습관을 드러낼 수 있습니다. 증명 자료 파일에는 검증을 위해 정확한 시각이 그대로 들어갑니다.
          </small>
        </fieldset>

        {leftOut.length > 0 && (
          <p className="warn-text">
            열 수 없는 회차 {leftOut.length}개({leftOut.map((u) => `‘${u.titleGuess.trim() || UNTITLED}’`).join(', ')})는 증명서에
            담지 못하고 ‘빠진 회차’로 적힙니다. 왼쪽 목록에서 먼저 고쳐 열면 함께 담깁니다.
          </p>
        )}

        <div className="field">
          <span className="field-label">증명서 첫 문장</span>
          <p className="proof-summary">{ready ? (preview ?? '…') : '증명서에 넣을 회차를 골라 주세요.'}</p>
        </div>
      </div>
    </Modal>
  );
}
