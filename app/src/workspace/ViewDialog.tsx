// 보기 설정: how the writing screen looks on this device (font, spacing, colours).

import { Modal } from '../components/Modal';
import { ACCENTS, PALETTES, previewColors } from '../lib/colors';
import { FONT_LABEL, type BodyFont, type Theme, type ViewSettings } from '../lib/view';
import { closeDialog, setView, useApp } from '../store';

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
