// 문자표 (Ctrl+F10, as in 한글): special characters and spaces, put in where
// the cursor was when it opened — the manuscript, or a text box such as a
// card field or a note.

import { useMemo, useState } from 'react';
import type { Editor } from '@tiptap/core';
import { Modal } from '../components/Modal';
import { FIXED_SPACE, FULL_WIDTH_SPACE, NO_BREAK_SPACE } from '../editor/paragraph';
import { closeDialog, openDialog, useApp } from '../store';

const RECENT_KEY = 'wp.recentSymbols';
const RECENT_LIMIT = 24;

function range(from: number, to: number, skip: number[] = []): string[] {
  const out: string[] = [];
  for (let c = from; c <= to; c += 1) if (!skip.includes(c)) out.push(String.fromCodePoint(c));
  return out;
}

const chars = (s: string) => [...s.replace(/\s+/g, '')];

interface Space {
  char: string;
  name: string;
  about: string;
}

const SPACES: Space[] = [
  { char: NO_BREAK_SPACE, name: '묶음 빈칸', about: '앞뒤 낱말이 줄 끝에서 떨어지지 않습니다 (Ctrl+Shift+Space)' },
  { char: FIXED_SPACE, name: '고정폭 빈칸', about: '양쪽 맞춤에서도 너비가 늘지 않습니다 (Alt+Shift+Space)' },
  { char: FULL_WIDTH_SPACE, name: '전각 빈칸', about: '한 글자만큼 넓은 빈칸' },
];

const GROUPS: { id: string; name: string; chars: string[] }[] = [
  { id: 'punct', name: '문장 부호', chars: chars('“ ” ‘ ’ 「 」 『 』 《 》 〈 〉 … ‥ · ・ ― — – ～ 〃 ※ ‼ ⁉ ¿ ¡ ˝ ′ ″ ° ˚') },
  { id: 'brackets', name: '괄호', chars: chars('（ ） ［ ］ ｛ ｝ 【 】 〔 〕 〖 〗 〘 〙 〚 〛 ｢ ｣ ⟨ ⟩') },
  { id: 'shapes', name: '도형', chars: chars('○ ● ◎ ◇ ◆ □ ■ △ ▲ ▽ ▼ ◁ ◀ ▷ ▶ ☆ ★ ♡ ♥ ♧ ♣ ♤ ♠ ♢ ◈ ▣ ◐ ◑ ▒ ▤ ▥ ▨ ▧ ▦ ▩ ◉ ⊙') },
  { id: 'arrows', name: '화살표', chars: chars('→ ← ↑ ↓ ↔ ↕ ↗ ↘ ↙ ↖ ⇒ ⇐ ⇑ ⇓ ⇔ ➔ ➜ ↻ ↺') },
  {
    id: 'numbers',
    name: '원·괄호 숫자',
    chars: [
      ...range(0x2460, 0x2473), // ① – ⑳
      ...range(0x2474, 0x2487), // ⑴ – ⒇
      ...range(0x2160, 0x216b), // Ⅰ – Ⅻ
      ...range(0x2170, 0x217b), // ⅰ – ⅻ
      ...range(0x3260, 0x326d), // ㉠ – ㉭
      ...range(0x326e, 0x327b), // ㉮ – ㉻
      ...range(0x3200, 0x320d), // ㈀ – ㈍
      ...range(0x320e, 0x321b), // ㈎ – ㈛
      ...range(0x249c, 0x24b5), // ⒜ – ⒵
    ],
  },
  { id: 'misc', name: '음표·기타', chars: chars('♪ ♩ ♫ ♬ ♭ ♯ ☀ ☁ ☂ ☃ ☎ ☏ ☜ ☞ ✓ ✔ ✕ ✖ ✗ † ‡ § ¶ © ® ™ ℡ № ♂ ♀ ∞ ∴ ∵ ≒ ≠ ≤ ≥ ± × ÷ √ ∑ ∫') },
  { id: 'units', name: '단위·돈', chars: chars('㎜ ㎝ ㎞ ㎡ ㎥ ㎖ ℓ ㎏ ㎎ ㎐ ℃ ℉ ％ ‰ ₩ ＄ ￥ € £ ¢') },
  { id: 'greek', name: '그리스 문자', chars: [...range(0x391, 0x3a9, [0x3a2]), ...range(0x3b1, 0x3c9)] },
];

function loadRecent(): string[] {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    return Array.isArray(raw) ? raw.filter((c): c is string => typeof c === 'string').slice(0, RECENT_LIMIT) : [];
  } catch {
    return [];
  }
}

