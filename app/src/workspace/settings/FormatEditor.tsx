// 원고 서식 in 작품 설정: presets, paper, margins, type and spacing, with a live page preview and page estimate.

import { useEffect, useState } from 'react';
import { api } from '../../api';
import type { FormatCatalog, ManuscriptFormat } from '../../api/types';
import { num } from '../../lib/format';
import { formatName } from '../../lib/labels';
import { loadCatalog, toastError } from '../../store';
import { PageEditor, type Guide } from '../PageEditor';
import { FootField, HeadField } from './HeadFootFields';
import { Num } from './Num';

export function FormatEditor({
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
  // The line on the page for the number being typed in.
  const [lit, setLit] = useState<Guide | null>(null);
  const light = (g: Guide) => ({ onFocus: () => setLit(g), onBlur: () => setLit(null) });
  const rules = format.indentRules;
  const setRule = (patch: Partial<typeof rules>) => set({ indentRules: { ...rules, ...patch } });
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
                위 <Num value={format.margins.top} onChange={(v) => setMargin('top', v)} {...light('top')} label="위 여백" />
              </label>
              <label>
                아래 <Num value={format.margins.bottom} onChange={(v) => setMargin('bottom', v)} {...light('bottom')} label="아래 여백" />
              </label>
              <label>
                왼쪽(안쪽) <Num value={format.margins.inside} onChange={(v) => setMargin('inside', v)} {...light('inside')} label="왼쪽 여백" />
              </label>
              <label>
                오른쪽(바깥쪽) <Num value={format.margins.outside} onChange={(v) => setMargin('outside', v)} {...light('outside')} label="오른쪽 여백" />
              </label>
              <label>
                머리말 <Num value={format.margins.header} onChange={(v) => setMargin('header', v)} {...light('header')} label="머리말 여백" />
              </label>
              <label>
                꼬리말 <Num value={format.margins.footer} onChange={(v) => setMargin('footer', v)} {...light('footer')} label="꼬리말 여백" />
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
              첫 줄 들여쓰기 <Num value={format.indent} step={0.5} onChange={(indent) => set({ indent })} label="첫 줄 들여쓰기" {...light('indent')} /> 자
            </label>
          </div>
        </div>

        {format.indent > 0 && (
          <fieldset className="field">
            <legend className="field-label">첫 줄을 들이지 않을 곳</legend>
            <div className="rule-grid">
              <label className="check">
                <input type="checkbox" checked={rules.margined} onChange={(e) => setRule({ margined: e.target.checked })} />
                여백 준 문단 (편지·인용)
              </label>
              <label className="check">
                <input type="checkbox" checked={rules.chapterFirst} onChange={(e) => setRule({ chapterFirst: e.target.checked })} />
                장·회차 첫 문단
              </label>
              <label className="check">
                <input type="checkbox" checked={rules.afterScene} onChange={(e) => setRule({ afterScene: e.target.checked })} />
                장면 나눔 바로 뒤 문단
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={rules.dialogue}
                  onChange={(e) => setRule({ dialogue: e.target.checked, dialogueHang: e.target.checked ? false : rules.dialogueHang })}
                />
                대화문 (따옴표로 시작)
              </label>
            </div>
            <label className="check">
              <input
                type="checkbox"
                checked={rules.dialogueHang}
                disabled={rules.dialogue}
                onChange={(e) => setRule({ dialogueHang: e.target.checked })}
              />
              대화문을 원고지처럼: 모든 줄을 한 칸 들이기
            </label>
            <small className="hint">
              한국 소설책은 보통 여백 준 문단만 켭니다. 영미권 책처럼 하려면 장 첫 문단과 장면 나눔 뒤도 켜세요. 문단마다 따로 정한 첫 줄(눈금자, 문단 여백 메뉴)이 이 규칙보다 먼저입니다.
            </small>
          </fieldset>
        )}

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
            </>
          )}
        </div>

        {hasPaper && <HeadField format={format} onChange={set} />}
        {hasPaper && <FootField format={format} onChange={set} />}
      </div>

      <div className="format-side">
        <PageEditor
          format={format}
          onChange={onChange}
          title={title}
          penName={penName}
          lit={lit}
          maxHeight={Math.max(300, Math.min(560, window.innerHeight * 0.8 - 300))}
        />
        {hasPaper && <small className="hint">자·선을 끌어 여백과 첫 줄 들여쓰기를 맞춥니다.</small>}
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
