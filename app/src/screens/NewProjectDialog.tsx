import { useEffect, useState, type FormEvent } from 'react';
import { api } from '../api';
import type { Place, ProjectKind } from '../api/types';
import { Modal } from '../components/Modal';
import { PlacePicker } from '../components/PlacePicker';
import { fileSafe } from '../lib/format';
import { closeDialog, enterProject, toastError } from '../store';

const KINDS: { kind: ProjectKind; label: string; hint: string }[] = [
  { kind: 'webnovel', label: '웹소설', hint: '회차 단위로 연재합니다. 비축·예약·연재됨 상태와 회차 목표 분량을 씁니다.' },
  { kind: 'print', label: '출판 장편', hint: '장 단위로 씁니다. 원고지 매수를 보고, 편집자와 교정을 주고받습니다.' },
];

export function NewProjectDialog() {
  const [title, setTitle] = useState('');
  const [kind, setKind] = useState<ProjectKind>('webnovel');
  const [parent, setParent] = useState('');
  const [goal, setGoal] = useState('5000');
  const [countSpaces, setCountSpaces] = useState(true);
  const [firstChapter, setFirstChapter] = useState(true);
  const [busy, setBusy] = useState(false);
  const [places, setPlaces] = useState<Place[] | null>(null);

  useEffect(() => {
    api.defaultLocation().then(setParent, () => setParent(''));
    api.storagePlaces().then(setPlaces, () => setPlaces([]));
  }, []);

  const pickKind = (k: ProjectKind) => {
    setKind(k);
    setGoal(k === 'webnovel' ? '5000' : '');
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !parent || busy) return;
    setBusy(true);
    try {
      const target = Number.parseInt(goal.replace(/[^0-9]/g, ''), 10);
      const ov = await api.projectCreate({
        parent,
        title: title.trim(),
        kind,
        perDocGoal: Number.isFinite(target) && target > 0 ? target : null,
        countSpaces,
        firstChapter,
      });
      closeDialog();
      enterProject(ov);
    } catch (err) {
      toastError('작품을 만들지 못함', err);
      setBusy(false);
    }
  };

  const sep = parent.includes('\\') ? '\\' : '/';
  const noun = kind === 'webnovel' ? '회차' : '장';

  return (
    <Modal
      title="새 작품"
      onClose={closeDialog}
      width={560}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="submit" form="new-project" className="btn primary" disabled={!title.trim() || !parent || busy}>
            만들기
          </button>
        </>
      }
    >
      <form id="new-project" className="form" onSubmit={submit}>
        <label className="field">
          <span className="field-label">작품 제목</span>
          <input data-autofocus value={title} onChange={(e) => setTitle(e.target.value)} placeholder="예: 달빛 서점의 마지막 손님" maxLength={100} />
        </label>

        <fieldset className="field">
          <legend className="field-label">유형</legend>
          <div className="kind-options">
            {KINDS.map((k) => (
              <label key={k.kind} className={`kind-option${kind === k.kind ? ' on' : ''}`}>
                <input type="radio" name="kind" checked={kind === k.kind} onChange={() => pickKind(k.kind)} />
                <strong>{k.label}</strong>
                <small>{k.hint}</small>
              </label>
            ))}
          </div>
        </fieldset>

        <div className="field">
          <span className="field-label">저장 위치</span>
          <PlacePicker places={places} value={parent} onChange={setParent} />
          {title.trim() && parent && (
            <small className="hint">
              {parent}
              {sep}
              {fileSafe(title) || '새 작품'} 폴더에 저장됩니다
            </small>
          )}
        </div>

        <div className="field">
          <span className="field-label">{noun} 목표 분량</span>
          <div className="row">
            <input
              inputMode="numeric"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              placeholder="없음"
              className="short"
              aria-label={`${noun} 목표 분량`}
            />
            <span>자</span>
            <select value={countSpaces ? 'with' : 'without'} onChange={(e) => setCountSpaces(e.target.value === 'with')} aria-label="공백 포함 여부">
              <option value="with">공백 포함</option>
              <option value="without">공백 제외</option>
            </select>
          </div>
        </div>

        <label className="check">
          <input type="checkbox" checked={firstChapter} onChange={(e) => setFirstChapter(e.target.checked)} />
          1부와 첫 {noun} 만들기
        </label>
      </form>
    </Modal>
  );
}
