// 교정본 주고받기: what was sent to editors, newest first, and for each one
// the corrected file taken back (교정본 가져오기) or its review opened.

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { ExchangeInfo } from '../api/types';
import { Icon } from '../components/Icon';
import { Modal } from '../components/Modal';
import { errorText } from '../lib/format';
import { dayLabel, exchangeStateText, sentChaptersText } from '../lib/review';
import { closeDialog, openDialog, openReview, takeBackCorrected, useApp } from '../store';

const KIND_NAME = { hwpx: '한글 파일', docx: 'Word 파일' } as const;

export function ExchangesDialog() {
  const root = useApp((s) => s.overview!.root);
  const version = useApp((s) => s.exchangesVersion);
  const [list, setList] = useState<ExchangeInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    api.exchangeList(root).then(
      (l) => alive && setList(l),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [root, version]);

  const takeBack = async (id: string) => {
    if (busy) return;
    setBusy(true);
    await takeBackCorrected(id);
    setBusy(false);
  };

  return (
    <Modal
      title="교정본 주고받기"
      onClose={closeDialog}
      width={620}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            닫기
          </button>
          <span className="grow" />
          <button type="button" className="btn primary" onClick={() => openDialog({ kind: 'export', toEditor: true })}>
            편집자에게 보내기…
          </button>
        </>
      }
    >
      {error && <p className="warn-text">보낸 원고 기록을 읽지 못함 · {error}</p>}
      {list && list.length === 0 && (
        <p className="empty-note exchanges-empty">
          아직 편집자에게 보낸 원고가 없습니다. ‘편집자에게 보내기’로 한글이나 Word 파일을 만들면 보낸 원고가 여기에 남고, 편집자가 고친
          파일을 받으면 그 원고와 비교해 볼 수 있습니다.
        </p>
      )}
      {list && list.length > 0 && (
        <ul className="exchanges">
          {list.map((ex) => (
            <li key={ex.id}>
              <Icon name="doc" size={16} />
              <div className="exchange-main">
                <div className="exchange-title">
                  <strong>{dayLabel(ex.created)} 보냄</strong>
                  <span>
                    {KIND_NAME[ex.kind]} · {sentChaptersText(ex)}
                  </span>
                </div>
                <span className="exchange-files ellipsis" title={ex.files.join(', ')}>
                  {ex.files.join(', ')}
                </span>
                <span className={`exchange-state${ex.pending ? ' open' : ''}`}>{exchangeStateText(ex)}</span>
              </div>
              <div className="exchange-actions">
                {ex.received.length > 0 && (
                  <button type="button" className="btn small" onClick={() => void openReview(ex.id)}>
                    {ex.pending ? '검토 이어 하기' : '검토 보기'}
                  </button>
                )}
                <button
                  type="button"
                  className={`btn small${ex.received.length ? '' : ' primary'}`}
                  disabled={busy}
                  onClick={() => void takeBack(ex.id)}
                >
                  {ex.received.length ? '다른 교정본 가져오기…' : '교정본 가져오기…'}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <p className="hint">한글(hwpx)과 Word(docx) 교정본을 읽습니다. 옛 한글 파일(.hwp)로 받았다면 한글에서 한글 문서(.hwpx)로 저장해 주세요.</p>
    </Modal>
  );
}
