import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from 'react';
import { create } from 'zustand';

export type MenuItem =
  | { label: string; onSelect: () => void; danger?: boolean; checked?: boolean; disabled?: boolean }
  | { separator: true }
  | { heading: string };

interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

const useMenu = create<{ menu: MenuState | null }>(() => ({ menu: null }));

/** Opens a menu at the pointer (right click) or under the clicked button. */
export function openMenu(e: ReactMouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  let { clientX: x, clientY: y } = e;
  if (e.type === 'click') {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    x = rect.left;
    y = rect.bottom + 4;
  }
  useMenu.setState({ menu: { x, y, items } });
}

export function MenuHost() {
  const menu = useMenu((s) => s.menu);
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const close = () => useMenu.setState({ menu: null });

  useLayoutEffect(() => {
    if (!menu || !ref.current) return;
    const { width, height } = ref.current.getBoundingClientRect();
    setPos({
      x: Math.max(8, Math.min(menu.x, window.innerWidth - width - 8)),
      y: Math.max(8, Math.min(menu.y, window.innerHeight - height - 8)),
    });
    ref.current.querySelector<HTMLElement>('button:not(:disabled)')?.focus();
  }, [menu]);

  useEffect(() => {
    if (!menu) return;
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const buttons = [...(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled)') ?? [])];
        const i = buttons.indexOf(document.activeElement as HTMLElement);
        const next = e.key === 'ArrowDown' ? i + 1 : i - 1;
        buttons[(next + buttons.length) % buttons.length]?.focus();
      }
    };
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', close);
    };
  }, [menu]);

  if (!menu) return null;
  return (
    <div className="menu" role="menu" ref={ref} style={{ left: pos.x, top: pos.y }}>
      {menu.items.map((item, i) => {
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
              close();
              item.onSelect();
            }}
          >
            <span className="menu-check">{item.checked ? '✓' : ''}</span>
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
