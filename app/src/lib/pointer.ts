// Which kind of pointer the writer used last. Menus open as a bottom sheet
// after a finger or pen, and as a small menu next to the pointer after a mouse.

let last = 'mouse';

if (typeof window !== 'undefined') {
  window.addEventListener(
    'pointerdown',
    (e) => {
      last = e.pointerType || 'mouse';
    },
    { capture: true, passive: true },
  );
}

/** The last press came from a finger or a pen. */
export function touchLike(): boolean {
  return last === 'touch' || last === 'pen';
}

/** A screen used with fingers only (phones, tablets without a mouse). */
export function fingerScreen(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(hover: none) and (pointer: coarse)').matches;
}

/** The screen takes touch at all (also touch laptops). */
export function touchCapable(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(any-pointer: coarse)').matches;
}
