// Every keyboard shortcut of the app in one table: the 단축키 list shows it,
// the window's key handler (workspace/Workspace.tsx) matches against it, and
// the editor extensions take their keys from it, so the list cannot drift
// from what the keys do. Keys are written the ProseMirror way: "Mod-Shift-d"
// (Mod is Ctrl, ⌘ on a Mac).

export type ShortcutGroup = '쓰기' | '서식' | '찾기' | '탭·창' | '기타';

export const SHORTCUT_GROUPS: ShortcutGroup[] = ['쓰기', '서식', '찾기', '탭·창', '기타'];

export interface Shortcut {
  group: ShortcutGroup;
  /** What it does, in the writer's words. */
  label: string;
  /** Key combinations, the first one shown first. */
  keys: string[];
  /** Said after the keys in the list, e.g. where it works. */
  note?: string;
}

export const SHORTCUTS = {
  // 쓰기
  save: { group: '쓰기', label: '지금 저장', keys: ['Mod-s'], note: '쓰는 동안 저절로 저장됩니다' },
  undo: { group: '쓰기', label: '되돌리기', keys: ['Mod-z'] },
  redo: { group: '쓰기', label: '다시 하기', keys: ['Mod-y', 'Mod-Shift-z'] },
  lineBreak: { group: '쓰기', label: '문단 안에서 줄 바꾸기', keys: ['Shift-Enter'] },
  sceneBreak: { group: '쓰기', label: '장면 나눔 넣기', keys: ['Enter'], note: '빈 줄에 ***를 쓰고' },
  noBreakSpace: { group: '쓰기', label: '묶음 빈칸', keys: ['Mod-Shift-Space', 'Alt-Space'] },
  fixedSpace: { group: '쓰기', label: '고정폭 빈칸', keys: ['Alt-Shift-Space'] },
  symbols: { group: '쓰기', label: '문자표 (특수 문자)', keys: ['Mod-F10'] },
  note: { group: '쓰기', label: '고른 글에 메모 달기', keys: ['Mod-Alt-m'] },
  readAloud: { group: '쓰기', label: '소리 내어 읽기 / 멈춤', keys: ['Mod-Shift-r'] },
  focusMode: { group: '쓰기', label: '집중 모드 켜고 끄기', keys: ['F11'], note: 'Esc로도 나감' },

  // 서식
  bold: { group: '서식', label: '굵게', keys: ['Mod-b'] },
  italic: { group: '서식', label: '기울임', keys: ['Mod-i'] },
  underline: { group: '서식', label: '밑줄', keys: ['Mod-u'] },
  strike: { group: '서식', label: '취소선', keys: ['Mod-Shift-s'] },
  dot: { group: '서식', label: '방점', keys: ['Mod-Shift-d'] },
  marginIn: { group: '서식', label: '문단 여백 늘리기 (통째로 들이기)', keys: ['Mod-]'] },
  marginOut: { group: '서식', label: '문단 여백 줄이기', keys: ['Mod-['] },
  showMarks: { group: '서식', label: '빈칸·문단 부호 보이기', keys: ['Mod-Shift-8'] },
  format: { group: '서식', label: '원고 서식 (편집 용지)', keys: ['F7'] },

  // 찾기
  find: { group: '찾기', label: '이 문서에서 찾기', keys: ['Mod-f'] },
  findAll: { group: '찾기', label: '작품 전체에서 찾기', keys: ['Mod-Shift-f'] },
  replace: { group: '찾기', label: '바꾸기', keys: ['Mod-h'] },
  replaceAll: { group: '찾기', label: '작품 전체에서 바꾸기', keys: ['Mod-Shift-h'] },

  // 탭·창
  closeTab: { group: '탭·창', label: '탭 닫기', keys: ['Mod-w'] },
  reopenTab: { group: '탭·창', label: '닫은 탭 다시 열기', keys: ['Mod-Shift-t'] },
  nextTab: { group: '탭·창', label: '다음 탭', keys: ['Ctrl-Tab'] },
  prevTab: { group: '탭·창', label: '이전 탭', keys: ['Ctrl-Shift-Tab'] },
  back: { group: '탭·창', label: '탭에서 뒤로', keys: ['Alt-ArrowLeft'] },
  forward: { group: '탭·창', label: '탭에서 앞으로', keys: ['Alt-ArrowRight'] },
  split: { group: '탭·창', label: '나눠 보기 / 하나로', keys: ['Mod-\\'] },

  // 기타
  shortcuts: { group: '기타', label: '단축키 목록', keys: ['F1', 'Mod-/'] },
  escape: { group: '기타', label: '창 닫기 · 읽기 멈춤 · 집중 모드 나가기', keys: ['Escape'], note: '한글을 조합하는 중에는 조합만 취소' },
} satisfies Record<string, Shortcut>;

