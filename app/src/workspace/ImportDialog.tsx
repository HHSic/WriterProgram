// 가져오기 (S3): txt, md, docx and hwpx files become chapters, in three steps:
// pick files, see how they are cut into chapters, confirm where they go.
// Tables, pictures and footnotes are never imported; the last step says so
// and asks whether to leave them out or cancel.
//
// From the start screen (`newProject`) there is no project yet: the last step
// asks for a title, kind and place like 새 작품, and the chapters go into the
// first part of the project made then.

import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import type { ImportChapter, ImportOptions, ImportPreview, LineMode, ProjectKind, SkipTotals, SplitRule } from '../api/types';
import { Icon } from '../components/Icon';
import { Modal } from '../components/Modal';
import { num } from '../lib/format';
import { docNoun, withSubject } from '../lib/labels';
import { KindOptions, PlaceField, useNewPlace } from '../screens/NewProjectDialog';
import {
  closeDialog,
  enterProject,
  loadNotes,
  newDocPartId,
  refreshOverview,
  saveEverything,
  selectDoc,
  showToast,
  toastError,
  useApp,
} from '../store';

type Step = 'files' | 'preview' | 'confirm';

const STEPS: { id: Step; label: string }[] = [
  { id: 'files', label: '파일' },
  { id: 'preview', label: '나누기' },
  { id: 'confirm', label: '확인' },
];

const RULES: { id: SplitRule; label: string }[] = [
  { id: 'auto', label: '자동으로 찾기' },
  { id: 'heading', label: '제목 서식 (Word 제목, md의 # 줄)' },
  { id: 'episode', label: '“제3화”, “3화”, “3회” 줄' },
  { id: 'chapter', label: '“제3장”, “3장”, “Chapter 3” 줄' },
  { id: 'number', label: '“#3” 줄' },
  { id: 'regex', label: '직접 쓴 패턴' },
  { id: 'file', label: '나누지 않기 (파일 하나가 하나)' },
];

const LINE_MODES: { id: LineMode; label: string }[] = [
  { id: 'auto', label: '자동' },
  { id: 'line', label: '줄마다 문단' },
  { id: 'blank', label: '빈 줄로 문단 나누기' },
];

const ENCODINGS: { id: string; label: string }[] = [
  { id: '', label: '자동' },
  { id: 'utf-8', label: 'UTF-8' },
  { id: 'euc-kr', label: 'EUC-KR (옛 한글 텍스트)' },
];

