// 고쳐 열기 (docs/safety-design.md S4): a document whose file is there but
// cannot be read (saved as ANSI in Notepad, empty, its details broken). It
// is read the way importing reads files and shown before the writer agrees;
// the original stays next to it, nothing is removed.

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { MendPreview } from '../api/types';
import { Modal } from '../components/Modal';
import { errorText, num } from '../lib/format';
import { UNTITLED } from '../lib/labels';
import { closeDialog, mendDoc, useApp } from '../store';
import { UNREADABLE_TITLE } from './UnreadableItem';

const ENCODING: Record<string, string> = {
  'utf-8': 'UTF-8',
  'utf-16': 'UTF-16',
  'euc-kr': 'EUC-KR(메모장의 ANSI)',
};

export function MendDialog({ docId }: { docId: string }) {
  const ov = useApp((s) => s.overview)!;
  const item = ov.unreadable.find((u) => u.id === docId);
  const [preview, setPreview] = useState<MendPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const repairable = item?.repairable ?? true;

  useEffect(() => {
    if (!repairable) return;
    let alive = true;
    api.docMendPreview(ov.root, docId).then(
      (p) => alive && setPreview(p),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [ov.root, docId, repairable]);

  const mend = async () => {
    if (!preview || busy) return;
    setBusy(true);
    if (await mendDoc(docId)) closeDialog();
    else setBusy(false);
  };

  const title = item?.titleGuess.trim() || preview?.title.trim() || UNREADABLE_TITLE;
  return (
    <Modal
      title="고쳐 열기"
      onClose={closeDialog}
      width={560}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            닫기
          </button>
          {repairable && (
            <button type="button" className="btn primary" disabled={!preview || busy} onClick={() => void mend()}>
              {busy ? '고치는 중…' : '고쳐 열기'}
            </button>
          )}
        </>
      }
    >
      <div className="form">
        <p className="dialog-text">
          ‘{title}’ 파일을 열 수 없습니다 · {item?.reason ?? preview?.reason ?? ''}
        </p>
        {!repairable && (
          <p className="dialog-text">
            다른 프로그램(동기화 프로그램, 백신 등)이 이 파일을 쓰고 있거나 읽을 권한이 없습니다. 잠시 뒤 작품을 다시 열어 보세요.
          </p>
        )}
        {error && <p className="empty-note error-note">{error}</p>}
        {preview && (
          <>
            <p className="dialog-text">
              {ENCODING[preview.encoding] ?? preview.encoding} 글자로 읽으면 아래와 같습니다. 맞으면 ‘고쳐 열기’를 누르세요.
              {preview.newFront && ' 제목 등 회차 정보는 새로 붙이고, 본문은 그대로 살립니다.'}
            </p>
            <div className="mend-head">
              <strong>{preview.title.trim() || UNTITLED}</strong>
              <span>{num(preview.chars)}자</span>
            </div>
            <pre className="mend-preview">{preview.text || '(내용 없음)'}</pre>
            <p className="hint">
              원래 파일은 지우지 않고 같은 폴더에 ‘{docId}.md.broken-시각’ 이름으로 남겨 둡니다.
            </p>
          </>
        )}
      </div>
    </Modal>
  );
}
