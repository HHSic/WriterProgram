import { describe, expect, it } from 'vitest';
import { SHORTCUTS, SHORTCUT_GROUPS, comboText, editorKeys, keysText, matches, shortcutOf, shortcutsByGroup, type KeyLike, type ShortcutId } from './shortcuts';

/** A key press: "Ctrl+Shift+f" style, with the key and code a browser would give. */
function press(key: string, code: string, mods: Partial<Pick<KeyLike, 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>> = {}): KeyLike {
  return { key, code, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...mods };
}

const ids = Object.keys(SHORTCUTS) as ShortcutId[];

describe('the shortcut table', () => {
  it('gives no two things the same keys', () => {
    const seen = new Map<string, string>();
    for (const id of ids) {
      for (const k of SHORTCUTS[id].keys) {
        const normal = k.replace(/^Mod-/, 'Ctrl-').toLowerCase();
        expect(seen.get(normal), `${k} is taken by ${seen.get(normal)}`).toBeUndefined();
        seen.set(normal, id);
      }
    }
  });

  it('lists every shortcut once, in its group', () => {
    const listed = shortcutsByGroup();
    expect(listed.map((g) => g.group)).toEqual(SHORTCUT_GROUPS);
    expect(listed.flatMap((g) => g.items.map((s) => s.id)).sort()).toEqual([...ids].sort());
    for (const g of listed) expect(g.items.length).toBeGreaterThan(0);
  });

  it('writes keys the way the screen shows them', () => {
    expect(comboText('Mod-Shift-d')).toBe('Ctrl+Shift+D');
    expect(comboText('Alt-ArrowLeft')).toBe('Alt+←');
    expect(comboText('Mod-\\')).toBe('Ctrl+\\');
    expect(comboText('Escape')).toBe('Esc');
    expect(comboText('Mod-Alt-m', true)).toBe('⌘+⌥+M');
    expect(keysText('shortcuts')).toBe('F1 / Ctrl+/');
  });

  it('gives the editor the keys of the table', () => {
    const run = () => true;
    expect(Object.keys(editorKeys('noBreakSpace', run))).toEqual(['Mod-Shift-Space', 'Alt-Space']);
    expect(Object.keys(editorKeys('dot', run))).toEqual(['Mod-Shift-d']);
  });
});

describe('matching key presses', () => {
  it('finds letters by their place, also with a Korean layout', () => {
    // Ctrl+F with the 한글 input on gives "ㄹ" as the key.
    expect(matches(press('ㄹ', 'KeyF', { ctrlKey: true }), 'find')).toBe(true);
    expect(matches(press('F', 'KeyF', { ctrlKey: true, shiftKey: true }), 'find')).toBe(false);
    expect(matches(press('F', 'KeyF', { ctrlKey: true, shiftKey: true }), 'findAll')).toBe(true);
    // ⌘ on a Mac counts as Ctrl.
    expect(matches(press('s', 'KeyS', { metaKey: true }), 'save')).toBe(true);
  });

  it('finds digits under Shift, and the other keys by name', () => {
    expect(matches(press('*', 'Digit8', { ctrlKey: true, shiftKey: true }), 'showMarks')).toBe(true);
    expect(matches(press('F11', 'F11'), 'focusMode')).toBe(true);
    expect(matches(press('F11', 'F11', { ctrlKey: true }), 'focusMode')).toBe(false);
    expect(matches(press('/', 'Slash', { ctrlKey: true }), 'shortcuts')).toBe(true);
    expect(matches(press('F1', 'F1'), 'shortcuts')).toBe(true);
    expect(matches(press('\\', 'Backslash', { ctrlKey: true }), 'split')).toBe(true);
    expect(matches(press('Tab', 'Tab', { ctrlKey: true, shiftKey: true }), 'prevTab')).toBe(true);
    expect(matches(press('Tab', 'Tab', { ctrlKey: true, shiftKey: true }), 'nextTab')).toBe(false);
    expect(matches(press('Escape', 'Escape'), 'escape')).toBe(true);
  });

  it('picks the one shortcut a press is among those handled', () => {
    expect(shortcutOf(press('h', 'KeyH', { ctrlKey: true, shiftKey: true }), ['find', 'findAll', 'replace', 'replaceAll'])).toBe('replaceAll');
    expect(shortcutOf(press('a', 'KeyA'), ['find', 'save'])).toBeNull();
  });
});