interface Row {
  index: number;
  on: boolean;
  title: string;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** "표 1개 · 그림 2개". */
export function skipText(s: SkipTotals): string {
  return [
    s.tables > 0 && `표 ${s.tables}개`,
    s.images > 0 && `그림 ${s.images}개`,
    s.footnotes > 0 && `각주 ${s.footnotes}개`,
  ]
    .filter(Boolean)
    .join(' · ');
}

function hasSkips(s: SkipTotals): boolean {
  return s.tables + s.images + s.footnotes > 0;
}

/**
 * A chapter this long makes typing slow (docs/feature-gap-report.md §3.3:
 * 1.4 million characters took seconds a keystroke), so the preview suggests
 * cutting it.
 */
export const BIG_CHAPTER = 50_000;

/** A file name without its folder and extension, as a first title. */
export function titleFromPath(path: string): string {
  return fileName(path).replace(/\.[^.]+$/, '').trim();
}

export function ImportDialog({ partId, newProject = false }: { partId?: string; newProject?: boolean }) {
  const ov = useApp((s) => s.overview);
  const [kind, setKind] = useState<ProjectKind>(ov?.project.kind ?? 'webnovel');
  const [title, setTitle] = useState('');
  const place = useNewPlace(newProject);
  const noun = docNoun(newProject || !ov ? kind : ov.project.kind);
  const [step, setStep] = useState<Step>('files');
  const [paths, setPaths] = useState<string[]>([]);
  const [opts, setOpts] = useState<ImportOptions>({ rule: 'auto', pattern: '', lineMode: 'auto', encoding: null });
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [rows, setRows] = useState<Row[]>([]);
  const [loading, setLoading] = useState(false);
  const [target, setTarget] = useState(() => partId ?? newDocPartId() ?? ov?.parts[ov.parts.length - 1]?.id ?? '');
  const [leaveNotes, setLeaveNotes] = useState(true);
  const [followPage, setFollowPage] = useState(false);
  const [busy, setBusy] = useState(false);
  const asked = useRef(0);

  const needPattern = opts.rule === 'regex' && !opts.pattern.trim();
  const textFiles = paths.some((p) => /\.(txt|md|markdown)$/i.test(p));

  // The preview follows every change of files and options.
  useEffect(() => {
    if (step === 'files' || !paths.length || needPattern) return;
    const ticket = ++asked.current;
    setLoading(true);
    const timer = window.setTimeout(() => {
      api.importPreview(paths, opts).then(
        (result) => {
          if (ticket !== asked.current) return;
          setPreview(result);
          setRows(result.chapters.map((c) => ({ index: c.index, on: true, title: c.title })));
          setLoading(false);
        },
        (e) => {
          if (ticket !== asked.current) return;
          setLoading(false);
          toastError('파일을 읽지 못함', e);
        },
      );
    }, 200);
    return () => window.clearTimeout(timer);
  }, [step, paths, opts, needPattern]);

  const pick = async () => {
    try {
      const picked = await api.pickFiles('가져올 원고 파일');
      if (!picked.length) return;
      setPaths((old) => [...old, ...picked.filter((p) => !old.includes(p))]);
      if (newProject && !title.trim()) setTitle(titleFromPath(picked[0]));
    } catch (e) {
      toastError('파일을 고르지 못함', e);
    }
  };

  const chosen: ImportChapter[] = preview ? rows.filter((r) => r.on).map((r) => preview.chapters[r.index]) : [];
  const skipped: SkipTotals = chosen.reduce(
    (t, c) => ({
      tables: t.tables + c.skipped.tables,
      images: t.images + c.skipped.images,
      footnotes: t.footnotes + c.skipped.footnotes,
    }),
    { tables: 0, images: 0, footnotes: 0 },
  );
  const totalChars = chosen.reduce((n, c) => n + c.chars, 0);
  const leaving = hasSkips(skipped);
  const readable = preview?.files.filter((f) => !f.error).length ?? 0;
  // A 한글 file's paper, margins, 머리말, 꼬리말 and page numbers.
  const paged = preview?.files.find((f) => !f.error && f.page) ?? null;

  const big = chosen.filter((c) => c.chars > BIG_CHAPTER);
  const biggest = big.reduce<ImportChapter | null>((b, c) => (!b || c.chars > b.chars ? c : b), null);
  const bigTitle = biggest ? rows.find((r) => r.index === biggest.index)?.title.trim() || '제목 없음' : '';
  const particle = withSubject(bigTitle).slice(bigTitle.length);

  const commitSpec = (partId: string | null) => ({
    partId,
    after: null,
    picks: rows.filter((r) => r.on).map((r) => ({ index: r.index, title: r.title.trim() })),
    leaveNotes: leaving && leaveNotes,
    pageSetup: !!paged && followPage,
  });

  const done = (count: number, notes: number, format: boolean) =>
    showToast({
      text: `${noun} ${count}개를 가져왔습니다${notes > 0 ? ` · 빠진 자리에 메모 ${notes}개` : ''}${format ? ' · 원고 서식도 맞췄습니다' : ''}`,
    });

  // 시작 화면: make the project, then bring the chapters into its first part.
  const runNew = async () => {
    const made = await api.projectCreate({
      parent: place.parent,
      title: title.trim(),
      kind,
      perDocGoal: kind === 'webnovel' ? 5000 : null,
      countSpaces: true,
      firstChapter: false,
    });
    try {
      const result = await api.importCommit(made.root, paths, opts, commitSpec(made.parts[0]?.id ?? null));
      enterProject(await api.projectOverview(made.root));
      closeDialog();
      done(result.docs.length, result.notes, result.format);
      if (result.docs[0]) await selectDoc(result.docs[0]);
    } catch (e) {
      // The project is there, only empty: open it so nothing is left hidden.
      enterProject(made);
      closeDialog();
      toastError(`작품은 만들었지만 원고를 가져오지 못했습니다. ‘원고 가져오기’로 다시 해 보세요`, e);
    }
  };

  const run = async () => {
    if (busy || !preview) return;
    setBusy(true);
    try {
      if (newProject) {
        await runNew();
        return;
      }
      if (!ov || !(await saveEverything())) return;
      const result = await api.importCommit(ov.root, paths, opts, commitSpec(target || null));
      await refreshOverview();
      if (result.notes > 0) await loadNotes();
      closeDialog();
      done(result.docs.length, result.notes, result.format);
      if (result.docs[0]) await selectDoc(result.docs[0]);
    } catch (e) {
      toastError(newProject ? '작품을 만들지 못함' : '가져오지 못함', e);
    } finally {
      setBusy(false);
    }
  };

  const footer = (
    <>
      {step !== 'files' && (
        <button type="button" className="btn ghost import-back" onClick={() => setStep(step === 'confirm' ? 'preview' : 'files')}>
          <Icon name="back" size={14} />
          이전
        </button>
      )}
      <button type="button" className="btn" onClick={closeDialog}>
        취소
      </button>
      {step === 'files' && (
        <button type="button" className="btn primary" disabled={!paths.length} onClick={() => setStep('preview')}>
          다음
        </button>
      )}
      {step === 'preview' && (
        <button type="button" className="btn primary" disabled={loading || !chosen.length} onClick={() => setStep('confirm')}>
          다음
        </button>
      )}
      {step === 'confirm' && (
        <button
          type="button"
          className="btn primary"
          disabled={busy || !chosen.length || (newProject && (!title.trim() || !place.parent))}
          onClick={() => void run()}
        >
          {newProject ? (leaving ? '제외하고 새 작품 만들기' : '새 작품 만들기') : leaving ? '제외하고 가져오기' : '가져오기'}
        </button>
      )}
    </>
  );

  return (
    <Modal
      title={newProject ? '기존 원고 가져오기' : '원고 가져오기'}
      onClose={closeDialog}
      width={640}
      footer={footer}
      dirty={paths.length > 0}
    >
      <ol className="steps" aria-label="진행 단계">
        {STEPS.map((s, i) => (
          <li key={s.id} className={s.id === step ? 'on' : STEPS.findIndex((x) => x.id === step) > i ? 'done' : ''} aria-current={s.id === step ? 'step' : undefined}>
            <span className="steps-no">{i + 1}</span>
            {s.label}
          </li>
        ))}
      </ol>

      {step === 'files' && (
        <div className="form">
          <button type="button" className="import-drop" onClick={() => void pick()}>
            <Icon name="plus" size={18} />
            <strong>파일 고르기</strong>
            <span>txt · md · docx · 한글(hwpx), 여러 개를 한꺼번에 고를 수 있습니다</span>
          </button>
          {paths.length > 0 && (
            <ul className="import-files">
              {paths.map((p) => (
                <li key={p}>
                  <Icon name="doc" size={15} />
                  <span className="import-file-name ellipsis" title={p}>
                    {fileName(p)}
                  </span>
                  <button type="button" className="icon-btn" aria-label={`${fileName(p)} 빼기`} onClick={() => setPaths((old) => old.filter((x) => x !== p))}>
                    <Icon name="close" size={14} />
                  </button>
                </li>
              ))}
            </ul>
          )}
          {newProject && (
            <p className="hint">한글이나 Word, 메모장에서 쓰던 원고로 새 작품을 만듭니다. 원래 파일은 그대로 둡니다.</p>
          )}
          <p className="hint">
            표, 그림, 각주는 가져오지 않습니다. 옛 한글 파일(.hwp)은 한글에서 한글 문서(.hwpx)로 저장한 뒤 골라 주세요.
          </p>
        </div>
      )}

      {step === 'preview' && (
        <div className="form">
          <div className="import-options">
            <label className="field">
              <span className="field-label">{noun} 나누는 방법</span>
              <select value={opts.rule} onChange={(e) => setOpts({ ...opts, rule: e.target.value as SplitRule })}>
                {RULES.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.label}
                  </option>
                ))}
              </select>
            </label>
            {textFiles && (
              <label className="field">
                <span className="field-label">줄바꿈</span>
                <select value={opts.lineMode} onChange={(e) => setOpts({ ...opts, lineMode: e.target.value as LineMode })}>
                  {LINE_MODES.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {textFiles && (
              <label className="field">
                <span className="field-label">글자 인코딩</span>
                <select value={opts.encoding ?? ''} onChange={(e) => setOpts({ ...opts, encoding: e.target.value || null })}>
                  {ENCODINGS.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          {opts.rule === 'regex' && (
            <label className="field">
              <span className="field-label">패턴 (정규식)</span>
              <input
                value={opts.pattern}
                placeholder="예: ^제\s*\d+\s*편"
                spellCheck={false}
                onChange={(e) => setOpts({ ...opts, pattern: e.target.value })}
              />
              <small className="hint">이 패턴에 맞는 줄이 {noun}의 제목이 됩니다. 괄호 ( )로 묶은 부분이 있으면 그 부분만 제목으로 씁니다.</small>
            </label>
          )}

          {needPattern ? (
            <p className="empty-note">패턴을 쓰면 {noun} 목록이 나옵니다.</p>
          ) : !preview ? (
            <p className="empty-note">파일을 읽는 중…</p>
          ) : (
            <>
              <ul className="import-files import-results" aria-busy={loading}>
                {preview.files.map((f, i) => (
                  <li key={`${f.name}${i}`} className={f.error ? 'bad' : ''}>
                    <Icon name="doc" size={15} />
                    <span className="import-file-name ellipsis" title={paths[i]}>
                      {f.name}
                    </span>
                    <span className="import-file-note">
                      {f.error ??
                        `${noun} ${f.chapters}개 · ${f.rule}${f.encoding && f.kind !== 'docx' && f.kind !== 'hwpx' ? ` · ${f.encoding.toUpperCase()}` : ''}`}
                    </span>
                  </li>
                ))}
              </ul>

              {biggest && !loading && (
                <div className="import-warn import-big" role="status">
                  <p>
                    {opts.rule === 'auto' ? (
                      <>
                        ‘{bigTitle}’{particle} 공백 포함 {num(biggest.chars)}자입니다. 자동으로 찾을 제목 줄이 없어 더 나누지
                        못했습니다. {noun} 하나가 이렇게 길면 쓰는 동안 화면이 느려질 수 있으니, 제목 줄의 모양을 패턴으로 알려 주면 나눠 드립니다.
                      </>
                    ) : (
                      <>
                        ‘{bigTitle}’{particle} 공백 포함 {num(biggest.chars)}자로 아주 깁니다. {noun} 하나가 이렇게 길면 쓰는 동안 화면이
                        느려질 수 있습니다. “3화”, “제3장” 같은 제목 줄에서 나누는 것을 권합니다.
                      </>
                    )}
                    {big.length > 1 && ` (아주 긴 ${noun} ${big.length}개)`}
                  </p>
                  <div className="row">
                    {opts.rule === 'auto' ? (
                      <button type="button" className="btn" onClick={() => setOpts({ ...opts, rule: 'regex' })}>
                        패턴 쓰기
                      </button>
                    ) : (
                      <button type="button" className="btn" onClick={() => setOpts({ ...opts, rule: 'auto' })}>
                        자동으로 나누기
                      </button>
                    )}
                  </div>
                </div>
              )}

              {readable === 0 ? (
                <p className="empty-note">읽을 수 있는 파일이 없습니다. 이전으로 돌아가 다른 파일을 골라 주세요.</p>
              ) : (
                <div className={`import-list${loading ? ' loading' : ''}`}>
                  <div className="import-list-head">
                    <label className="check">
                      <input
                        type="checkbox"
                        checked={rows.length > 0 && rows.every((r) => r.on)}
                        onChange={(e) => setRows(rows.map((r) => ({ ...r, on: e.target.checked })))}
                        aria-label={`${noun} 모두 고르기`}
                      />
                      <span>
                        {noun} {chosen.length}/{rows.length}개 · 공백 포함 {num(totalChars)}자
                      </span>
                    </label>
                  </div>
                  <ul>
                    {rows.map((r, n) => {
                      const c = preview.chapters[r.index];
                      return (
                        <li key={r.index} className={r.on ? '' : 'off'}>
                          <input
                            type="checkbox"
                            checked={r.on}
                            aria-label={`${n + 1}번째 ${noun} 가져오기`}
                            onChange={(e) => setRows(rows.map((x) => (x.index === r.index ? { ...x, on: e.target.checked } : x)))}
                          />
                          <span className="import-no">{n + 1}</span>
                          <div className="import-chapter">
                            <input
                              className="cell-input"
                              value={r.title}
                              placeholder="제목 없음"
                              maxLength={200}
                              aria-label={`${n + 1}번째 ${noun} 제목`}
                              onChange={(e) => setRows(rows.map((x) => (x.index === r.index ? { ...x, title: e.target.value } : x)))}
                            />
                            <span className="import-snippet ellipsis">{c.snippet || '(본문 없음)'}</span>
                          </div>
                          <span className="import-meta">
                            {num(c.chars)}자
                            {hasSkips(c.skipped) && <span className="chip status-revise">{skipText(c.skipped)} 빠짐</span>}
                          </span>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {step === 'confirm' && (
        <div className="form">
          {newProject || !ov ? (
            <>
              <label className="field">
                <span className="field-label">작품 제목</span>
                <input data-autofocus value={title} onChange={(e) => setTitle(e.target.value)} placeholder="예: 달빛 서점의 마지막 손님" maxLength={100} />
              </label>
              <KindOptions kind={kind} onPick={setKind} />
              <PlaceField places={place.places} parent={place.parent} onParent={place.setParent} title={title} />
              <p className="dialog-text">
                {noun} <strong>{chosen.length}개</strong> · 공백 포함 <strong>{num(totalChars)}자</strong>로 새 작품을 만듭니다. 목표 분량과 원고 서식은
                작품 설정에서 바꿀 수 있습니다.
              </p>
            </>
          ) : (
            <>
              <label className="field">
                <span className="field-label">넣을 곳</span>
                <select value={target} onChange={(e) => setTarget(e.target.value)}>
                  {ov.parts.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.title} (맨 끝에 이어서)
                    </option>
                  ))}
                </select>
              </label>
              <p className="dialog-text">
                {noun} <strong>{chosen.length}개</strong> · 공백 포함 <strong>{num(totalChars)}자</strong>를 가져옵니다. 이미 있는 {noun}는 그대로입니다.
              </p>
            </>
          )}
          {paged?.page && (
            <div className="import-page">
              <label className="check">
                <input type="checkbox" checked={followPage} onChange={(e) => setFollowPage(e.target.checked)} />
                원고 서식도 {paged.name}의 쪽 모양에 맞추기
              </label>
              <p className="hint">{paged.page.summary}</p>
              <p className="hint">
                {followPage
                  ? '글꼴, 글자 크기, 줄 간격은 지금 원고 서식 그대로 둡니다. 작품 설정에서 언제든 다시 바꿀 수 있습니다.'
                  : '켜지 않으면 머리말·꼬리말·쪽 번호는 가져오지 않고, 원고 서식도 그대로입니다.'}
              </p>
            </div>
          )}
          {leaving && (
            <div className="import-warn" role="alert">
              <strong>표·그림은 가져올 수 없습니다</strong>
              <p>
                이 앱은 표, 그림, 각주를 지원하지 않아 그대로 가져오면 모양이 깨집니다. <b>{skipText(skipped)}</b>를 뺀 글만 가져올까요?
              </p>
              <ul>
                {chosen
                  .filter((c) => hasSkips(c.skipped))
                  .slice(0, 6)
                  .map((c) => (
                    <li key={c.index}>
                      {rows.find((r) => r.index === c.index)?.title.trim() || '제목 없음'} — {skipText(c.skipped)}
                    </li>
                  ))}
                {chosen.filter((c) => hasSkips(c.skipped)).length > 6 && <li>…</li>}
              </ul>
              <label className="check">
                <input type="checkbox" checked={leaveNotes} onChange={(e) => setLeaveNotes(e.target.checked)} />
                빠진 자리에 메모를 남기기
              </label>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
