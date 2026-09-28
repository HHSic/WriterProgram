// 내보내기 (S11): 한글 (HWPX), Word (docx), text or clipboard.

import { useState } from 'react';
import { api } from '../api';
import type { ExportItem, FileKind, ManuscriptFormat } from '../api/types';
import { Modal } from '../components/Modal';
import { fileSafe, num } from '../lib/format';
import { docNoun, docNumber, formatName } from '../lib/labels';
import { allManuscript, closeDialog, saveEverything, showToast, toastError, useApp } from '../store';

type Scope = 'current' | 'all';
type Target = FileKind | 'txt' | 'clipboard';

const TARGETS: { id: Target; label: string }[] = [
  { id: 'hwpx', label: '한글 (HWPX)' },
  { id: 'docx', label: 'Word (docx)' },
  { id: 'txt', label: '텍스트 (txt)' },
  { id: 'clipboard', label: '클립보드' },
];

export function ExportDialog() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const catalog = useApp((s) => s.catalog);
  const kind = ov.project.kind;
  const noun = docNoun(kind);
  const manuscript = allManuscript(ov);
  const activeIsManuscript = manuscript.some((d) => d.id === activeDocId);

  const [scope, setScope] = useState<Scope>(activeIsManuscript ? 'current' : 'all');
  const [target, setTarget] = useState<Target>(kind === 'print' ? 'hwpx' : 'txt');
  const [perDoc, setPerDoc] = useState(false);
  const [includeTitles, setIncludeTitles] = useState(true);
  const [blankLine, setBlankLine] = useState(kind === 'webnovel');
  const [symbol, setSymbol] = useState(ov.project.sceneBreak);
  const [formatChoice, setFormatChoice] = useState('project');
  const [busy, setBusy] = useState(false);

  const isFile = target === 'hwpx' || target === 'docx';
  const formatFor = (choice: string): ManuscriptFormat => {
    const [source, key] = choice.split(':');
    if (source === 'builtin') {
      const found = catalog?.builtin.find((b) => b.id === key);
      if (found) return found.format;
    }
    if (source === 'user') {
      const found = catalog?.user.find((u) => u.name === key);
      if (found) return found.format;
    }
    return ov.project.manuscriptFormat;
  };

  const items: ExportItem[] = manuscript
    .map((doc, i) => ({ doc, n: i + 1 }))
    .filter(({ doc }) => scope === 'all' || doc.id === activeDocId)
    .map(({ doc, n }) => ({
      docId: doc.id,
      heading: `${docNumber(kind, n)}${doc.title ? ` ${doc.title}` : ''}`,
      fileName: `${fileSafe(ov.project.title)}_${String(n).padStart(3, '0')}`,
    }));
  const sceneBreak = symbol.trim() || '◆';

  const run = async () => {
    if (!items.length || busy) return;
    setBusy(true);
    try {
      if (!(await saveEverything())) return;
      if (target === 'clipboard') {
        const text = await api.exportText(ov.root, items, { includeTitles, blankLineBetween: blankLine, sceneBreak });
        await navigator.clipboard.writeText(text);
        closeDialog();
        showToast({ text: `클립보드에 복사함 · ${num([...text].length)}자` });
        return;
      }
      const many = perDoc && items.length > 1;
      const extension = target;
      let dest: string | null;
      if (many) {
        dest = await api.pickFolder('내보낼 폴더');
      } else {
        const base = items.length === 1 ? items[0].fileName : fileSafe(ov.project.title);
        const labels: Record<string, string> = { hwpx: '한글 파일로', docx: 'Word 파일로', txt: '텍스트 파일로' };
        dest = await api.pickSaveFile(`${labels[extension]} 내보내기`, `${base}.${extension}`, extension);
      }
      if (!dest) return;
      const files =
        target === 'txt'
          ? await api.exportTxt(ov.root, items, { includeTitles, blankLineBetween: blankLine, sceneBreak }, dest, many)
          : await api.exportFile(ov.root, items, { includeTitles, sceneBreak }, formatFor(formatChoice), target, dest, many);
      closeDialog();
      showToast({
        text: files.length > 1 ? `내보냄 · 파일 ${files.length}개` : '내보냄',
        action: api.isDesktop ? { label: '폴더 열기', run: () => void api.reveal(files[0]) } : undefined,
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
      width={540}
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
              지금 {noun}
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
            {TARGETS.map((t) => (
              <label key={t.id} className={target === t.id ? 'on' : ''}>
                <input type="radio" checked={target === t.id} onChange={() => setTarget(t.id)} />
                {t.label}
              </label>
            ))}
          </div>
        </fieldset>

        {isFile && (
          <label className="field">
            <span className="field-label">원고 서식</span>
            <select value={formatChoice} onChange={(e) => setFormatChoice(e.target.value)}>
              <option value="project">작품 서식 · {formatName(ov.project.manuscriptFormat, catalog)}</option>
              <optgroup label="이번만 다른 서식">
                {catalog?.builtin.map((b) => (
                  <option key={b.id} value={`builtin:${b.id}`}>
                    {b.name}
                  </option>
                ))}
                {catalog?.user.map((u) => (
                  <option key={u.name} value={`user:${u.name}`}>
                    {u.name}
                  </option>
                ))}
              </optgroup>
            </select>
            <small className="hint">용지, 여백, 글꼴, 줄 간격, 자간, 들여쓰기, 쪽 번호가 이 서식대로 들어갑니다.</small>
          </label>
        )}

        {target !== 'clipboard' && scope === 'all' && (
          <fieldset className="field">
            <legend className="field-label">파일</legend>
            <div className="segmented">
              <label className={!perDoc ? 'on' : ''}>
                <input type="radio" checked={!perDoc} onChange={() => setPerDoc(false)} />
                파일 하나로
              </label>
              <label className={perDoc ? 'on' : ''}>
                <input type="radio" checked={perDoc} onChange={() => setPerDoc(true)} />
                {noun}마다 파일 하나
              </label>
            </div>
          </fieldset>
        )}

        <div className="field">
          <span className="field-label">모양</span>
          <label className="check">
            <input type="checkbox" checked={includeTitles} onChange={(e) => setIncludeTitles(e.target.checked)} />
            {noun} 제목 줄 넣기
          </label>
          {!isFile && (
            <label className="check">
              <input type="checkbox" checked={blankLine} onChange={(e) => setBlankLine(e.target.checked)} />
              문단 사이에 빈 줄 넣기 (연재 플랫폼에 붙여 넣을 때)
            </label>
          )}
          <label className="row">
            <span>장면 나눔 표시</span>
            <input className="short" value={symbol} onChange={(e) => setSymbol(e.target.value)} aria-label="장면 나눔 표시" />
          </label>
        </div>
      </div>
    </Modal>
  );
}
