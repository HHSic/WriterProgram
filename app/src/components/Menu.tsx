// Menus: a small menu next to the pointer for the mouse, or a sheet from the
// bottom of the screen after a finger (right click, long press and ⋯ buttons
// all open the same items; see lib/press.ts).

import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from 'react';
import { create } from 'zustand';
import { fingerScreen, touchLike } from '../lib/pointer';

export type MenuItem =
  | { label: string; onSelect: () => void; danger?: boolean; checked?: boolean; disabled?: boolean }
  | { separator: true }
  | { heading: string };

export interface MenuOptions {
  /** What the menu is for, shown at the top of the sheet. */
  title?: string;
  /** A line under the title (a synopsis, a summary). */
  subtitle?: string;
  /** Force the sheet (or the small menu). */
  sheet?: boolean;
}

interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
  sheet: boolean;
  title?: string;
  subtitle?: string;
  openedAt: number;
}

const useMenu = create<{ menu: MenuState | null }>(() => ({ menu: null }));

/** Whether a menu is open (the in-app browser hides under it). */
export function useMenuOpen(): boolean {
  return useMenu((s) => s.menu !== null);
}

/** Opens a menu at a point; after a finger it becomes a sheet. */
export function openMenuAt(x: number, y: number, items: MenuItem[], opts: MenuOptions = {}) {
  const sheet = opts.sheet ?? (touchLike() || fingerScreen());
  useMenu.setState({ menu: { x, y, items, sheet, title: opts.title, subtitle: opts.subtitle, openedAt: Date.now() } });
}

/** Opens a menu at the pointer (right click) or under the clicked button. */
export function openMenu(e: ReactMouseEvent, items: MenuItem[], opts: MenuOptions = {}) {
  e.preventDefault();
  e.stopPropagation();
  let { clientX: x, clientY: y } = e;
  if (e.type === 'click') {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    x = rect.left;
    y = rect.bottom + 4;
  }
  openMenuAt(x, y, items, opts);
}

export function closeMenu() {
  useMenu.setState({ menu: null });
}

export function MenuHost() {
  const menu = useMenu((s) => s.menu);
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: 0, y: 0 });

  useLayoutEffect(() => {
    if (!menu || !ref.current) return;
    if (!menu.sheet) {
      const { width, height } = ref.current.getBoundingClientRect();
      setPos({
        x: Math.max(8, Math.min(menu.x, window.innerWidth - width - 8)),
        y: Math.max(8, Math.min(menu.y, window.innerHeight - height - 8)),
      });
      ref.current.querySelector<HTMLElement>('button:not(:disabled)')?.focus();
    }
  }, [menu]);

  useEffect(() => {
    if (!menu) return;
    // A press outside closes it (not a click: the release of the long press
    // that opened a sheet lands outside it).
    const onDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) closeMenu();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeMenu();
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const buttons = [...(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled)') ?? [])];
        const i = buttons.indexOf(document.activeElement as HTMLElement);
        const next = e.key === 'ArrowDown' ? i + 1 : i - 1;
        buttons[(next + buttons.length) % buttons.length]?.focus();
      }
    };
    window.addEventListener('pointerdown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', closeMenu);
    return () => {
      window.removeEventListener('pointerdown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', closeMenu);
    };
  }, [menu]);

  if (!menu) return null;

  const items = menu.items.map((item, i) => {
    if ('separator' in item) return <div key={i} className="menu-sep" role="separator" />;
    if ('heading' in item) return <div key={i} className="menu-heading">{item.heading}</div>;
    return (
      <button
        key={i}
        type="button"
        role="menuitem"
        className={`menu-item${item.danger ? ' danger' : ''}`}
        disabled={item.disabled}
        onClick={() => {
          // The release of the press that opened the sheet is not a choice.
          if (menu.sheet && Date.now() - menu.openedAt < 350) return;
          closeMenu();
          item.onSelect();
        }}
      >
        <span className="menu-check">{item.checked ? '✓' : ''}</span>
        {item.label}
      </button>
    );
  });

  if (menu.sheet) {
    return (
      <div className="menu-sheet-backdrop">
        <div className="menu-sheet" role="menu" aria-label={menu.title} ref={ref}>
          {(menu.title || menu.subtitle) && (
            <div className="menu-sheet-head">
              {menu.title && <strong>{menu.title}</strong>}
              {menu.subtitle && <p>{menu.subtitle}</p>}
            </div>
          )}
          <div className="menu-sheet-items">{items}</div>
          <button type="button" className="menu-sheet-close" onClick={closeMenu}>
            닫기
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="menu" role="menu" ref={ref} style={{ left: pos.x, top: pos.y }}>
      {items}
    </div>
  );
}
