// 휴지통: documents, cards and notes put away, to bring back or delete for good.

import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import type { TrashItem } from '../api/types';
import { Modal } from '../components/Modal';
import { num, timeLabel } from '../lib/format';
import { UNTITLED } from '../lib/labels';
import { closeDialog, loadNotes, refreshOverview, toastError, useApp } from '../store';

export function TrashDialog() {
  const root = useApp((s) => s.overview!.root);
  const [items, setItems] = useState<TrashItem[] | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);

  const load = useCallback(
    () =>
      api.trashList(root).then(setItems, (e) => {
        setItems([]);
        toastError('휴지통을 읽지 못함', e);
      }),
    [root],
  );

  useEffect(() => {
    void load();
  }, [load]);

  const restore = async (item: TrashItem) => {
    try {
      await api.trashRestore(root, item.id);
      await refreshOverview();
      if (item.section === 'notes') await loadNotes();
      await load();
    } catch (e) {
      toastError('되살리지 못함', e);
    }
  };

  const remove = async (item: TrashItem) => {
    try {
      await api.trashDelete(root, item.id);
      setConfirming(null);
      await refreshOverview();
      await load();
    } catch (e) {
      toastError('지우지 못함', e);
    }
  };

  return (
    <Modal title="휴지통" onClose={closeDialog} width={560}>
      {items !== null && items.length === 0 && <p className="empty-note">휴지통이 비어 있습니다.</p>}
      <ul className="trash-list">
        {items?.map((item) => (
          <li key={item.id} className="trash-item">
            <div className="grow">
              <strong>{item.title || UNTITLED}</strong>
              <span className="meta">
                {item.file && '다른 기기 사본 · '}
                {item.section === 'cards'
                  ? '설정 카드'
                  : item.section === 'notes'
                    ? '메모'
                    : `${item.section === 'planning' ? '기획' : '원고'} · ${num(item.chars)}자`}{' '}
                · {timeLabel(item.deletedAt)}에 지움
              </span>
            </div>
            {confirming === item.id ? (
              <>
                <span className="warn-text">완전히 지울까요?</span>
                <button type="button" className="btn small danger" onClick={() => void remove(item)}>
                  지우기
                </button>
                <button type="button" className="btn small" onClick={() => setConfirming(null)}>
                  취소
                </button>
              </>
            ) : (
              <>
                <button type="button" className="btn small" onClick={() => void restore(item)}>
                  되살리기
                </button>
                <button type="button" className="btn small ghost" onClick={() => setConfirming(item.id)}>
                  완전히 지우기
                </button>
              </>
            )}
          </li>
        ))}
      </ul>
      <p className="hint">휴지통에 넣은 문서·카드·메모는 30일 동안 보관한 뒤 지워집니다.</p>
    </Modal>
  );
}
