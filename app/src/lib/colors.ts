// 화면 색 (screen colors) and 강조 색 (accent), per device like the other view
// settings. The default beige lives in styles.css; the others are made from a
// hue and a little chroma in OKLCH, so each has a matching light and dark
// version and text keeps the same contrast whatever the tint.

export type PaletteId = 'beige' | 'white' | 'gray' | 'green' | 'sky' | 'pink' | 'custom';
export type AccentId = 'auto' | 'vermilion' | 'blue' | 'green' | 'violet' | 'ink';

interface Tint {
  hue: number;
  /** Chroma of the page background; the other colors scale from it. */
  chroma: number;
}

export const PALETTES: { id: Exclude<PaletteId, 'custom'>; name: string; tint: Tint | null; accent: Exclude<AccentId, 'auto'> }[] = [
  { id: 'beige', name: '베이지', tint: null, accent: 'vermilion' },
  { id: 'white', name: '흰색', tint: { hue: 90, chroma: 0 }, accent: 'ink' },
  { id: 'gray', name: '회색', tint: { hue: 255, chroma: 0.011 }, accent: 'blue' },
  { id: 'green', name: '연두', tint: { hue: 135, chroma: 0.018 }, accent: 'green' },
  { id: 'sky', name: '하늘', tint: { hue: 232, chroma: 0.017 }, accent: 'blue' },
  { id: 'pink', name: '연분홍', tint: { hue: 12, chroma: 0.014 }, accent: 'vermilion' },
];

export const ACCENTS: { id: Exclude<AccentId, 'auto'>; name: string; light: [string, string]; dark: [string, string] }[] = [
  { id: 'vermilion', name: '주홍', light: ['#b8432f', '#fffdf7'], dark: ['#d0634e', '#1c1a17'] },
  { id: 'blue', name: '파랑', light: ['#2d62ad', '#ffffff'], dark: ['#7eaae8', '#14181e'] },
  { id: 'green', name: '초록', light: ['#2f7649', '#ffffff'], dark: ['#74b98b', '#131a15'] },
  { id: 'violet', name: '보라', light: ['#6a4bb0', '#ffffff'], dark: ['#a98ee8', '#17141d'] },
  { id: 'ink', name: '먹색', light: ['#2b2a28', '#ffffff'], dark: ['#e8e6e1', '#1a1a19'] },
];

/** The tokens a palette sets (the rest of styles.css stays as it is). */
const KEYS = [
  'bg',
  'sidebar',
  'surface',
  'page-border',
  'border',
  'border-strong',
  'text',
  'text-2',
  'muted',
  'body-text',
  'track',
  'hover',
  'overlay',
  'shadow',
  'accent',
  'accent-ink',
] as const;

export type Tokens = Partial<Record<(typeof KEYS)[number], string>>;

function oklch(l: number, c: number, h: number, alpha?: number): string {
  const a = alpha === undefined ? '' : ` / ${alpha}`;
  return `oklch(${l} ${Math.max(0, c).toFixed(4)} ${h.toFixed(1)}${a})`;
}

/** Background, lines and text for a tint. Lightness steps follow the beige in styles.css. */
export function neutrals(tint: Tint, dark: boolean): Tokens {
  const o = (l: number, k: number, alpha?: number) => oklch(l, tint.chroma * k, tint.hue, alpha);
  if (dark) {
    return {
      bg: o(0.215, 0.45),
      sidebar: o(0.235, 0.5),
      surface: o(0.255, 0.5),
      'page-border': o(0.3, 0.7),
      border: o(0.31, 0.7),
      'border-strong': o(0.375, 0.9),
      text: o(0.925, 0.9),
      'text-2': o(0.83, 1),
      muted: o(0.67, 1),
      'body-text': o(0.9, 0.9),
      track: o(0.32, 0.8),
      hover: o(0.925, 0.9, 0.07),
      overlay: 'rgba(0, 0, 0, 0.5)',
      shadow: '0 12px 32px rgba(0, 0, 0, 0.45), 0 2px 6px rgba(0, 0, 0, 0.3)',
    };
  }
  return {
    bg: o(0.957, 1),
    sidebar: o(0.936, 1.2),
    surface: o(0.994, 0.55),
    'page-border': o(0.905, 1.5),
    border: o(0.89, 1.6),
    'border-strong': o(0.85, 1.9),
    text: o(0.225, 0.6),
    'text-2': o(0.39, 1),
    muted: o(0.5, 1.1),
    'body-text': o(0.26, 0.7),
    track: o(0.87, 1.7),
    hover: o(0.225, 0.6, 0.06),
    overlay: o(0.225, 0.6, 0.32),
    shadow: `0 12px 32px ${o(0.225, 0.6, 0.16)}, 0 2px 6px ${o(0.225, 0.6, 0.08)}`,
  };
}

