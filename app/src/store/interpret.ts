// 해석 모드: the writer clicks a sentence and the right column shows what it
// does and means and what it links to (작품 해석, docs/reading.md). The
// reading comes from an assistant through writer-mcp; the writer can change
// it here, add sentences and link them, also into other chapters.

import { api } from '../api';
import type { Interpretation, InterpretIncoming, InterpretSentence } from '../api/types';
import { get, set, type InterpretLinking, type InterpretTarget } from './state';
import { showToast, toastError } from './ui';

export function setInterpretMode(on: boolean) {
  if (get().interpretMode === on) return;
  set(
    on
      ? { interpretMode: true, rightOpen: true, rightTab: 'interpret' }
      : { interpretMode: false, interpretPick: null, interpretLinking: null },
  );
}

export function toggleInterpretMode() {
  setInterpretMode(!get().interpretMode);
}

/** Loads the open chapter's reading. A newer call (or save) wins over a slower older one. */
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

export function pickSentence(docId: string, block: number, offset: number, sentence?: InterpretTarget | null) {
  set({ interpretPick: { docId, block, offset, sentence } });
}

/** An empty reading of `docId`, for the first sentence the writer adds. */
export function emptyReading(docId: string): Interpretation {
  return { doc: docId, rev: '', made: '', by: '', summary: '', scenes: [], sentences: [] };
}

/** The loaded reading of `docId`, copied to be changed. */
function loaded(docId: string): Interpretation {
  const now = get().interpret;
  const reading = now?.docId === docId && now.view ? now.view.reading : emptyReading(docId);
  return structuredClone(reading);
}

/** Saves the writer's changes; shows the reading again when it is the open chapter's. */
async function keep(reading: Interpretation): Promise<boolean> {
  const root = get().overview?.root;
  if (!root) return false;
  const mine = ++asked;
  try {
    const view = await api.interpretSave(root, reading);
    if (mine === asked && get().interpret?.docId === reading.doc) set({ interpret: { docId: reading.doc, view } });
    return true;
  } catch (e) {
    toastError('해석을 저장하지 못함', e);
    return false;
  }
}

/** Changes one sentence's reading, or adds it when the reading has no such id. */
export async function saveSentence(docId: string, sentence: InterpretSentence): Promise<boolean> {
  const reading = loaded(docId);
  const at = reading.sentences.findIndex((s) => s.id === sentence.id);
  if (at < 0) reading.sentences.push(sentence);
  else reading.sentences[at] = sentence;
  return keep(reading);
}

/** A sentence reading for `target`, not saved yet. */
export function newSentence(docId: string, target: InterpretTarget): InterpretSentence {
  return { id: freeId(loaded(docId)), block: target.block, quote: target.text, roles: [], meaning: '', intent: '', links: [] };
}

function freeId(reading: Interpretation): string {
  const used = new Set(reading.sentences.map((s) => s.id));
  let n = reading.sentences.length + 1;
  while (used.has(`w${n}`)) n += 1;
  return `w${n}`;
}

/** Drops a sentence's reading and the links in this chapter that point at it. */
export async function removeSentence(docId: string, id: string): Promise<boolean> {
  const reading = loaded(docId);
  reading.sentences = reading.sentences
    .filter((s) => s.id !== id)
    .map((s) => ({ ...s, links: s.links.filter((l) => l.doc || l.to !== id) }));
  return keep(reading);
}

/** Drops link `index` of a sentence. */
export async function removeLink(docId: string, sentenceId: string, index: number): Promise<boolean> {
  const reading = loaded(docId);
  const s = reading.sentences.find((x) => x.id === sentenceId);
  if (!s) return false;
  s.links.splice(index, 1);
  return keep(reading);
}

/** Drops a link another chapter's reading makes into `docId`. That reading is changed. */
export async function removeIncoming(docId: string, link: InterpretIncoming): Promise<boolean> {
  const root = get().overview?.root;
  if (!root) return false;
  const source = (await api.interpretLoad(root, link.doc).catch(() => null))?.reading;
  const s = source?.sentences.find((x) => x.id === link.from);
  if (!source || !s) return false;
  s.links = s.links.filter((l) => !(l.doc === docId && l.quote === link.quote && l.kind === link.kind));
  const ok = await keep(source);
  if (ok) await loadInterpretation(docId);
  return ok;
}

/** Starts 연결 추가 from a sentence of `docId`'s reading. */
export function startLinking(docId: string, sentence: InterpretSentence) {
  set({ interpretLinking: { fromDoc: docId, fromId: sentence.id, fromQuote: sentence.quote, kind: '회수', note: '' } });
}

export function setLinking(patch: Partial<Pick<InterpretLinking, 'kind' | 'note'>>) {
  const now = get().interpretLinking;
  if (now) set({ interpretLinking: { ...now, ...patch } });
}

export function cancelLinking() {
  set({ interpretLinking: null });
}

/**
 * Links the sentence 연결 추가 started from to `target` in chapter `docId`.
 * In the same chapter the target gets a reading of its own if it had none
 * (links point at sentence ids); in another chapter the link carries that
 * chapter and the sentence's words, and shows there as a link coming in.
 */
export async function finishLinking(docId: string, target: InterpretTarget): Promise<boolean> {
  const linking = get().interpretLinking;
  const root = get().overview?.root;
  if (!linking || !root) return false;
  const same = docId === linking.fromDoc;
  const source = same
    ? loaded(docId)
    : (await api.interpretLoad(root, linking.fromDoc).catch(() => null))?.reading ?? null;
  const from = source?.sentences.find((s) => s.id === linking.fromId);
  if (!source || !from) {
    showToast({ text: '연결을 시작한 문장의 해석이 없어졌습니다.' });
    cancelLinking();
    return false;
  }
  const note = linking.note.trim();
  if (same) {
    const places = get().interpret?.docId === docId ? (get().interpret?.view?.places ?? []) : [];
    let to = places.find((p) => p.block === target.block && p.start < target.end && target.start < p.end)?.id ?? null;
    if (to === from.id) {
      showToast({ text: '같은 문장끼리는 이을 수 없습니다. 다른 문장을 고르세요.' });
      return false;
    }
    if (!to) {
      to = freeId(source);
      source.sentences.push({ id: to, block: target.block, quote: target.text, roles: [], meaning: '', intent: '', links: [] });
    }
    if (!from.links.some((l) => !l.doc && l.to === to && l.kind === linking.kind)) {
      from.links.push({ to, kind: linking.kind, note });
    }
  } else if (!from.links.some((l) => l.doc === docId && l.quote === target.text && l.kind === linking.kind)) {
    from.links.push({ to: '', kind: linking.kind, note, doc: docId, quote: target.text });
  }
  const ok = await keep(source);
  if (!ok) return false;
  cancelLinking();
  if (!same) await loadInterpretation(docId);
  return true;
}
