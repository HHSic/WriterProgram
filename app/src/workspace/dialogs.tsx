import { useCallback, useEffect, useState } from 'react';
import { api } from '../api';
import type { ExportItem, TextOptions, TrashItem } from '../api/types';
import { Modal } from '../components/Modal';
import { fileSafe, num, timeLabel } from '../lib/format';
import { UNTITLED, docNoun, docNumber } from '../lib/labels';
import { FONT_LABEL, type BodyFont, type Theme } from '../lib/view';
import { NewProjectDialog } from '../screens/NewProjectDialog';
import {
  allManuscript,
  closeDialog,
  refreshOverview,
  saveEverything,
  setView,
  showToast,
  toastError,
  useApp,
  type Dialog,
} from '../store';

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
                {item.section === 'planning' ? '기획' : '원고'} · {num(item.chars)}자 · {timeLabel(item.deletedAt)}에 지움
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
      <p className="hint">휴지통에 넣은 문서는 30일 동안 보관한 뒤 지워집니다.</p>
    </Modal>
  );
}

type Scope = 'current' | 'all';
type Target = 'file' | 'clipboard';

export function ExportDialog() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const kind = ov.project.kind;
  const manuscript = allManuscript(ov);
  const activeIsManuscript = manuscript.some((d) => d.id === activeDocId);

  const [scope, setScope] = useState<Scope>(activeIsManuscript ? 'current' : 'all');
  const [target, setTarget] = useState<Target>('file');
  const [perDoc, setPerDoc] = useState(false);
  const [includeTitles, setIncludeTitles] = useState(true);
  const [blankLine, setBlankLine] = useState(kind === 'webnovel');
  const [symbol, setSymbol] = useState(ov.project.sceneBreak);
  const [busy, setBusy] = useState(false);

  const items: ExportItem[] = manuscript
    .map((doc, i) => ({ doc, n: i + 1 }))
    .filter(({ doc }) => scope === 'all' || doc.id === activeDocId)
    .map(({ doc, n }) => ({
      docId: doc.id,
      heading: `${docNumber(kind, n)}${doc.title ? ` ${doc.title}` : ''}`,
      fileName: `${fileSafe(ov.project.title)}_${String(n).padStart(3, '0')}`,
    }));

  const opts: TextOptions = { includeTitles, blankLineBetween: blankLine, sceneBreak: symbol.trim() || '◆' };

  const run = async () => {
    if (!items.length || busy) return;
    setBusy(true);
    try {
      if (!(await saveEverything())) return;
      if (target === 'clipboard') {
        const text = await api.exportText(ov.root, items, opts);
        await navigator.clipboard.writeText(text);
        closeDialog();
        showToast({ text: `클립보드에 복사함 · ${num([...text].length)}자` });
        return;
      }
      let dest: string | null;
      if (perDoc && items.length > 1) {
        dest = await api.pickFolder('내보낼 폴더');
      } else {
        const name = items.length === 1 ? `${items[0].fileName}.txt` : `${fileSafe(ov.project.title)}.txt`;
        dest = await api.pickSaveFile('텍스트 파일로 내보내기', name);
      }
      if (!dest) return;
      const files = await api.exportTxt(ov.root, items, opts, dest, perDoc && items.length > 1);
      closeDialog();
      showToast({
        text: files.length > 1 ? `내보냄 · 파일 ${files.length}개` : '내보냄',
        action: { label: '폴더 열기', run: () => void api.reveal(files[0]) },
      });
    } catch (e) {
      toastError('내보내지 못함', e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="내보내기"
      onClose={closeDialog}
      width={520}
      footer={
        <>
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="button" className="btn primary" disabled={!items.length || busy} onClick={() => void run()}>
            {target === 'clipboard' ? '복사하기' : '내보내기'}
          </button>
        </>
      }
    >
      <div className="form">
        <fieldset className="field">
          <legend className="field-label">범위</legend>
          <div className="segmented">
            <label className={scope === 'current' ? 'on' : ''}>
              <input type="radio" checked={scope === 'current'} disabled={!activeIsManuscript} onChange={() => setScope('current')} />
              지금 {docNoun(kind)}
            </label>
            <label className={scope === 'all' ? 'on' : ''}>
              <input type="radio" checked={scope === 'all'} onChange={() => setScope('all')} />
              작품 전체 ({manuscript.length}개)
            </label>
          </div>
        </fieldset>

        <fieldset className="field">
          <legend className="field-label">형식</legend>
          <div className="segmented">
            <label className={target === 'file' ? 'on' : ''}>
              <input type="radio" checked={target === 'file'} onChange={() => setTarget('file')} />
              텍스트 파일 (txt)
            </label>
            <label className={target === 'clipboard' ? 'on' : ''}>
              <input type="radio" checked={target === 'clipboard'} onChange={() => setTarget('clipboard')} />
              클립보드
            </label>
          </div>
          <small className="hint">docx와 한글(HWPX) 내보내기는 다음 단계에서 붙습니다.</small>
        </fieldset>

        {target === 'file' && scope === 'all' && (
          <fieldset className="field">
            <legend className="field-label">파일</legend>
            <div className="segmented">
              <label className={!perDoc ? 'on' : ''}>
                <input type="radio" checked={!perDoc} onChange={() => setPerDoc(false)} />
                파일 하나로
              </label>
              <label className={perDoc ? 'on' : ''}>
                <input type="radio" checked={perDoc} onChange={() => setPerDoc(true)} />
                {docNoun(kind)}마다 파일 하나
              </label>
            </div>
          </fieldset>
        )}

        <div className="field">
          <span className="field-label">모양</span>
          <label className="check">
            <input type="checkbox" checked={includeTitles} onChange={(e) => setIncludeTitles(e.target.checked)} />
            {docNoun(kind)} 제목 줄 넣기
          </label>
          <label className="check">
            <input type="checkbox" checked={blankLine} onChange={(e) => setBlankLine(e.target.checked)} />
            문단 사이에 빈 줄 넣기 (연재 플랫폼에 붙여 넣을 때)
          </label>
          <label className="row">
            <span>장면 나눔 표시</span>
            <input className="short" value={symbol} onChange={(e) => setSymbol(e.target.value)} aria-label="장면 나눔 표시" />
          </label>
        </div>
      </div>
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
        <fieldset className="field">
          <legend className="field-label">화면</legend>
          <div className="segmented">
            {THEMES.map((t) => (
              <label key={t.id} className={view.theme === t.id ? 'on' : ''}>
                <input type="radio" checked={view.theme === t.id} onChange={() => setView({ theme: t.id })} />
                {t.label}
              </label>
            ))}
          </div>
        </fieldset>
      </div>
    </Modal>
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
    case 'view':
      return <ViewDialog />;
    case 'newProject':
      return <NewProjectDialog />;
  }
}
