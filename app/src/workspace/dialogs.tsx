import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import type { TrashItem } from '../api/types';
import { Modal } from '../components/Modal';
import { num, timeLabel } from '../lib/format';
import { UNTITLED } from '../lib/labels';
import { ACCENTS, PALETTES, previewColors } from '../lib/colors';
import { FONT_LABEL, type BodyFont, type Theme, type ViewSettings } from '../lib/view';
import { NewProjectDialog } from '../screens/NewProjectDialog';
import { CompareDialog } from './Compare';
import { CopiesDialog } from './Copies';
import { DriveImportDialog, DrivesDialog } from './Drives';
import { ExportDialog } from './ExportDialog';
import { ImportDialog } from './ImportDialog';
import { MoveDialog } from './MoveProject';
import { ProjectSettingsDialog } from './ProjectSettings';
import { SymbolsDialog } from './Symbols';
import { closeDialog, loadNotes, refreshOverview, setView, toastError, useApp, type Dialog } from '../store';

export function PromptDialog({ dialog }: { dialog: Extract<Dialog, { kind: 'prompt' }> }) {
  const [value, setValue] = useState(dialog.value);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (busy) return;
    setBusy(true);
    await dialog.onSubmit(value);
    closeDialog();
  };
  return (
    <Modal
      title={dialog.title}
      onClose={closeDialog}
      width={420}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="submit" form="prompt-form" className="btn primary" disabled={busy}>
            {dialog.confirm}
          </button>
        </>
      }
    >
      <form
        id="prompt-form"
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <label className="field">
          <span className="field-label">{dialog.label}</span>
          <input
            data-autofocus
            value={value}
            inputMode={dialog.inputMode}
            onChange={(e) => setValue(e.target.value)}
            onFocus={(e) => e.target.select()}
          />
        </label>
      </form>
    </Modal>
  );
}

export function ConfirmDialog({ dialog }: { dialog: Extract<Dialog, { kind: 'confirm' }> }) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title={dialog.title}
      onClose={closeDialog}
      width={440}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button
            type="button"
            className={`btn primary${dialog.danger ? ' danger' : ''}`}
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              await dialog.onConfirm();
              closeDialog();
            }}
          >
            {dialog.confirm}
          </button>
        </>
      }
    >
      <p className="dialog-text">{dialog.message}</p>
    </Modal>
  );
}

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

const THEMES: { id: Theme; label: string }[] = [
  { id: 'system', label: '시스템 따라' },
  { id: 'light', label: '밝게' },
  { id: 'dark', label: '어둡게' },
];

export function ViewDialog() {
  const view = useApp((s) => s.view);
  return (
    <Modal title="보기 설정" onClose={closeDialog} width={500}>
      <div className="form">
        <p className="hint">이 기기의 작성 화면에만 적용됩니다. 원고 파일과 내보내기 모양은 바뀌지 않습니다.</p>
        <label className="field">
          <span className="field-label">본문 글꼴</span>
          <select value={view.font} onChange={(e) => setView({ font: e.target.value as BodyFont })}>
            {(Object.keys(FONT_LABEL) as BodyFont[]).map((f) => (
              <option key={f} value={f}>
                {FONT_LABEL[f]}
              </option>
            ))}
          </select>
        </label>
        <Slider label="글자 크기" value={view.fontSize} min={14} max={26} step={1} unit="px" onChange={(fontSize) => setView({ fontSize })} />
        <Slider
          label="줄 간격"
          value={Math.round(view.lineHeight * 100)}
          min={140}
          max={240}
          step={5}
          unit="%"
          onChange={(v) => setView({ lineHeight: v / 100 })}
        />
        <Slider
          label="자간"
          value={view.letterSpacing}
          min={-10}
          max={20}
          step={1}
          unit="%"
          onChange={(letterSpacing) => setView({ letterSpacing })}
        />
        <Slider
          label="문단 사이"
          value={view.paragraphGap}
          min={0}
          max={1.5}
          step={0.25}
          unit="줄"
          onChange={(paragraphGap) => setView({ paragraphGap })}
        />
        <Slider label="첫 줄 들여쓰기" value={view.indent} min={0} max={3} step={1} unit="자" onChange={(indent) => setView({ indent })} />
        <Slider label="본문 폭" value={view.width} min={440} max={840} step={20} unit="px" onChange={(width) => setView({ width })} />
        <label className="check">
          <input type="checkbox" checked={view.showMarks} onChange={(e) => setView({ showMarks: e.target.checked })} />
          빈칸·문단 부호 보이기 (Ctrl+Shift+8)
        </label>
        <label className="field">
          <span className="field-label">편집 도구줄 (본문 아래 버튼 줄)</span>
          <select value={view.toolbar} onChange={(e) => setView({ toolbar: e.target.value as ViewSettings['toolbar'] })}>
            <option value="auto">터치 화면에서만</option>
            <option value="always">늘 보이기</option>
            <option value="never">숨기기</option>
          </select>
        </label>
        <fieldset className="field">
          <legend className="field-label">화면 밝기</legend>
          <div className="segmented">
            {THEMES.map((t) => (
              <label key={t.id} className={view.theme === t.id ? 'on' : ''}>
                <input type="radio" checked={view.theme === t.id} onChange={() => setView({ theme: t.id })} />
                {t.label}
              </label>
            ))}
          </div>
        </fieldset>
        <ColorFields view={view} />
      </div>
    </Modal>
  );
}

