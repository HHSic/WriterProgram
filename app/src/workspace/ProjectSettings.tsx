// 작품 설정 (S12): basic information, goals and the manuscript format
// (원고 서식) with a live page preview and page estimate.

import { useEffect, useMemo, useState, type CSSProperties } from 'react';
import { api } from '../api';
import type { FormatCatalog, Goal, HeadAlign, HeadContent, ManuscriptFormat, ProjectKind, RunningHead } from '../api/types';
import { Modal } from '../components/Modal';
import { num } from '../lib/format';
import { KIND_LABEL, docNoun, formatName } from '../lib/labels';
import { closeDialog, loadCatalog, toastError, updateProject, useApp, type SettingsTab } from '../store';

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
      width={tab === 'format' ? 860 : 560}
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
            </div>
          </div>
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

// ---------------------------------------------------------------------------
// 원고 서식

const PREVIEW_FONT: Record<string, string> = {
  batang: "'Batang', 'Noto Serif KR', serif",
  dotum: "'Malgun Gothic', 'IBM Plex Sans KR', sans-serif",
  'nanum-myeongjo': "'Nanum Myeongjo', serif",
  'noto-serif': "'Noto Serif KR', serif",
  'gowun-batang': "'Gowun Batang', serif",
};

const SAMPLE = [
  '셔터를 반쯤 내렸을 때 종이 울렸다. 이 시간에 문을 여는 사람은 없었다. 적어도 지난 삼 년 동안은 그랬다.',
  '“영업, 끝났나요?”',
  '“끝났어요. 그런데 들어오세요. 그 봉투가 젖으면 곤란할 것 같으니까.”',
  '윤서하는 계산대 아래에서 우산을 꺼내 들었다. 문턱에 선 남자는 우산도 없이 젖어 있었고, 품에 안은 종이봉투만은 이상하리만치 말라 있었다.',
  '남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다. 누렇게 바랜 칸마다 같은 이름이 적혀 있었다.',
  '마지막 칸의 날짜는 서하가 태어나기도 전이었다. 서하는 카드를 뒤집어 보았다.',
];

