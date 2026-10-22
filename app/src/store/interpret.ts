// 해석 모드: the writer clicks a sentence and the right column shows what it
// does and means and what it links to (작품 해석, docs/reading.md). The
// reading comes from an assistant through writer-mcp; the app only shows it.

import { api } from '../api';
import { get, set } from './state';
import { toastError } from './ui';

export function setInterpretMode(on: boolean) {
  if (get().interpretMode === on) return;
  set(on ? { interpretMode: true, rightOpen: true, rightTab: 'interpret' } : { interpretMode: false, interpretPick: null });
}

export function toggleInterpretMode() {
  setInterpretMode(!get().interpretMode);
}

/** Loads the open chapter's reading. A newer call wins over a slower older one. */
let asked = 0;
export async function loadInterpretation(docId: string) {
  const root = get().overview?.root;
  if (!root) return;
  const mine = ++asked;
  try {
    const view = await api.interpretLoad(root, docId);
    if (mine === asked) set({ interpret: { docId, view } });
  } catch (e) {
    if (mine === asked) set({ interpret: { docId, view: null } });
    toastError('작품 해석을 읽지 못함', e);
  }
}

export function pickSentence(docId: string, block: number, offset: number) {
  set({ interpretPick: { docId, block, offset } });
}
