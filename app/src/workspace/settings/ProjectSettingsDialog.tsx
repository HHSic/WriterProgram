// 작품 설정 (S12): basic information, goals and the manuscript format
// (원고 서식) with a live page preview and page estimate.

import { useState } from 'react';
import { api } from '../../api';
import type { Goal, ManuscriptFormat, ProjectKind } from '../../api/types';
import { Modal } from '../../components/Modal';
import { placeNote, usePlaceOf } from '../../components/PlacePicker';
import { KIND_LABEL, docNoun } from '../../lib/labels';
import { closeDialog, openDialog, toastError, updateProject, useApp, type SettingsTab } from '../../store';
import { ProjectDriveField } from '../Drives';
import { FormatEditor } from './FormatEditor';
import { JournalField } from './JournalField';

const TABS: { id: SettingsTab; label: string }[] = [
  { id: 'basic', label: '기본 정보' },
  { id: 'goal', label: '목표' },
  { id: 'format', label: '원고 서식' },
];

const SCENE_SYMBOLS = ['◆', '◇', '*', '* * *', '***', '#', '○'];

export function ProjectSettingsDialog({ tab: initialTab }: { tab?: SettingsTab }) {
  const ov = useApp((s) => s.overview)!;
  const catalog = useApp((s) => s.catalog);
  const project = ov.project;
  const [tab, setTab] = useState<SettingsTab>(initialTab ?? 'basic');
  const [title, setTitle] = useState(project.title);
  const [kind, setKind] = useState<ProjectKind>(project.kind);
  const [penName, setPenName] = useState(project.penName);
  const [sceneBreak, setSceneBreak] = useState(project.sceneBreak);
  const [goal, setGoal] = useState<Goal>(project.goal);
  const [format, setFormat] = useState<ManuscriptFormat>(project.manuscriptFormat);
  const [busy, setBusy] = useState(false);
  const where = usePlaceOf(ov.root);

  const save = async () => {
    if (busy) return;
    setBusy(true);
    const ok = await updateProject({
      title,
      kind,
      penName,
      sceneBreak,
      goal,
      manuscriptFormat: format,
    });
    setBusy(false);
    if (ok) closeDialog();
  };

  return (
    <Modal
      title="작품 설정"
      onClose={closeDialog}
      width={tab === 'format' ? 1040 : 560}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="button" className="btn primary" disabled={busy || !title.trim()} onClick={() => void save()}>
            저장
          </button>
        </>
      }
    >
      <div className="tabs dialog-tabs" role="tablist">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            className={`tab${tab === t.id ? ' on' : ''}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'basic' && (
        <div className="form">
          <label className="field">
            <span className="field-label">작품 제목</span>
            <input value={title} onChange={(e) => setTitle(e.target.value)} maxLength={100} />
          </label>
          <fieldset className="field">
            <legend className="field-label">유형</legend>
            <div className="segmented">
              {(['webnovel', 'print'] as ProjectKind[]).map((k) => (
                <label key={k} className={kind === k ? 'on' : ''}>
                  <input type="radio" checked={kind === k} onChange={() => setKind(k)} />
                  {KIND_LABEL[k]}
                </label>
              ))}
            </div>
            <small className="hint">유형을 바꾸면 {docNoun(kind)} 이름과 상태 목록이 바뀝니다. 원고는 그대로입니다.</small>
          </fieldset>
          <label className="field">
            <span className="field-label">필명</span>
            <input value={penName} onChange={(e) => setPenName(e.target.value)} placeholder="내보낸 파일의 지은이로 들어갑니다" />
          </label>
          <fieldset className="field">
            <legend className="field-label">장면 나눔 표시</legend>
            <div className="segmented">
              {SCENE_SYMBOLS.map((symbol) => (
                <label key={symbol} className={sceneBreak === symbol ? 'on' : ''}>
                  <input type="radio" checked={sceneBreak === symbol} onChange={() => setSceneBreak(symbol)} />
                  {symbol}
                </label>
              ))}
            </div>
            <small className="hint">화면과 내보낸 원고에 이 기호로 보입니다.</small>
          </fieldset>
          <div className="field">
            <span className="field-label">저장 위치</span>
            <div className="row">
              <input readOnly value={ov.root} className="grow" aria-label="저장 위치" />
              {api.isDesktop && (
                <button type="button" className="btn" onClick={() => void api.reveal(ov.root).catch((e) => toastError('폴더를 열지 못함', e))}>
                  폴더 열기
                </button>
              )}
              <button type="button" className="btn" onClick={() => openDialog({ kind: 'move' })}>
                옮기기…
              </button>
            </div>
            {where !== undefined && (
              <small className={`hint storage-note${where ? ' synced' : ''}`}>{placeNote(where)}</small>
            )}
          </div>
          <ProjectDriveField />
          <JournalField root={ov.root} />
        </div>
      )}

      {tab === 'goal' && (
        <div className="form">
          <div className="field">
            <span className="field-label">{docNoun(kind)} 목표 분량</span>
            <div className="row">
              <NumberInput
                value={goal.perDoc}
                onChange={(perDoc) => setGoal({ ...goal, perDoc })}
                label={`${docNoun(kind)} 목표 분량`}
                placeholder="없음"
              />
              <span>자</span>
              <select
                value={goal.countSpaces ? 'with' : 'without'}
                onChange={(e) => setGoal({ ...goal, countSpaces: e.target.value === 'with' })}
                aria-label="공백 포함 여부"
              >
                <option value="with">공백 포함</option>
                <option value="without">공백 제외</option>
              </select>
            </div>
            <small className="hint">{docNoun(kind)}마다 따로 정할 수도 있습니다 (왼쪽 목록에서 오른쪽 클릭).</small>
          </div>
          <div className="field">
            <span className="field-label">하루 목표</span>
            <div className="row">
              <NumberInput value={goal.daily} onChange={(daily) => setGoal({ ...goal, daily })} label="하루 목표" placeholder="없음" />
              <span>자</span>
            </div>
          </div>
        </div>
      )}

      {tab === 'format' && (
        <FormatEditor format={format} onChange={setFormat} catalog={catalog} root={ov.root} title={title} penName={penName} />
      )}
    </Modal>
  );
}

function NumberInput({
  value,
  onChange,
  label,
  placeholder,
}: {
  value: number | null;
  onChange: (v: number | null) => void;
  label: string;
  placeholder?: string;
}) {
  return (
    <input
      className="short"
      inputMode="numeric"
      aria-label={label}
      placeholder={placeholder}
      value={value ?? ''}
      onChange={(e) => {
        const n = Number.parseInt(e.target.value.replace(/[^0-9]/g, ''), 10);
        onChange(Number.isFinite(n) && n > 0 ? n : null);
      }}
    />
  );
}
