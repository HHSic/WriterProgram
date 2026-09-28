// 다른 곳으로 옮기기: moves the whole project folder, for example into a
// folder OneDrive keeps in step with other devices (store.ts moveProject).

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { Place } from '../api/types';
import { Modal } from '../components/Modal';
import { PlacePicker } from '../components/PlacePicker';
import { closeDialog, moveProject, useApp } from '../store';

function split(path: string): { parent: string; name: string; sep: string } {
  const sep = path.includes('\\') ? '\\' : '/';
  const trimmed = path.replace(/[\\/]+$/, '');
  const at = trimmed.lastIndexOf(sep);
  return { parent: trimmed.slice(0, at), name: trimmed.slice(at + 1), sep };
}

export function MoveDialog() {
  const root = useApp((s) => s.overview!.root);
  const { parent: here, name, sep } = split(root);
  const [places, setPlaces] = useState<Place[] | null>(null);
  const [dest, setDest] = useState(here);
  const same = dest.replace(/[\\/]+$/, '').toLowerCase() === here.toLowerCase();

  useEffect(() => {
    api.storagePlaces().then(setPlaces, () => setPlaces([]));
  }, []);

  return (
    <Modal
      title="다른 곳으로 옮기기"
      onClose={closeDialog}
      width={560}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="button" className="btn primary" disabled={same || !dest} onClick={() => void moveProject(dest)}>
            옮기기
          </button>
        </>
      }
    >
      <div className="form">
        <p className="dialog-text">작품 폴더를 통째로 옮깁니다. 기록과 휴지통도 함께 갑니다.</p>
        <PlacePicker places={places} value={dest} onChange={setDest} here={here} />
        {!same && dest && (
          <small className="hint">
            {dest}
            {sep}
            {name} 폴더로 옮깁니다. 다른 디스크로 옮길 때는 모두 옮겨졌는지 확인한 뒤에 원래 폴더를 지웁니다.
          </small>
        )}
      </div>
    </Modal>
  );
}
