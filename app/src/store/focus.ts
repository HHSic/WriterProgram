// 집중 모드 (docs/screens.md S6): only the page, centred, without the side
// columns, tab strip and bars. How it looks (dimming other paragraphs,
// typewriter scrolling) is in 보기 설정; editor/focus.ts does both.

import { get, set } from './state';

export function setFocusMode(on: boolean) {
  if (get().focusMode !== on) set({ focusMode: on });
}

export function toggleFocusMode() {
  setFocusMode(!get().focusMode);
}
