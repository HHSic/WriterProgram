// Writing screen settings (작성 화면 설정): per device, never written into
// manuscript files (docs/mvp-scope.md "서식 모델").

import type { AccentId, PaletteId } from './colors';

export type Theme = 'system' | 'light' | 'dark';
export type BodyFont = 'noto-serif' | 'gowun-batang' | 'nanum-myeongjo' | 'sans';

export interface ViewSettings {
  theme: Theme;
  /** 화면 색: beige (the default), a preset tint, or one made from customColor. */
  palette: PaletteId;
  /** 강조 색; auto takes the palette's own. */
  accent: AccentId;
  /** #rrggbb picked for palette custom. */
  customColor: string;
  font: BodyFont;
  /** px */
  fontSize: number;
  /** multiple of the font size */
  lineHeight: number;
  /** letter spacing (자간) in percent of the font size, as in 한글 */
  letterSpacing: number;
  /** space between paragraphs, in lines */
  paragraphGap: number;
  /** first-line indent, in characters */
  indent: number;
  /** text column width, px */
  width: number;
  /** 빈칸·문단 부호 보이기 */
  showMarks: boolean;
  /** 편집 도구줄 under the page: on touch screens, always, or never. */
  toolbar: 'auto' | 'always' | 'never';
  /** 눈금자 over the page (paragraph margins and first line). Off: the
   * margins are set in 원고 서식, a paragraph's own in the 문단 여백 menu. */
  ruler: boolean;
}

export const FONT_LABEL: Record<BodyFont, string> = {
  'noto-serif': '본명조 (Noto Serif KR)',
  'gowun-batang': '고운바탕',
  'nanum-myeongjo': '나눔명조',
  sans: '고딕 (IBM Plex Sans KR)',
};

export const FONT_STACK: Record<BodyFont, string> = {
  'noto-serif': "'Noto Serif KR', 'Batang', serif",
  'gowun-batang': "'Gowun Batang', 'Noto Serif KR', serif",
  'nanum-myeongjo': "'Nanum Myeongjo', 'Noto Serif KR', serif",
  sans: "'IBM Plex Sans KR', 'Malgun Gothic', sans-serif",
};

export const DEFAULT_VIEW: ViewSettings = {
  theme: 'system',
  palette: 'beige',
  accent: 'auto',
  customColor: '#e8e0c8',
  font: 'noto-serif',
  fontSize: 18,
  lineHeight: 1.95,
  letterSpacing: 0,
  paragraphGap: 0.5,
  indent: 0,
  width: 560,
  showMarks: false,
  toolbar: 'auto',
  ruler: false,
};

const KEY = 'wp.view';

export function loadView(): ViewSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULT_VIEW, ...(JSON.parse(raw) as Partial<ViewSettings>) };
  } catch {
    // Storage can be unavailable; the defaults are fine.
  }
  return { ...DEFAULT_VIEW };
}

export function saveView(view: ViewSettings) {
  try {
    localStorage.setItem(KEY, JSON.stringify(view));
  } catch {
    // Not critical: settings only last for this session.
  }
}

/** CSS variables for the writing surface. */
export function viewStyle(view: ViewSettings): Record<string, string> {
  return {
    '--body-font': FONT_STACK[view.font],
    '--body-size': `${view.fontSize}px`,
    '--body-leading': String(view.lineHeight),
    '--tracking': `${view.letterSpacing / 100}em`,
    '--para-gap': `${view.paragraphGap * view.lineHeight}em`,
    '--indent': `${view.indent}em`,
    '--measure': `${view.width}px`,
  };
}
