// 편집 용지 on a page drawn to scale, as in 한글: rulers along the top and the
// left in mm, the margins as lines that can be dragged, and the first-line
// indent as a mark on the top ruler. The numbers beside it stay for exact
// values; the one being typed in lights up its line here.

import { useMemo, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import type { HeadAlign, IndentRules, ManuscriptFormat, RunningHead } from '../api/types';

export type Guide = 'top' | 'header' | 'footer' | 'bottom' | 'inside' | 'outside' | 'indent';

export const GUIDE_LABEL: Record<Guide, string> = {
  top: '위',
  header: '머리말',
  footer: '꼬리말',
  bottom: '아래',
  inside: '왼쪽(안쪽)',
  outside: '오른쪽(바깥쪽)',
  indent: '첫 줄 들여쓰기',
};

const PREVIEW_FONT: Record<string, string> = {
  batang: "'Batang', 'Noto Serif KR', serif",
  dotum: "'Malgun Gothic', 'IBM Plex Sans KR', sans-serif",
  'nanum-myeongjo': "'Nanum Myeongjo', serif",
  'noto-serif': "'Noto Serif KR', serif",
  'gowun-batang': "'Gowun Batang', serif",
};

/** A sample chapter: narration, dialogue, a scene break and a letter. */
const SAMPLE: { text: string; left?: number; scene?: boolean }[] = [
  { text: '셔터를 반쯤 내렸을 때 종이 울렸다. 이 시간에 문을 여는 사람은 없었다. 적어도 지난 삼 년 동안은 그랬다.' },
  { text: '“영업, 끝났나요?”' },
  { text: '“끝났어요. 그런데 들어오세요. 그 봉투가 젖으면 곤란할 것 같으니까.”' },
  { text: '윤서하는 계산대 아래에서 우산을 꺼내 들었다. 문턱에 선 남자는 우산도 없이 젖어 있었고, 품에 안은 종이봉투만은 이상하리만치 말라 있었다.' },
  { text: '', scene: true },
  { text: '남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다. 누렇게 바랜 칸마다 같은 이름이 적혀 있었다.' },
  { text: '서하에게. 이 카드를 찾았다면 지하 서고의 열쇠도 찾은 거란다.', left: 2 },
  { text: '마지막 칸의 날짜는 서하가 태어나기도 전이었다. 서하는 카드를 뒤집어 보았다. 뒷면에는 연필로 쓴 숫자가 희미하게 남아 있었다.' },
  { text: '“이거, 어디서 나셨어요?”' },
  { text: '남자는 대답 대신 창밖을 보았다. 빗줄기가 가늘어지고 있었다.' },
];

const QUOTES = ['“', '"', '「', '『', '‘', "'", '《', '«'];

/** How the rules shape each sample paragraph (same as editor/indent.ts). */
function shapes(rules: IndentRules | undefined): ('flush' | 'hang' | null)[] {
  let first = true;
  let afterScene = false;
  return SAMPLE.map((p) => {
    if (p.scene) {
      afterScene = true;
      return null;
    }
    const dialogue = QUOTES.includes(p.text[0]);
    let kind: 'flush' | 'hang' | null = null;
    if (rules) {
      if ((rules.chapterFirst && first) || (rules.afterScene && afterScene) || (rules.margined && (p.left ?? 0) > 0) || (rules.dialogue && dialogue)) {
        kind = 'flush';
      } else if (rules.dialogueHang && dialogue) {
        kind = 'hang';
      }
    }
    first = false;
    afterScene = false;
    return kind;
  });
}

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

/** Where a running line lands on the first page shown (an odd page). */
function oddSide(align: HeadAlign): 'left' | 'center' | 'right' {
  return align === 'outside' ? 'right' : align;
}

const RULER = 20;
/** Smallest body the margins must leave, in mm (format.rs validate). */
const MIN_BODY = 20;

export function PageEditor({
  format,
  onChange,
  title,
  penName,
  lit,
  maxWidth = 380,
  maxHeight = 540,
}: {
  format: ManuscriptFormat;
  onChange: (f: ManuscriptFormat) => void;
  title: string;
  penName: string;
  /** A line to light up (the number being typed in). */
  lit: Guide | null;
  maxWidth?: number;
  maxHeight?: number;
}) {
  const hasPaper = format.paper.kind !== 'none';
  const [W, H] = hasPaper ? [format.paper.widthMm, format.paper.heightMm] : [210, 297];
  const scale = Math.min(maxWidth / W, maxHeight / H); // px per mm
  const m = format.margins;
  const sizeMm = format.sizePt * (25.4 / 72);
  const charMm = sizeMm * (1 + format.letterSpacing / 100);
  const page = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ guide: Guide; value: number } | null>(null);
  const kinds = useMemo(() => shapes(format.indentRules), [format.indentRules]);

  // Where each line is on the page, in mm.
  const at: Record<Guide, number> = {
    top: m.top,
    header: m.top + m.header,
    footer: H - m.bottom - m.footer,
    bottom: H - m.bottom,
    inside: m.inside,
    outside: W - m.outside,
    indent: m.inside + format.indent * charMm,
  };
  const vertical = (g: Guide) => g === 'inside' || g === 'outside' || g === 'indent';

  /** The format with one line moved to `mm` (kept sensible), and its shown value. */
  const moved = (guide: Guide, mm: number): { next: ManuscriptFormat; value: number } => {
    const r = (v: number) => Math.round(v);
    const bodyW = (inside: number, outside: number) => W - inside - outside >= MIN_BODY;
    const bodyH = (next: ManuscriptFormat['margins']) => H - next.top - next.header - next.bottom - next.footer >= MIN_BODY;
    const margins = { ...m };
    let value = 0;
    switch (guide) {
      case 'top':
        value = margins.top = Math.max(0, r(mm));
        break;
      case 'header':
        value = margins.header = Math.max(0, r(mm - m.top));
        break;
      case 'footer':
        value = margins.footer = Math.max(0, r(H - m.bottom - mm));
        break;
      case 'bottom':
        value = margins.bottom = Math.max(0, r(H - mm));
        break;
      case 'inside':
        value = margins.inside = Math.max(0, r(mm));
        break;
      case 'outside':
        value = margins.outside = Math.max(0, r(W - mm));
        break;
      case 'indent': {
        const indent = Math.max(0, Math.min(10, Math.round(((mm - m.inside) / charMm) * 2) / 2));
        return { next: { ...format, indent }, value: indent };
      }
    }
    const fits = vertical(guide) ? bodyW(margins.inside, margins.outside) : bodyH(margins);
    return { next: fits ? { ...format, margins } : format, value: fits ? value : (m as Record<string, number>)[guide] };
  };

  const start = (guide: Guide) => (e: ReactPointerEvent) => {
    if (!hasPaper) return;
    e.preventDefault();
    const target = e.currentTarget as HTMLElement;
    try {
      target.setPointerCapture(e.pointerId);
    } catch {
      // The drag still follows the events.
    }
    const mmAt = (ev: { clientX: number; clientY: number }) => {
      const r = page.current!.getBoundingClientRect();
      return vertical(guide) ? (ev.clientX - r.left) / scale : (ev.clientY - r.top) / scale;
    };
    const move = (ev: PointerEvent) => setDrag({ guide, value: moved(guide, mmAt(ev)).value });
    const up = (ev: PointerEvent) => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', up);
      target.removeEventListener('pointercancel', up);
      setDrag(null);
      if (ev.type !== 'pointercancel') onChange(moved(guide, mmAt(ev)).next);
    };
    setDrag({ guide, value: moved(guide, at[guide]).value });
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', up);
    target.addEventListener('pointercancel', up);
  };

  // While dragging, the page shows the format as it would be.
  const shown = drag ? moved(drag.guide, (() => {
    const g = drag.guide;
    switch (g) {
      case 'top': return drag.value;
      case 'header': return m.top + drag.value;
      case 'footer': return H - m.bottom - drag.value;
      case 'bottom': return H - drag.value;
      case 'inside': return drag.value;
      case 'outside': return W - drag.value;
      case 'indent': return m.inside + drag.value * charMm;
    }
  })()).next : format;
  const sm = shown.margins;
  const sizePx = sizeMm * scale;
  const style = {
    width: W * scale,
    height: H * scale,
    '--pv-font': PREVIEW_FONT[shown.font] ?? PREVIEW_FONT.batang,
    '--pv-size': `${sizePx}px`,
    '--pv-leading': String(shown.lineSpacing / 100),
    '--pv-tracking': `${shown.letterSpacing / 100}em`,
    '--pv-indent': `${shown.indent}em`,
    '--pv-gap': shown.blankLineBetween ? `${shown.lineSpacing / 100}em` : '0',
  } as CSSProperties;
  const area: CSSProperties = hasPaper
    ? { left: sm.inside * scale, right: sm.outside * scale, top: (sm.top + sm.header) * scale, bottom: (sm.bottom + sm.footer) * scale }
    : { left: 30 * scale, right: 30 * scale, top: 20 * scale, bottom: 0 };
  const shownAt: Record<Guide, number> = {
    top: sm.top,
    header: sm.top + sm.header,
    footer: H - sm.bottom - sm.footer,
    bottom: H - sm.bottom,
    inside: sm.inside,
    outside: W - sm.outside,
    indent: sm.inside + shown.indent * charMm,
  };

  const unit = (g: Guide, v: number) => (g === 'indent' ? `${v}자` : `${v}mm`);
  const active = drag?.guide ?? lit;
  const bubble = drag ?? (lit ? { guide: lit, value: lit === 'indent' ? format.indent : (lit === 'header' ? m.header : lit === 'footer' ? m.footer : (m as Record<string, number>)[lit]) } : null);

  const ticks = (length: number, horizontal: boolean) => {
    const out = [];
    const step = scale >= 2.5 ? 1 : 2;
    for (let mm = 0; mm <= length; mm += step) {
      const size = mm % 10 === 0 ? 'major' : mm % 5 === 0 ? 'mid' : '';
      out.push(
        <span key={mm} className={`pe-tick ${size}`} style={horizontal ? { left: mm * scale } : { top: mm * scale }}>
          {mm % 10 === 0 && mm > 0 ? <i>{mm / 10}</i> : null}
        </span>,
      );
    }
    return out;
  };

  const handle = (g: Guide) => {
    const pos = shownAt[g] * scale;
    const horizontalRuler = vertical(g);
    return (
      <button
        key={g}
        type="button"
        className={`pe-handle ${horizontalRuler ? 'x' : 'y'}${g === 'indent' ? ' indent' : ''}${active === g ? ' on' : ''}`}
        style={horizontalRuler ? { left: RULER + pos } : { top: RULER + pos }}
        title={`${GUIDE_LABEL[g]} · 끌어서 바꾸기`}
        aria-label={GUIDE_LABEL[g]}
        onPointerDown={start(g)}
      />
    );
  };

  if (!hasPaper) {
    return (
      <div className="page-editor continuous">
        <div ref={page} className="page-preview continuous" style={style} aria-label="원고 서식 미리보기">
          <Sample area={area} kinds={kinds} />
        </div>
      </div>
    );
  }

  const guides: Guide[] = ['top', 'header', 'footer', 'bottom', 'inside', 'outside'];

  return (
    <div className="page-editor" style={{ width: RULER + W * scale, height: RULER + H * scale }}>
      <div className="pe-ruler top" style={{ left: RULER, width: W * scale }}>
        <div className="pe-body" style={{ left: sm.inside * scale, width: (W - sm.inside - sm.outside) * scale }} />
        {ticks(W, true)}
      </div>
      <div className="pe-ruler left" style={{ top: RULER, height: H * scale }}>
        <div className="pe-body" style={{ top: (sm.top + sm.header) * scale, height: (H - sm.top - sm.header - sm.bottom - sm.footer) * scale }} />
        {ticks(H, false)}
      </div>
      <div ref={page} className="page-preview" style={{ ...style, position: 'absolute', left: RULER, top: RULER }} aria-label="원고 서식 미리보기">
        <Sample area={area} kinds={kinds} />
        {shown.header.content !== 'none' && (
          <div
            className="pv-head"
            style={{
              left: sm.inside * scale,
              right: sm.outside * scale,
              top: sm.top * scale,
              height: sm.header * scale,
              fontSize: sizePx * 0.9,
              textAlign: shown.header.align === 'outside' ? 'right' : shown.header.align,
            }}
          >
            {previewHead(shown.header, title, penName)}
          </div>
        )}
        {(shown.pageNumbers || shown.footer.text.trim() !== '') && (
          <div
            className="pv-foot"
            style={{
              left: sm.inside * scale,
              right: sm.outside * scale,
              top: (H - sm.bottom - sm.footer) * scale,
              height: sm.footer * scale,
              fontSize: sizePx * 0.9,
            }}
          >
            {(['left', 'center', 'right'] as const).map((slot) => (
              <span key={slot} className={`pv-foot-${slot}`}>
                {shown.pageNumbers && oddSide(shown.pageNumberAlign) === slot && <span className="pv-page-number">- 1 -</span>}
                {shown.footer.text.trim() !== '' && oddSide(shown.footer.align) === slot && shown.footer.text}
              </span>
            ))}
          </div>
        )}
        {guides.map((g) => (
          <div
            key={g}
            className={`pe-line ${vertical(g) ? 'x' : 'y'}${active === g ? ' on' : ''}`}
            style={vertical(g) ? { left: shownAt[g] * scale } : { top: shownAt[g] * scale }}
            onPointerDown={start(g)}
            title={`${GUIDE_LABEL[g]} · 끌어서 바꾸기`}
          />
        ))}
      </div>
      {[...guides, 'indent' as Guide].map(handle)}
      {bubble && (
        <div
          className="pe-bubble"
          style={
            vertical(bubble.guide)
              ? { left: RULER + shownAt[bubble.guide] * scale, top: RULER + 6 }
              : { top: RULER + shownAt[bubble.guide] * scale, left: RULER + (W * scale) / 2 }
          }
        >
          {GUIDE_LABEL[bubble.guide]} {unit(bubble.guide, bubble.value)}
        </div>
      )}
    </div>
  );
}

function Sample({ area, kinds }: { area: CSSProperties; kinds: ('flush' | 'hang' | null)[] }) {
  return (
    <div className="pv-area" style={area}>
      <p className="pv-title">1장 비에 젖은 손님</p>
      {SAMPLE.map((p, i) =>
        p.scene ? (
          <p key={i} className="pv-para pv-scene">
            ◆
          </p>
        ) : (
          <p
            key={i}
            className={`pv-para${kinds[i] === 'flush' ? ' flush' : kinds[i] === 'hang' ? ' hang' : ''}`}
            style={p.left ? { marginLeft: `${p.left}em` } : undefined}
          >
            {p.text}
          </p>
        ),
      )}
    </div>
  );
}