/** sRGB hex (#rrggbb) to an OKLCH hue and a pale chroma for the paper. */
export function hexToTint(hex: string): Tint {
  const m = hex.trim().match(/^#?([0-9a-f]{6})$/i);
  if (!m) return { hue: 90, chroma: 0 };
  const n = parseInt(m[1], 16);
  const lin = (v: number) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const [r, g, b] = [lin((n >> 16) & 255), lin((n >> 8) & 255), lin(n & 255)];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const mm = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const A = 1.9779984951 * l - 2.428592205 * mm + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * mm - 0.808675766 * s;
  const chroma = Math.hypot(A, B);
  const hue = ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360;
  // A picked color becomes a pale paper of its hue: strong colors are toned
  // down so the text stays easy to read.
  return { hue, chroma: Math.min(0.03, chroma * 0.3) };
}

export interface ColorChoice {
  palette: PaletteId;
  accent: AccentId;
  /** For palette 'custom': the picked color, #rrggbb. */
  customColor: string;
}

/** Tokens to set for a choice; empty means styles.css as it is (beige, its own accent). */
export function colorTokens(choice: ColorChoice, dark: boolean): Tokens {
  const preset = PALETTES.find((p) => p.id === choice.palette);
  const tint = choice.palette === 'custom' ? hexToTint(choice.customColor) : (preset?.tint ?? null);
  const tokens: Tokens = tint ? neutrals(tint, dark) : {};
  // Beige with its own accent needs nothing more; anything else sets the accent too.
  if (tint || choice.accent !== 'auto') {
    const accentId = choice.accent === 'auto' ? (preset?.accent ?? 'vermilion') : choice.accent;
    const accent = ACCENTS.find((a) => a.id === accentId) ?? ACCENTS[0];
    const [color, ink] = dark ? accent.dark : accent.light;
    tokens.accent = color;
    tokens['accent-ink'] = ink;
  }
  return tokens;
}

/** The beige of styles.css, for previews. */
const BEIGE = {
  light: { bg: '#f4f1e8', surface: '#fffdf7', border: '#d6cfbe', accent: '#b8432f' },
  dark: { bg: '#1c1a17', surface: '#262320', border: '#48423a', accent: '#d0634e' },
};

/** Colors to draw a small sample of a choice in the settings. */
export function previewColors(choice: ColorChoice, dark: boolean): { bg: string; surface: string; border: string; accent: string } {
  const tokens = colorTokens(choice, dark);
  const beige = dark ? BEIGE.dark : BEIGE.light;
  return {
    bg: tokens.bg ?? beige.bg,
    surface: tokens.surface ?? beige.surface,
    border: tokens['border-strong'] ?? beige.border,
    accent: tokens.accent ?? beige.accent,
  };
}

/** Sets the tokens on the page root, clearing the ones not set. */
export function applyColors(root: HTMLElement, choice: ColorChoice, dark: boolean) {
  const tokens = colorTokens(choice, dark);
  for (const key of KEYS) {
    const value = tokens[key];
    if (value) root.style.setProperty(`--${key}`, value);
    else root.style.removeProperty(`--${key}`);
  }
}
