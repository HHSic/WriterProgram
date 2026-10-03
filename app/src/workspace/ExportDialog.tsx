// 내보내기 (S11): 한글 (HWPX), Word (docx), text or clipboard. With "편집자에게
// 보내는 원고로 남기기" (or opened as 편집자에게 보내기) a 한글 or Word file is
// sent through `exchange_send`, which keeps the chapters as sent so the
// corrected file can be compared with them later (docs/corrections.md).
// The clipboard has 붙여넣기 서식: a preset per 연재 플랫폼 for empty lines and
// whether HTML paragraphs go along, with a preview (docs/platforms.md).

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { ExportItem, FileKind, ManuscriptFormat } from '../api/types';
import { Modal } from '../components/Modal';
import { useChanged } from '../lib/useChanged';
import { noteInAppCopy } from '../editor/journal';
import { fileSafe, num } from '../lib/format';
import { loadPaste, previewLines, savePaste, writeClipboard } from '../lib/paste';
import { PLATFORMS, defaultPaste, platformOf, presetOf, type PasteHtml } from '../lib/platforms';
import { UNTITLED, docNoun, docNumber, formatName, withObject } from '../lib/labels';
import { allManuscript, closeDialog, noteManuscriptOut, noteSent, openDialog, saveEverything, showToast, toastError, useApp } from '../store';

type Scope = 'current' | 'all' | 'pick';
type Target = FileKind | 'txt' | 'clipboard';

const TARGETS: { id: Target; label: string }[] = [
  { id: 'hwpx', label: '한글 (HWPX)' },
  { id: 'docx', label: 'Word (docx)' },
  { id: 'txt', label: '텍스트 (txt)' },
  { id: 'clipboard', label: '클립보드' },
];