export type ShortcutId = keyof typeof SHORTCUTS;

/** One key combination taken apart. */
interface Combo {
  key: string;
  mod: boolean;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
}

function parse(spec: string): Combo {
  // The last part is the key; it can itself be "-" only as "Mod--", which no shortcut uses.
  const parts = spec.split('-');
  const key = parts.pop() ?? '';
  const has = (name: string) => parts.includes(name);
  return { key, mod: has('Mod'), ctrl: has('Ctrl'), shift: has('Shift'), alt: has('Alt') };
}

const KEY_NAMES: Record<string, string> = {
  ArrowLeft: '←',
  ArrowRight: '→',
  ArrowUp: '↑',
  ArrowDown: '↓',
  Escape: 'Esc',
};

/** "Mod-Shift-d" as the screen writes it: "Ctrl+Shift+D". */
export function comboText(spec: string, mac = false): string {
  const c = parse(spec);
  const out: string[] = [];
  if (c.mod) out.push(mac ? '⌘' : 'Ctrl');
  if (c.ctrl) out.push('Ctrl');
  if (c.alt) out.push(mac ? '⌥' : 'Alt');
  if (c.shift) out.push('Shift');
  out.push(KEY_NAMES[c.key] ?? (c.key.length === 1 ? c.key.toUpperCase() : c.key));
  return out.join('+');
}

/** A shortcut's keys for a tooltip: "Ctrl+Shift+D", or "F1 / Ctrl+/". */
export function keysText(id: ShortcutId): string {
  return SHORTCUTS[id].keys.map((k) => comboText(k)).join(' / ');
}

/** What a key handler needs of a key event. */
export type KeyLike = Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>;

function keyMatches(c: Combo, e: KeyLike): boolean {
  const k = c.key;
  // Letters and digits by the key's place, so a Korean keyboard layout or
  // Shift (Ctrl+Shift+8 gives "*") does not change them.
  if (/^[a-z]$/i.test(k)) return e.code === `Key${k.toUpperCase()}` || e.key.toLowerCase() === k.toLowerCase();
  if (/^[0-9]$/.test(k)) return e.code === `Digit${k}` || e.key === k;
  if (k === 'Space') return e.code === 'Space' || e.key === ' ';
  if (k === '/') return e.key === '/' || e.code === 'Slash';
  if (k === '\\') return e.key === '\\' || e.code === 'Backslash';
  return e.key === k;
}

function comboMatches(spec: string, e: KeyLike): boolean {
  const c = parse(spec);
  const mod = e.ctrlKey || e.metaKey;
  const wantMod = c.mod || c.ctrl;
  if (mod !== wantMod || e.shiftKey !== c.shift || e.altKey !== c.alt) return false;
  if (c.ctrl && !e.ctrlKey) return false;
  return keyMatches(c, e);
}

/** Whether a key press is one of a shortcut's combinations. */
export function matches(e: KeyLike, id: ShortcutId): boolean {
  return SHORTCUTS[id].keys.some((spec) => comboMatches(spec, e));
}

/** The first of `ids` the key press is, if any. */
export function shortcutOf<T extends ShortcutId>(e: KeyLike, ids: readonly T[]): T | null {
  return ids.find((id) => matches(e, id)) ?? null;
}

/** A shortcut's keys for a Tiptap `addKeyboardShortcuts` map. */
export function editorKeys<T>(id: ShortcutId, run: () => T): Record<string, () => T> {
  return Object.fromEntries(SHORTCUTS[id].keys.map((k) => [k, run]));
}

/** The table by group, in the list's order. */
export function shortcutsByGroup(): { group: ShortcutGroup; items: (Shortcut & { id: ShortcutId })[] }[] {
  const all = (Object.entries(SHORTCUTS) as [ShortcutId, Shortcut][]).map(([id, s]) => ({ ...s, id }));
  return SHORTCUT_GROUPS.map((group) => ({ group, items: all.filter((s) => s.group === group) }));
}
