// The menu on a word with a red wavy line (맞춤법 표시): what the writer may
// have meant, 사전에 넣기 and 무시. Opened by a right click or a long press
// in the editor (editor/spell.ts) and by ⋯ in the 맞춤법 panel.

import type { EditorView } from '@tiptap/pm/view';
import { menuShows, openMenuAt, type MenuItem } from '../components/Menu';
import { dropSpelling, replaceSpelling, type SpellHit, type SpellSetup } from '../editor/spell';
import { addSpellWord, checkSpelling, ignoreSpellWord, spellSuggestions } from '../store';

/** Items for a flagged word. `fix` puts a suggestion in; null when the text cannot change (locked). */
export function spellItems(
  word: string,
  suggested: string[] | null,
  fix: ((text: string) => void) | null,
  after?: () => void,
): MenuItem[] {
  const heading: MenuItem = {
    heading: suggested === null ? '고칠 말 찾는 중' : suggested.length ? '이렇게 고치기' : '고칠 말을 찾지 못했습니다',
  };
  const fixes: MenuItem[] = (suggested ?? []).map((s) => ({
    label: s,
    disabled: !fix,
    hint: fix ? undefined : '잠근 글',
    onSelect: () => fix?.(s),
  }));
  return [
    heading,
    ...fixes,
    { separator: true },
    {
      label: '사전에 넣기 (이 작품)',
      onSelect: () => {
        after?.();
        void addSpellWord(word, 'project');
      },
    },
    {
      label: '사전에 넣기 (이 기기의 모든 작품)',
      onSelect: () => {
        after?.();
        void addSpellWord(word, 'device');
      },
    },
    {
      label: '무시',
      hint: '앱을 닫을 때까지',
      onSelect: () => {
        after?.();
        ignoreSpellWord(word);
      },
    },
  ];
}

/** Opens the items at a point now, and again with suggestions once they come. */
export function openSpellMenuAt(x: number, y: number, word: string, fix: ((text: string) => void) | null, after?: () => void) {
  const first = spellItems(word, null, fix, after);
  openMenuAt(x, y, first, { title: word });
  void spellSuggestions(word).then((found) => {
    if (menuShows(first)) openMenuAt(x, y, spellItems(word, found, fix, after), { title: word });
  });
}

function openSpellMenu(view: EditorView, hit: SpellHit, event: MouseEvent) {
  openSpellMenuAt(
    event.clientX,
    event.clientY,
    hit.word,
    view.editable ? (text) => replaceSpelling(view, hit, text) : null,
    () => dropSpelling(view, hit.word),
  );
}

/** What the editor needs for 맞춤법 표시; one object, so turning on twice is noticed. */
export const SPELL_SETUP: SpellSetup = {
  check: checkSpelling,
  onMenu: openSpellMenu,
};