function FormatEditor({
  format,
  onChange,
  catalog,
  root,
  title,
  penName,
}: {
  format: ManuscriptFormat;
  onChange: (f: ManuscriptFormat) => void;
  catalog: FormatCatalog | null;
  root: string;
  title: string;
  penName: string;
}) {
  const [estimate, setEstimate] = useState<number | null | undefined>(undefined);
  const [naming, setNaming] = useState(false);
  const [presetName, setPresetName] = useState('');
  const hasPaper = format.paper.kind !== 'none';
  const userPreset = catalog?.user.find((u) => u.name === format.preset);

  useEffect(() => {
    let alive = true;
    const timer = setTimeout(() => {
      api.formatEstimate(root, format).then(
        (pages) => alive && setEstimate(pages),
        () => alive && setEstimate(undefined),
      );
    }, 250);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [root, format]);

  const set = (patch: Partial<ManuscriptFormat>) => onChange({ ...format, ...patch });
  const setMargin = (key: keyof ManuscriptFormat['margins'], v: number) => set({ margins: { ...format.margins, [key]: v } });

  const pickPreset = (value: string) => {
    const [source, key] = value.split(':');
    const found =
      source === 'builtin'
        ? catalog?.builtin.find((b) => b.id === key)?.format
        : catalog?.user.find((u) => u.name === key)?.format;
    if (found) onChange(structuredClone(found));
  };

  const pickPaper = (key: string) => {
    if (key === 'none' || key === 'custom') {
      set({ paper: { ...format.paper, kind: key } });
      return;
    }
    const paper = catalog?.papers.find((p) => p.key === key);
    if (paper) set({ paper: { kind: key, widthMm: paper.widthMm, heightMm: paper.heightMm } });
  };

  const savePreset = async () => {
    try {
      await api.formatSavePreset(presetName, format);
      await loadCatalog();
      onChange({ ...format, preset: presetName.trim() });
      setNaming(false);
    } catch (e) {
      toastError('서식을 저장하지 못함', e);
    }
  };

  const deletePreset = async (name: string) => {
    try {
      await api.formatDeletePreset(name);
      await loadCatalog();
    } catch (e) {
      toastError('서식을 지우지 못함', e);
    }
  };

  const currentValue = catalog?.builtin.some((b) => b.id === format.preset)
    ? `builtin:${format.preset}`
    : userPreset
      ? `user:${userPreset.name}`
      : '';

  return (
    <div className="format-editor">
      <div className="form">
        <div className="field">
          <span className="field-label">서식</span>
          <div className="row">
            <select className="grow" value={currentValue} onChange={(e) => pickPreset(e.target.value)} aria-label="서식 고르기">
              {currentValue === '' && <option value="">직접 정한 서식</option>}
              <optgroup label="기본 서식">
                {catalog?.builtin.map((b) => (
                  <option key={b.id} value={`builtin:${b.id}`}>
                    {b.name}
                  </option>
                ))}
              </optgroup>
              {catalog && catalog.user.length > 0 && (
                <optgroup label="내 서식">
                  {catalog.user.map((u) => (
                    <option key={u.name} value={`user:${u.name}`}>
                      {u.name}
                    </option>
                  ))}
                </optgroup>
              )}
            </select>
            {userPreset && (
              <button type="button" className="btn ghost" onClick={() => void deletePreset(userPreset.name)}>
                지우기
              </button>
            )}
            <button
              type="button"
              className="btn"
              onClick={() => {
                setPresetName(userPreset?.name ?? '');
                setNaming(true);
              }}
            >
              내 서식으로 저장
            </button>
          </div>
          {naming && (
            <div className="row">
              <input
                className="grow"
                autoFocus
                value={presetName}
                placeholder="서식 이름 (예: 출판사 A 투고 규정)"
                aria-label="서식 이름"
                onChange={(e) => setPresetName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
                    e.preventDefault();
                    void savePreset();
                  }
                }}
              />
              <button type="button" className="btn primary small" disabled={!presetName.trim()} onClick={() => void savePreset()}>
                저장
              </button>
              <button type="button" className="btn small" onClick={() => setNaming(false)}>
                취소
              </button>
            </div>
          )}
          <small className="hint">지금 모양: {formatName(format, catalog)}. 내보낸 docx·한글 파일과 예상 쪽수에 쓰입니다. 작성 화면 모양은 보기 설정에서 따로 바꿉니다.</small>
        </div>

        <div className="field">
          <span className="field-label">용지</span>
          <div className="row">
            <select value={format.paper.kind} onChange={(e) => pickPaper(e.target.value)} aria-label="용지">
              {catalog?.papers.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label} ({p.widthMm}×{p.heightMm}mm)
                </option>
              ))}
              <option value="custom">사용자 지정</option>
              <option value="none">용지 없음 (이어지는 원고)</option>
            </select>
            {format.paper.kind === 'custom' && (
              <>
                <Num value={format.paper.widthMm} step={1} onChange={(v) => set({ paper: { ...format.paper, widthMm: v } })} label="용지 너비" />
                <span>×</span>
                <Num value={format.paper.heightMm} step={1} onChange={(v) => set({ paper: { ...format.paper, heightMm: v } })} label="용지 높이" />
                <span>mm</span>
              </>
            )}
          </div>
        </div>

        {hasPaper && (
          <div className="field">
            <span className="field-label">여백 (mm)</span>
            <div className="margin-grid">
              <label>
                위 <Num value={format.margins.top} onChange={(v) => setMargin('top', v)} label="위 여백" />
              </label>
              <label>
                아래 <Num value={format.margins.bottom} onChange={(v) => setMargin('bottom', v)} label="아래 여백" />
              </label>
              <label>
                왼쪽(안쪽) <Num value={format.margins.inside} onChange={(v) => setMargin('inside', v)} label="왼쪽 여백" />
              </label>
              <label>
                오른쪽(바깥쪽) <Num value={format.margins.outside} onChange={(v) => setMargin('outside', v)} label="오른쪽 여백" />
              </label>
              <label>
                머리말 <Num value={format.margins.header} onChange={(v) => setMargin('header', v)} label="머리말 여백" />
              </label>
              <label>
                꼬리말 <Num value={format.margins.footer} onChange={(v) => setMargin('footer', v)} label="꼬리말 여백" />
              </label>
            </div>
          </div>
        )}

        <div className="field">
          <span className="field-label">글자</span>
          <div className="row wrap">
            <select value={format.font} onChange={(e) => set({ font: e.target.value })} aria-label="글꼴">
              {catalog?.fonts.map((f) => (
                <option key={f.key} value={f.key}>
                  {f.label}
                </option>
              ))}
            </select>
            <label className="row">
              크기 <Num value={format.sizePt} step={0.5} onChange={(sizePt) => set({ sizePt })} label="글자 크기" /> pt
            </label>
          </div>
        </div>

        <div className="field">
          <span className="field-label">간격</span>
          <div className="row wrap">
            <label className="row">
              줄 간격 <Num value={format.lineSpacing} step={5} onChange={(lineSpacing) => set({ lineSpacing: Math.round(lineSpacing) })} label="줄 간격" /> %
            </label>
            <label className="row">
              자간 <Num value={format.letterSpacing} step={1} onChange={(letterSpacing) => set({ letterSpacing: Math.round(letterSpacing) })} label="자간" allowNegative /> %
            </label>
            <label className="row">
              첫 줄 들여쓰기 <Num value={format.indent} step={0.5} onChange={(indent) => set({ indent })} label="첫 줄 들여쓰기" /> 자
            </label>
          </div>
        </div>

        <div className="field">
          <label className="check">
            <input type="checkbox" checked={format.blankLineBetween} onChange={(e) => set({ blankLineBetween: e.target.checked })} />
            문단 사이에 빈 줄
          </label>
          {hasPaper && (
            <>
              <label className="check">
                <input type="checkbox" checked={format.chapterNewPage} onChange={(e) => set({ chapterNewPage: e.target.checked })} />
                장마다 새 쪽에서 시작
              </label>
              <label className="check">
                <input type="checkbox" checked={format.pageNumbers} onChange={(e) => set({ pageNumbers: e.target.checked })} />
                쪽 번호 (아래 가운데)
              </label>
            </>
          )}
        </div>

        {hasPaper && <HeadField format={format} onChange={set} />}
      </div>

      <div className="format-side">
        <PagePreview format={format} title={title} penName={penName} />
        <p className="estimate">
          {!hasPaper
            ? '용지 없이 이어지는 원고입니다. docx·한글로 내보내면 A4에 담깁니다.'
            : estimate === undefined
              ? '예상 쪽수를 세는 중'
              : estimate === null
                ? ''
                : `이 서식이면 작품 전체 약 ${num(estimate)}쪽`}
        </p>
      </div>
    </div>
  );
}

