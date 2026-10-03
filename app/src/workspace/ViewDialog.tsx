// 보기 설정: how the writing screen looks on this device (font, spacing, colours),
// and the app's version with a check for a new one.

import { useEffect, useState } from 'react';
import { api } from '../api';
import { Modal } from '../components/Modal';
import { koreanVoices, loadVoices, pickVoice, sayOnce, speechAvailable } from '../editor/readAloud';
import { ACCENTS, PALETTES, previewColors } from '../lib/colors';
import { FONT_LABEL, READ_RATES, type BodyFont, type Theme, type ViewSettings } from '../lib/view';
import { autoUpdateCheck, checkForUpdateNow, closeDialog, setAutoUpdateCheck, setView, useApp } from '../store';
import { NO_VOICE_HELP, rateLabel } from './ReadingBar';

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
        <label className="check">
          <input type="checkbox" checked={view.ruler} onChange={(e) => setView({ ruler: e.target.checked })} />
          눈금자 보이기 (문단 여백·첫 줄을 끌어서 조절)
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
        <ReadAloudFields view={view} />
        <VersionField />
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

/** 소리 내어 읽기: which Korean voice and how fast. */
function ReadAloudFields({ view }: { view: ViewSettings }) {
  const [voices, setVoices] = useState<SpeechSynthesisVoice[] | null>(null);
  useEffect(() => {
    let alive = true;
    void loadVoices().then((all) => alive && setVoices(koreanVoices(all)));
    return () => {
      alive = false;
    };
  }, []);
  const voice = voices ? pickVoice(voices, view.readVoice) : null;

  return (
    <fieldset className="field read-aloud-fields">
      <legend className="field-label">소리 내어 읽기 (Ctrl+Shift+R)</legend>
      {!speechAvailable() ? (
        <span className="hint">이 기기에서는 소리 내어 읽을 수 없습니다.</span>
      ) : voices === null ? (
        <span className="hint">목소리를 찾는 중</span>
      ) : voices.length === 0 ? (
        <span className="hint">한국어 목소리가 없습니다. {NO_VOICE_HELP}</span>
      ) : (
        <>
          <label className="field">
            <span className="field-label">목소리</span>
            <select value={voice?.voiceURI ?? ''} onChange={(e) => setView({ readVoice: e.target.value })}>
              {voices.map((v) => (
                <option key={v.voiceURI} value={v.voiceURI}>
                  {v.name}
                  {v.localService ? '' : ' (인터넷 연결 필요)'}
                </option>
              ))}
            </select>
          </label>
          <Slider
            label="빠르기"
            value={view.readRate}
            min={READ_RATES[0]}
            max={READ_RATES[READ_RATES.length - 1]}
            step={0.1}
            unit=""
            show={rateLabel}
            onChange={(v) => setView({ readRate: Math.round(v * 10) / 10 })}
          />
          <div>
            <button
              type="button"
              className="btn small"
              disabled={!voice}
              onClick={() => voice && sayOnce('소리 내어 읽기는 이 목소리와 빠르기로 읽습니다.', voice, view.readRate)}
            >
              들어 보기
            </button>
          </div>
        </>
      )}
      <span className="hint">커서가 있는 문장부터 끝까지, 고른 글이 있으면 그 글만 읽습니다. 글을 고치기 시작하면 멈춥니다.</span>
    </fieldset>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  unit,
  show,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  /** How the value is written next to the label, when not just number + unit. */
  show?: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="field slider">
      <span className="field-label">
        {label}
        <output>{show ? show(value) : `${value}${unit}`}</output>
      </span>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  );
}

/** The app's version, and looking for a new one. */
function VersionField() {
  const [version, setVersion] = useState<string | null>(null);
  const [auto, setAuto] = useState(autoUpdateCheck);
  const [checking, setChecking] = useState(false);
  useEffect(() => {
    let alive = true;
    api.appVersion().then(
      (v) => alive && setVersion(v),
      () => {},
    );
    return () => {
      alive = false;
    };
  }, []);
  if (!api.isDesktop) return null;
  return (
    <fieldset className="field">
      <legend className="field-label">버전</legend>
      <div className="row wrap">
        <span className="grow">WriterProgram {version ?? ''}</span>
        <button
          type="button"
          className="btn small"
          disabled={checking}
          onClick={async () => {
            setChecking(true);
            await checkForUpdateNow();
            setChecking(false);
          }}
        >
          {checking ? '확인하는 중…' : '새 버전 확인'}
        </button>
      </div>
      <label className="check">
        <input
          type="checkbox"
          checked={auto}
          onChange={(e) => {
            setAutoUpdateCheck(e.target.checked);
            setAuto(e.target.checked);
          }}
        />
        하루 한 번 새 버전이 있는지 알아보기
      </label>
      <small className="hint">새 버전이 있는지만 GitHub에 묻습니다. 원고나 작품 정보는 보내지 않습니다.</small>
    </fieldset>
  );
}