export function ExportDialog({ toEditor = false, docIds }: { toEditor?: boolean; docIds?: string[] }) {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const catalog = useApp((s) => s.catalog);
  const kind = ov.project.kind;
  const noun = docNoun(kind);
  const manuscript = allManuscript(ov);
  const activeIsManuscript = manuscript.some((d) => d.id === activeDocId);

  const [scope, setScope] = useState<Scope>(docIds?.length ? 'pick' : activeIsManuscript ? 'current' : 'all');
  const [picked, setPicked] = useState<Set<string>>(() => new Set(docIds ?? (activeDocId && activeIsManuscript ? [activeDocId] : [])));
  const [forEditor, setForEditor] = useState(toEditor);
  const [target, setTarget] = useState<Target>(toEditor || kind === 'print' ? 'hwpx' : 'txt');
  const [perDoc, setPerDoc] = useState(false);
  const [includeTitles, setIncludeTitles] = useState(true);
  const [remembered] = useState(() => loadPaste(ov.project.id));
  const [blankLine, setBlankLine] = useState(() => (remembered ?? defaultPaste(ov.project)).blankLine);
  const [pasteMode, setPasteMode] = useState<PasteHtml>(() => (remembered ?? defaultPaste(ov.project)).html);
  const [pastePreset, setPastePreset] = useState(() => remembered?.preset ?? platformOf(ov.project)?.id ?? '');
  const [preview, setPreview] = useState<string | null>(null);
  const [symbol, setSymbol] = useState(ov.project.sceneBreak);
  const [formatChoice, setFormatChoice] = useState('project');
  const [busy, setBusy] = useState(false);
  const dirty = useChanged({ scope, picked, forEditor, target, perDoc, includeTitles, blankLine, symbol, formatChoice, pasteMode });

  const isFile = target === 'hwpx' || target === 'docx';
  const sending = forEditor && isFile;
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
    .filter(({ doc }) => (scope === 'all' ? true : scope === 'pick' ? picked.has(doc.id) : doc.id === activeDocId))
    .map(({ doc, n }) => ({
      docId: doc.id,
      heading: `${docNumber(kind, n)}${doc.title ? ` ${doc.title}` : ''}`,
      fileName: `${fileSafe(ov.project.title)}_${String(n).padStart(3, '0')}`,
    }));
  const sceneBreak = symbol.trim() || '◆';
  const firstDoc = items[0]?.docId;
  const firstHeading = items[0]?.heading;

  // How the first chapter will stand after pasting.
  useEffect(() => {
    if (target !== 'clipboard' || !firstDoc) {
      setPreview(null);
      return;
    }
    let alive = true;
    const timer = window.setTimeout(() => {
      api
        .exportText(ov.root, [{ docId: firstDoc, heading: firstHeading ?? '', fileName: '' }], {
          includeTitles,
          blankLineBetween: blankLine,
          sceneBreak,
        })
        .then(
          (text) => alive && setPreview(text),
          () => alive && setPreview(null),
        );
    }, 150);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [target, ov.root, firstDoc, firstHeading, includeTitles, blankLine, sceneBreak]);

  const pickPreset = (id: string) => {
    setPastePreset(id);
    const preset = presetOf(id);
    if (!preset) return;
    setBlankLine(preset.paste.blankLine);
    setPasteMode(preset.paste.html);
  };
  const preset = presetOf(pastePreset);

  const run = async () => {
    if (!items.length || busy) return;
    setBusy(true);
    try {
      if (!(await saveEverything())) return;
      if (target === 'clipboard') {
        const text = await api.exportText(ov.root, items, { includeTitles, blankLineBetween: blankLine, sceneBreak });
        await writeClipboard(text, { blankLine, html: pasteMode });
        savePaste(ov.project.id, { blankLine, html: pasteMode, preset: pastePreset });
        // Pasting it back into a chapter is not text from outside.
        noteInAppCopy(text);
        closeDialog();
        showToast({ text: `클립보드에 복사함 · ${num([...text].length)}자` });
        return;
      }
      const many = perDoc && items.length > 1;
      const extension = target;
      let dest: string | null;
      if (many) {
        dest = await api.pickFolder(sending ? '편집자에게 보낼 파일을 둘 폴더' : '내보낼 폴더');
      } else {
        const base = items.length === 1 ? items[0].fileName : fileSafe(ov.project.title);
        const labels: Record<string, string> = { hwpx: '한글 파일로', docx: 'Word 파일로', txt: '텍스트 파일로' };
        dest = await api.pickSaveFile(sending ? '편집자에게 보낼 파일' : `${labels[extension]} 내보내기`, `${base}.${extension}`, extension);
      }
      if (!dest) return;
      const reveal = (path: string) => (api.isDesktop ? { label: '폴더 열기', run: () => void api.reveal(path) } : undefined);
      if (sending) {
        const ex = await api.exchangeSend(ov.root, items, { includeTitles, sceneBreak }, formatFor(formatChoice), target, dest, many);
        noteSent();
        noteManuscriptOut();
        closeDialog();
        showToast({
          text: `편집자에게 보낼 파일을 만들었습니다${ex.files.length > 1 ? ` (${ex.files.length}개)` : ''}. 교정본을 받으면 ‘교정본 주고받기’에서 가져오세요.`,
          action: reveal(dest),
        });
        return;
      }
      const files =
        target === 'txt'
          ? await api.exportTxt(ov.root, items, { includeTitles, blankLineBetween: blankLine, sceneBreak }, dest, many)
          : await api.exportFile(ov.root, items, { includeTitles, sceneBreak }, formatFor(formatChoice), target, dest, many);
      noteManuscriptOut();
      closeDialog();
      showToast({
        text: files.length > 1 ? `내보냄 · 파일 ${files.length}개` : '내보냄',
        action: reveal(files[0]),
      });
    } catch (e) {
      toastError(sending ? '보내지 못함' : '내보내지 못함', e);
    } finally {
      setBusy(false);
    }
  };

  const targets = forEditor ? TARGETS.filter((t) => t.id === 'hwpx' || t.id === 'docx') : TARGETS;
  const togglePick = (id: string, on: boolean) =>
    setPicked((old) => {
      const next = new Set(old);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });

  return (
    <Modal
      title={toEditor ? '편집자에게 보내기' : '내보내기'}
      onClose={closeDialog}
      dirty={dirty}
      width={540}
      footer={
        <>
          {sending && (
            <button type="button" className="btn ghost" onClick={() => openDialog({ kind: 'exchanges' })}>
              보낸 원고 보기
            </button>
          )}
          <span className="grow" />
          <button type="button" className="btn" onClick={closeDialog}>
            취소
          </button>
          <button type="button" className="btn primary" disabled={!items.length || busy} onClick={() => void run()}>
            {target === 'clipboard' ? '복사하기' : sending ? '보낼 파일 만들기' : '내보내기'}
          </button>
        </>
      }
    >
      <div className="form">
        {toEditor && (
          <p className="hint">
            고른 {withObject(noun)} 한글이나 Word 파일로 만들고, 보낸 원고를 따로 남겨 둡니다. 편집자가 고친 파일(교정본)을 돌려주면 이 원고와 비교해
            바뀐 곳을 보여 드립니다.
          </p>
        )}
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
            {forEditor && (
              <label className={scope === 'pick' ? 'on' : ''}>
                <input type="radio" checked={scope === 'pick'} onChange={() => setScope('pick')} />
                골라서
              </label>
            )}
          </div>
          {forEditor && scope === 'pick' && (
            <ul className="send-picks" aria-label={`보낼 ${noun}`}>
              {manuscript.map((doc, i) => (
                <li key={doc.id}>
                  <label className="check">
                    <input type="checkbox" checked={picked.has(doc.id)} onChange={(e) => togglePick(doc.id, e.target.checked)} />
                    <span className="send-pick-num">{docNumber(kind, i + 1)}</span>
                    <span className="ellipsis">{doc.title || UNTITLED}</span>
                  </label>
                </li>
              ))}
            </ul>
          )}
        </fieldset>

        <fieldset className="field">
          <legend className="field-label">형식</legend>
          <div className="segmented">
            {targets.map((t) => (
              <label key={t.id} className={target === t.id ? 'on' : ''}>
                <input type="radio" checked={target === t.id} onChange={() => setTarget(t.id)} />
                {t.label}
              </label>
            ))}
          </div>
          {isFile && !toEditor && (
            <label className="check">
              <input type="checkbox" checked={forEditor} onChange={(e) => setForEditor(e.target.checked)} />
              편집자에게 보내는 원고로 남기기 (교정본을 받으면 이 원고와 비교)
            </label>
          )}
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

        {target !== 'clipboard' && items.length > 1 && (
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
          {target === 'txt' && (
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

        {target === 'clipboard' && (
          <div className="field paste-field">
            <span className="field-label">붙여넣기 서식</span>
            <label className="row">
              <span>플랫폼에 맞추기</span>
              <select value={pastePreset} onChange={(e) => pickPreset(e.target.value)} aria-label="플랫폼에 맞추기">
                <option value="">직접 고르기</option>
                {PLATFORMS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={blankLine}
                onChange={(e) => {
                  setBlankLine(e.target.checked);
                  setPastePreset('');
                }}
              />
              문단 사이에 빈 줄 넣기
            </label>
            <div className="segmented" role="radiogroup" aria-label="붙여 넣는 방식">
              <label className={pasteMode === 'none' ? 'on' : ''}>
                <input
                  type="radio"
                  checked={pasteMode === 'none'}
                  onChange={() => {
                    setPasteMode('none');
                    setPastePreset('');
                  }}
                />
                글자만
              </label>
              <label className={pasteMode === 'p' ? 'on' : ''}>
                <input
                  type="radio"
                  checked={pasteMode === 'p'}
                  onChange={() => {
                    setPasteMode('p');
                    setPastePreset('');
                  }}
                />
                문단 나눔도 함께
              </label>
            </div>
            <small className="hint">
              {pasteMode === 'p'
                ? '글자와 함께 문단 나눔을 넘깁니다. 한 문단이 한 줄로, 빈 줄은 빈 문단으로 들어갑니다.'
                : '메모장에 쓴 글처럼 글자만 넘깁니다. 줄을 어떻게 나눌지는 붙여 넣는 곳이 정합니다.'}
              {preset?.pasteNote ? ` ${preset.pasteNote}` : ''}
            </small>
            {preview !== null && (
              <div className="paste-preview" aria-label="붙여 넣은 모양 미리보기">
                {previewLines(preview).map((line, i) =>
                  line.blank ? (
                    <div key={i} className="paste-blank">
                      빈 줄
                    </div>
                  ) : (
                    <p key={i}>{line.text}</p>
                  ),
                )}
              </div>
            )}
            {preview !== null && (
              <small className="hint">
                첫 {noun}의 앞부분입니다. 붙여 넣은 뒤 문단 사이가 두세 줄로 벌어지면 ‘문단 사이에 빈 줄 넣기’를 끄고 다시 복사하세요.
              </small>
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}
