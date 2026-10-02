// AI with the writer's own API key: this device's settings, asking the AI
// tab for a request, and putting a summary into a chapter's synopsis. The
// AI never changes the manuscript; only the writer's click puts a summary in.

import { api } from '../api';
import type { AiProvider, AiSettings, AiSettingsPatch, AiTask } from '../api/types';
import { get, root, set } from './state';
import { selectDoc } from './docs';
import { patchSummary, toastError } from './ui';

/** Reads this device's AI settings (when a project opens, and after changes). */
export async function loadAi() {
  try {
    set({ ai: await api.aiSettings() });
  } catch {
    // Without settings AI stays off; the settings dialog reads them again.
  }
}

/** AI is on and the chosen company has a key. */
export function aiReady(ai: AiSettings | null = get().ai): boolean {
  if (!ai?.enabled) return false;
  return ai.companies.some((c) => c.provider === ai.provider && c.hasKey);
}

export async function updateAi(patch: AiSettingsPatch): Promise<AiSettings | null> {
  try {
    const ai = await api.aiSettingsSet(patch);
    set({ ai });
    return ai;
  } catch (e) {
    toastError('AI 설정을 바꾸지 못함', e);
    return null;
  }
}

/** Keeps the key in the system's credential store. Throws the reason. */
export async function setAiKey(provider: AiProvider, key: string): Promise<AiSettings> {
  const ai = await api.aiKeySet(provider, key);
  set({ ai });
  return ai;
}

export async function removeAiKey(provider: AiProvider) {
  try {
    set({ ai: await api.aiKeyRemove(provider) });
  } catch (e) {
    toastError('키를 지우지 못함', e);
  }
}

/** Opens the AI tab with a request ready to preview (nothing is sent yet). */
export async function askAi(task: AiTask, docIds: string[]) {
  if (!docIds.length) return;
  if (get().activeDocId !== docIds[0]) await selectDoc(docIds[0]);
  set({ rightOpen: true, rightTab: 'ai', previewCardId: null, aiRequest: { task, docIds, nonce: Date.now() } });
}

/** Puts a summary into the chapter's synopsis (the writer's click). */
export async function putSynopsis(docId: string, synopsis: string): Promise<boolean> {
  try {
    await api.docUpdateMeta(root(), docId, { synopsis });
    patchSummary(docId, { synopsis });
    return true;
  } catch (e) {
    toastError('시놉시스에 넣지 못함', e);
    return false;
  }
}