const HEAD_CONTENTS: { id: HeadContent; label: string }[] = [
  { id: 'none', label: '없음' },
  { id: 'title', label: '작품 제목' },
  { id: 'chapter', label: '장 제목' },
  { id: 'titleChapter', label: '책처럼: 왼쪽 쪽 작품 제목, 오른쪽 쪽 장 제목' },
  { id: 'author', label: '필명' },
  { id: 'custom', label: '직접 쓰기' },
];

const HEAD_ALIGNS: { id: HeadAlign; label: string }[] = [
  { id: 'left', label: '왼쪽' },
  { id: 'center', label: '가운데' },
  { id: 'right', label: '오른쪽' },
  { id: 'outside', label: '바깥쪽 (책처럼 펼친 면의 양 끝)' },
];

/** 머리말: what goes at the top of the pages, and where. */
function HeadField({ format, onChange }: { format: ManuscriptFormat; onChange: (patch: Partial<ManuscriptFormat>) => void }) {
  const head = format.header;
  const setHead = (patch: Partial<RunningHead>) => onChange({ header: { ...head, ...patch } });
  const on = head.content !== 'none';
  return (
    <div className="field">
      <span className="field-label">머리말</span>
      <div className="row wrap">
        <select value={head.content} onChange={(e) => setHead({ content: e.target.value as HeadContent })} aria-label="머리말 내용">
          {HEAD_CONTENTS.map((c) => (
            <option key={c.id} value={c.id}>
              {c.label}
            </option>
          ))}
        </select>
        {head.content === 'custom' && (
          <input
            className="grow"
            value={head.text}
            maxLength={100}
            placeholder="머리말에 넣을 글"
            aria-label="머리말 글"
            onChange={(e) => setHead({ text: e.target.value })}
          />
        )}
        {on && (
          <select value={head.align} onChange={(e) => setHead({ align: e.target.value as HeadAlign })} aria-label="머리말 자리">
            {HEAD_ALIGNS.map((a) => (
              <option key={a.id} value={a.id}>
                {a.label}
              </option>
            ))}
          </select>
        )}
      </div>
      {on && format.chapterNewPage && (
        <label className="check">
          <input type="checkbox" checked={head.skipChapterFirst} onChange={(e) => setHead({ skipChapterFirst: e.target.checked })} />
          장이 시작하는 쪽에는 넣지 않기
        </label>
      )}
      {head.content === 'author' && <small className="hint">필명은 작품 설정의 기본 정보에서 적습니다.</small>}
    </div>
  );
}

