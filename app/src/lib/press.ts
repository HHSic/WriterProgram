// Right click, long press (finger or pen) and a ⋯ button open the same menu,
// so everything done with a right click can be done with a finger too. In a
// list whose order can change, a long press that then moves becomes a drag,
// and the menu opens only if the finger lifts without moving.

import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from 'react';
import { openMenu, openMenuAt, type MenuItem, type MenuOptions } from '../components/Menu';

const HOLD_MS = 500;
/** Moving further than this (px) before the hold is a scroll, not a press. */
const SLOP = 10;

/** Moving something with a finger, after a long press. */
export interface TouchDrag {
  start: (x: number, y: number) => void;
  move: (x: number, y: number) => void;
  /** `cancelled` when the system took the gesture over. */
  end: (x: number, y: number, cancelled: boolean) => void;
}

interface Press {
  pointerId: number;
  x: number;
  y: number;
  timer: ReturnType<typeof setTimeout>;
  armed: boolean;
  dragging: boolean;
  el: HTMLElement;
  items: () => MenuItem[];
  opts?: () => MenuOptions;
  drag?: TouchDrag;
}

let press: Press | null = null;
/** When a press last opened a menu or ended a drag. The click and the system
 * context menu that follow it are not new requests. */
let handledAt = 0;

function stop() {
  if (!press) return;
  clearTimeout(press.timer);
  press.el.classList.remove('pressing');
  press = null;
}

function openFromPress(p: Press, x: number, y: number) {
  handledAt = Date.now();
  openMenuAt(x, y, p.items(), { ...p.opts?.(), sheet: true });
}

if (typeof window !== 'undefined') {
  // While a held finger moves something, the page must not scroll.
  window.addEventListener(
    'touchmove',
    (e) => {
      if (press?.armed && press.drag) e.preventDefault();
    },
    { passive: false },
  );
  window.addEventListener('pointermove', (e) => {
    const p = press;
    if (!p || e.pointerId !== p.pointerId) return;
    const moved = Math.hypot(e.clientX - p.x, e.clientY - p.y) > SLOP;
    if (!p.armed) {
      if (moved) stop();
      return;
    }
    if (!p.drag) return;
    if (!p.dragging && moved) {
      p.dragging = true;
      p.drag.start(e.clientX, e.clientY);
    }
    if (p.dragging) p.drag.move(e.clientX, e.clientY);
  });
  window.addEventListener('pointerup', (e) => {
    const p = press;
    if (!p || e.pointerId !== p.pointerId) return;
    if (p.armed && p.dragging) {
      handledAt = Date.now();
      p.drag?.end(e.clientX, e.clientY, false);
    } else if (p.armed) {
      openFromPress(p, e.clientX, e.clientY);
    }
    stop();
  });
  window.addEventListener('pointercancel', (e) => {
    const p = press;
    if (!p || e.pointerId !== p.pointerId) return;
    if (p.dragging) p.drag?.end(e.clientX, e.clientY, true);
    stop();
  });
}

/** Handlers that open `items` on right click and on a long press. */
export function pressMenu(items: () => MenuItem[], opts?: () => MenuOptions, drag?: TouchDrag) {
  return {
    onContextMenu(e: ReactMouseEvent) {
      e.preventDefault();
      if (Date.now() - handledAt < 1500) return;
      const p = press;
      if (p && p.el === e.currentTarget) {
        // The system saw a long press first (Android, Windows touch).
        if (p.drag) {
          // Wait for the finger: it may still move to drag.
          clearTimeout(p.timer);
          p.armed = true;
          p.el.classList.add('pressing');
          return;
        }
        stop();
      }
      handledAt = Date.now();
      openMenu(e, items(), opts?.());
    },
    onPointerDown(e: ReactPointerEvent) {
      if (e.pointerType === 'mouse' || !e.isPrimary) return;
      stop();
      const el = e.currentTarget as HTMLElement;
      const next: Press = {
        pointerId: e.pointerId,
        x: e.clientX,
        y: e.clientY,
        armed: false,
        dragging: false,
        el,
        items,
        opts,
        drag,
        timer: setTimeout(() => {
          if (press !== next) return;
          next.armed = true;
          el.classList.add('pressing');
          navigator.vibrate?.(10);
          if (!next.drag) {
            // Nothing to drag: show the menu while the finger is still down.
            openFromPress(next, next.x, next.y);
            stop();
          }
        }, HOLD_MS),
      };
      press = next;
    },
    onClickCapture(e: ReactMouseEvent) {
      if (Date.now() - handledAt < 700) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
  };
}