function remember(char: string): string[] {
  const list = [char, ...loadRecent().filter((c) => c !== char)].slice(0, RECENT_LIMIT);
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    // Not critical.
  }
  return list;
}

/** Where the characters go: set when the dialog opens. */
let target: { editor: Editor | null; field: HTMLInputElement | HTMLTextAreaElement | null } = { editor: null, field: null };

export function openSymbols() {
  const active = document.activeElement;
  const field =
    active instanceof HTMLTextAreaElement || (active instanceof HTMLInputElement && ['text', 'search', ''].includes(active.type))
      ? active
      : null;
  target = { editor: useApp.getState().editor, field };
  openDialog({ kind: 'symbols' });
}

function insertInto(text: string): boolean {
  const { editor, field } = target;
  if (field?.isConnected && !field.readOnly && !field.disabled) {
    const start = field.selectionStart ?? field.value.length;
    const end = field.selectionEnd ?? start;
    const value = field.value.slice(0, start) + text + field.value.slice(end);
    // React keeps its own copy of the value: go through the native setter so
    // its change handler sees the new text.
    const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), 'value')?.set;
    setter?.call(field, value);
    field.dispatchEvent(new Event('input', { bubbles: true }));
    field.setSelectionRange(start + text.length, start + text.length);
    return true;
  }
  if (editor && !editor.isDestroyed && editor.isEditable) {
    editor.view.dispatch(editor.state.tr.insertText(text));
    return true;
  }
  return false;
}

function finish() {
  closeDialog();
  const { editor, field } = target;
  setTimeout(() => {
    if (field?.isConnected) field.focus();
    else if (editor && !editor.isDestroyed) editor.commands.focus();
  }, 0);
}

export function SymbolsDialog() {
  const [recent, setRecent] = useState(loadRecent);
  const [group, setGroup] = useState(() => (loadRecent().length ? 'recent' : GROUPS[0].id));
  const [last, setLast] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const shown = useMemo(
    () => (group === 'recent' ? recent : (GROUPS.find((g) => g.id === group)?.chars ?? [])),
    [group, recent],
  );

  const put = (char: string) => {
    if (!insertInto(char)) {
      setFailed(true);
      return;
    }
    setFailed(false);
    setLast(char);
    setRecent(remember(char));
  };

  return (
    <Modal
      title="문자표"
      onClose={finish}
      width={620}
      footer={
        <button type="button" className="btn primary" onClick={finish}>
          닫기
        </button>
      }
    >
      <div className="symbols">
        <nav className="symbol-groups" aria-label="문자 묶음">
          {recent.length > 0 && (
            <button type="button" className={group === 'recent' ? 'on' : ''} onClick={() => setGroup('recent')}>
              최근 쓴 문자
            </button>
          )}
          {GROUPS.map((g) => (
            <button key={g.id} type="button" className={group === g.id ? 'on' : ''} onClick={() => setGroup(g.id)}>
              {g.name}
            </button>
          ))}
          <button type="button" className={group === 'spaces' ? 'on' : ''} onClick={() => setGroup('spaces')}>
            빈칸
          </button>
        </nav>
        <div className="symbol-body">
          {group === 'spaces' ? (
            <ul className="space-list">
              {SPACES.map((s) => (
                <li key={s.name}>
                  <button type="button" className="btn" onClick={() => put(s.char)}>
                    {s.name}
                  </button>
                  <span className="hint">{s.about}</span>
                </li>
              ))}
            </ul>
          ) : (
            <div className="symbol-grid" role="group" aria-label="문자">
              {shown.map((c) => {
                const space = SPACES.find((s) => s.char === c);
                return (
                  <button
                    key={c}
                    type="button"
                    className={`symbol${space ? ' space' : ''}`}
                    title={space ? space.name : `U+${c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0')}`}
                    onClick={() => put(c)}
                  >
                    {space ? space.name.replace(' 빈칸', '') : c}
                  </button>
                );
              })}
            </div>
          )}
          <p className={`symbol-note${failed ? ' warn-text' : ''}`}>
            {failed
              ? '넣을 곳이 없습니다. 원고나 입력칸에 커서를 두고 다시 여세요.'
              : last
                ? `넣음: ${SPACES.find((s) => s.char === last)?.name ?? last} · 이어서 더 넣을 수 있습니다.`
                : '누르면 커서 자리에 들어갑니다. 여러 개를 넣고 닫으세요.'}
          </p>
        </div>
      </div>
    </Modal>
  );
}
