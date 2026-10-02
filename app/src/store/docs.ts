// Chapters and parts in the tree.

import { api } from '../api';
import type { DocStatus, NewDoc } from '../api/types';
import { get } from './state';
import { allManuscript, findDoc, patchSummary, root, saveEverything, showToast, toastError } from './ui';
import { dropEverywhere, openTarget } from './tabs';
import { refreshOverview } from './project';

// ---------------------------------------------------------------------------
// Documents

/** Opens a document in the tab being worked in, or in a new tab. */
export function selectDoc(id: string, newTab = false) {
  return openTarget({ kind: 'doc', id }, { newTab });
}

export async function addDoc(spec: NewDoc) {
  try {
    if (!(await saveEverything())) return;
    const id = await api.docAdd(root(), spec);
    await refreshOverview();
    await selectDoc(id);
  } catch (e) {
    toastError('새 문서를 만들지 못함', e);
  }
}

export async function setStatus(id: string, status: DocStatus) {
  try {
    await api.docUpdateMeta(root(), id, { status });
    patchSummary(id, { status });
  } catch (e) {
    toastError('상태를 바꾸지 못함', e);
  }
}

export async function setTarget(id: string, target: number | null) {
  try {
    await api.docUpdateMeta(root(), id, { target });
    patchSummary(id, { target });
  } catch (e) {
    toastError('목표 분량을 바꾸지 못함', e);
  }
}

export async function renameDoc(id: string, title: string) {
  try {
    await api.docUpdateMeta(root(), id, { title });
    patchSummary(id, { title: title.trim() });
  } catch (e) {
    toastError('이름을 바꾸지 못함', e);
  }
}

export async function moveDoc(id: string, partId: string | null, index: number) {
  try {
    await api.docMove(root(), id, partId, index);
    await refreshOverview();
  } catch (e) {
    toastError('옮기지 못함', e);
  }
}

export async function trashDoc(id: string) {
  const ov = get().overview;
  if (!ov) return;
  try {
    if (!(await saveEverything())) return;
    // Its tabs close; a pane left empty shows a neighbour from the same list:
    // the next one, or the one before.
    const list = findDoc(ov, id)?.section === 'planning' ? ov.planning : allManuscript(ov);
    const i = list.findIndex((d) => d.id === id);
    const next = list[i + 1] ?? list[i - 1] ?? null;
    dropEverywhere((t) => t.kind === 'doc' && t.id === id, next ? { kind: 'doc', id: next.id } : null);
    const item = await api.docTrash(ov.root, id);
    await refreshOverview();
    showToast({
      text: '휴지통으로 옮김',
      action: {
        label: '되돌리기',
        run: () => {
          void (async () => {
            await api.trashRestore(ov.root, item.id);
            await refreshOverview();
          })();
        },
      },
    });
  } catch (e) {
    toastError('휴지통으로 옮기지 못함', e);
  }
}

// ---------------------------------------------------------------------------
// Parts

export async function addPart() {
  try {
    await api.partAdd(root(), '');
    await refreshOverview();
  } catch (e) {
    toastError('부를 만들지 못함', e);
  }
}

export async function renamePart(id: string, title: string) {
  try {
    await api.partRename(root(), id, title);
    await refreshOverview();
  } catch (e) {
    toastError('부 이름을 바꾸지 못함', e);
  }
}

export async function removePart(id: string) {
  try {
    await api.partRemove(root(), id);
    await refreshOverview();
  } catch (e) {
    toastError('부를 지우지 못함', e);
  }
}
