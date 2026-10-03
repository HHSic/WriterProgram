// Whether a dialog's choices differ from what it opened with, for Modal's
// `dirty` (ask "고친 내용을 버릴까요?" before closing).

import { useRef } from 'react';

function keyOf(value: unknown): string {
  return JSON.stringify(value, (_k, v: unknown) => (v instanceof Set ? [...v] : v));
}

/** True once `value` (plain data, Sets allowed) differs from its first render. */
export function useChanged(value: unknown): boolean {
  const key = keyOf(value);
  const first = useRef(key);
  return key !== first.current;
}