function Num({
  value,
  onChange,
  label,
  step = 1,
  allowNegative = false,
}: {
  value: number;
  onChange: (v: number) => void;
  label: string;
  step?: number;
  allowNegative?: boolean;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  return (
    <input
      className="num"
      type="number"
      step={step}
      min={allowNegative ? undefined : 0}
      value={text}
      aria-label={label}
      onChange={(e) => {
        setText(e.target.value);
        const v = Number.parseFloat(e.target.value);
        if (Number.isFinite(v)) onChange(v);
      }}
    />
  );
}

/** What 머리말 shows on the previewed page (the first page, an odd one). */
function previewHead(head: RunningHead, title: string, penName: string): string {
  switch (head.content) {
    case 'title':
      return title;
    case 'chapter':
    case 'titleChapter':
      return '1장 비에 젖은 손님';
    case 'author':
      return penName || '필명';
    case 'custom':
      return head.text;
    default:
      return '';
  }
}

/** A page drawn to scale with the format's type, spacing and margins. */
function PagePreview({ format, title, penName }: { format: ManuscriptFormat; title: string; penName: string }) {
  const hasPaper = format.paper.kind !== 'none';
  const width = 300;
  const [w, h] = hasPaper ? [format.paper.widthMm, format.paper.heightMm] : [210, 297];
  const scale = width / w; // px per mm
  const m = format.margins;
  const sizePx = format.sizePt * (25.4 / 72) * scale;
  const style = useMemo(
    () =>
      ({
        width,
        height: h * scale,
        '--pv-font': PREVIEW_FONT[format.font] ?? PREVIEW_FONT.batang,
        '--pv-size': `${sizePx}px`,
        '--pv-leading': String(format.lineSpacing / 100),
        '--pv-tracking': `${format.letterSpacing / 100}em`,
        '--pv-indent': `${format.indent}em`,
        '--pv-gap': format.blankLineBetween ? `${format.lineSpacing / 100}em` : '0',
      }) as CSSProperties,
    [format, h, scale, sizePx],
  );
  const area: CSSProperties = hasPaper
    ? {
        left: m.inside * scale,
        right: m.outside * scale,
        top: (m.top + m.header) * scale,
        bottom: (m.bottom + m.footer) * scale,
      }
    : { left: 30 * scale, right: 30 * scale, top: 20 * scale, bottom: 0 };

  return (
    <div className={`page-preview${hasPaper ? '' : ' continuous'}`} style={style} aria-label="원고 서식 미리보기">
      <div className="pv-area" style={area}>
        <p className="pv-title">1장 비에 젖은 손님</p>
        {SAMPLE.map((p, i) => (
          <p key={i} className="pv-para">
            {p}
          </p>
        ))}
        {Array.from({ length: 8 }, (_, i) => (
          <p key={`more-${i}`} className="pv-para">
            {SAMPLE[i % SAMPLE.length]}
          </p>
        ))}
      </div>
      {hasPaper && format.header.content !== 'none' && (
        <div
          className="pv-head"
          style={{
            left: m.inside * scale,
            right: m.outside * scale,
            top: m.top * scale,
            height: m.header * scale,
            fontSize: sizePx * 0.9,
            textAlign: format.header.align === 'outside' ? 'right' : format.header.align,
          }}
        >
          {previewHead(format.header, title, penName)}
        </div>
      )}
      {hasPaper && format.pageNumbers && (
        <div className="pv-page-number" style={{ bottom: m.bottom * scale * 0.5, fontSize: sizePx * 0.9 }}>
          - 1 -
        </div>
      )}
    </div>
  );
}