/** 화면 색 and 강조 색, drawn as small samples in the current brightness. */
function ColorFields({ view }: { view: ViewSettings }) {
  const dark =
    view.theme === 'dark' || (view.theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  const choice = { palette: view.palette, accent: view.accent, customColor: view.customColor };
  const paletteAccent = PALETTES.find((p) => p.id === view.palette)?.accent ?? 'vermilion';
  return (
    <>
      <fieldset className="field">
        <legend className="field-label">화면 색</legend>
        <div className="swatches" role="radiogroup" aria-label="화면 색">
          {PALETTES.map((p) => {
            const c = previewColors({ ...choice, palette: p.id, accent: 'auto' }, dark);
            return (
              <button
                key={p.id}
                type="button"
                role="radio"
                aria-checked={view.palette === p.id}
                className={`swatch${view.palette === p.id ? ' on' : ''}`}
                onClick={() => setView({ palette: p.id })}
              >
                <span className="swatch-chip" style={{ background: `linear-gradient(135deg, ${c.bg} 50%, ${c.surface} 50%)`, borderColor: c.border }}>
                  <span className="swatch-dot" style={{ background: c.accent }} />
                </span>
                <span className="swatch-name">{p.name}</span>
              </button>
            );
          })}
          <label
            className={`swatch${view.palette === 'custom' ? ' on' : ''}`}
            title="색을 고르면 그 색 기운으로 배경과 원고 종이를 맞춥니다"
            onClick={() => setView({ palette: 'custom' })}
          >
            <input
              type="color"
              className="swatch-input"
              value={view.customColor}
              aria-label="화면 색 직접 고르기"
              onChange={(e) => setView({ palette: 'custom', customColor: e.target.value })}
            />
            {(() => {
              const c = previewColors({ ...choice, palette: 'custom', accent: 'auto' }, dark);
              return (
                <span className="swatch-chip custom" style={{ background: `linear-gradient(135deg, ${c.bg} 50%, ${c.surface} 50%)`, borderColor: c.border }}>
                  <span className="swatch-dot" style={{ background: view.customColor }} />
                </span>
              );
            })()}
            <span className="swatch-name">직접 고르기</span>
          </label>
        </div>
      </fieldset>
      <fieldset className="field">
        <legend className="field-label">강조 색</legend>
        <div className="swatches" role="radiogroup" aria-label="강조 색">
          {[{ id: 'auto' as const, name: '색에 맞춤' }, ...ACCENTS].map((a) => {
            const color = previewColors({ ...choice, accent: a.id === 'auto' ? paletteAccent : a.id }, dark).accent;
            return (
              <button
                key={a.id}
                type="button"
                role="radio"
                aria-checked={view.accent === a.id}
                className={`swatch${view.accent === a.id ? ' on' : ''}`}
                onClick={() => setView({ accent: a.id })}
              >
                <span className={`swatch-chip accent${a.id === 'auto' ? ' auto' : ''}`} style={{ background: color }} />
                <span className="swatch-name">{a.name}</span>
              </button>
            );
          })}
        </div>
        <span className="hint">버튼, 진행 막대, 지금 보는 탭 표시에 쓰입니다.</span>
      </fieldset>
    </>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="field slider">
      <span className="field-label">
        {label}
        <output>
          {value}
          {unit}
        </output>
      </span>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  );
}

export function DialogHost() {
  const dialog = useApp((s) => s.dialog);
  if (!dialog) return null;
  switch (dialog.kind) {
    case 'prompt':
      return <PromptDialog key={dialog.title} dialog={dialog} />;
    case 'confirm':
      return <ConfirmDialog dialog={dialog} />;
    case 'trash':
      return <TrashDialog />;
    case 'export':
      return <ExportDialog />;
    case 'import':
      return <ImportDialog partId={dialog.partId} />;
    case 'view':
      return <ViewDialog />;
    case 'symbols':
      return <SymbolsDialog />;
    case 'newProject':
      return <NewProjectDialog />;
    case 'project':
      return <ProjectSettingsDialog tab={dialog.tab} />;
    case 'compare':
      return <CompareDialog key={`${dialog.docId}/${dialog.copy?.file ?? ''}`} docId={dialog.docId} copy={dialog.copy} />;
    case 'copies':
      return <CopiesDialog />;
    case 'move':
      return <MoveDialog />;
    case 'drives':
      return <DrivesDialog />;
    case 'driveImport':
      return <DriveImportDialog />;
  }
}
