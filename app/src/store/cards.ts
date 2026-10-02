// 설정집 cards.

import { api } from '../api';
import type { CardSummary } from '../api/types';
import { get, root, set } from './state';
import { saveEverything, showToast, toastError } from './ui';
import { dropEverywhere, openTarget } from './tabs';
import { refreshOverview } from './project';

// ---------------------------------------------------------------------------
// Setting cards

export function previewCard(id: string | null) {
  set({ previewCardId: id, rightOpen: id ? true : get().rightOpen });
}

/** Opens a card for editing in a tab of its own. */
export async function openCard(id: string) {
  set({ previewCardId: null });
  await openTarget({ kind: 'card', id }, { newTab: true });
  void refreshCardCounts();
}

export async function refreshCardCounts() {
  try {
    const counts = await api.cardCounts(root());
    set({ cardCounts: Object.fromEntries(counts) });
  } catch {
    // Counts are a nicety; the list works without them.
  }
}

export async function createCard(typeId: string, name: string) {
  try {
    const card = await api.cardCreate(root(), typeId, name);
    await refreshOverview();
    await openCard(card.id);
  } catch (e) {
    toastError('카드를 만들지 못함', e);
  }
}

/** Replaces one card's line in the overview after it was saved. */
export function patchCardSummary(summary: CardSummary) {
  const ov = get().overview;
  if (!ov) return;
  const cards = ov.cards.some((c) => c.id === summary.id)
    ? ov.cards.map((c) => (c.id === summary.id ? summary : c))
    : [...ov.cards, summary];
  cards.sort((a, b) => a.name.localeCompare(b.name));
  set({ overview: { ...ov, cards } });
}

export async function trashCard(id: string) {
  const ov = get().overview;
  if (!ov) return;
  try {
    if (!(await saveEverything())) return;
    const item = await api.cardTrash(ov.root, id);
    dropEverywhere((t) => t.kind === 'card' && t.id === id, null);
    if (get().previewCardId === id) set({ previewCardId: null });
    await refreshOverview();
    showToast({
      text: '카드를 휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void api.trashRestore(ov.root, item.id).then(refreshOverview);
        },
      },
    });
  } catch (e) {
    toastError('휴지통으로 옮기지 못함', e);
  }
}
