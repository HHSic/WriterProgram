// 맞춤법 검사: the dictionary on this device (fetched once, after the writer
// agrees), the words the writer accepts (setting card names and aliases,
// 이 작품 사전, 이 기기 사전, words ignored for now) and checking for the
// editor (editor/spell.ts) and the 맞춤법 panel (workspace/SpellTab.tsx).

import { api } from '../api';
import type { CardSummary, SpellPlace, SpellStatus, SpellWords } from '../api/types';
import { sizeText } from '../lib/format';
import { withObject } from '../lib/labels';
import { get, set } from './state';
import { openDialog, showToast, toastError } from './ui';

/** The open project's folder; empty on the start screen (only this device's list then). */
function projectRoot(): string {
  return get().overview?.root ?? '';
}

/** Words accepted besides the dictionary's, sent with every check. */
export function spellKnown(): string[] {
  const s = get();
  return knownWords(s.overview?.cards ?? [], s.spellWords, s.spellIgnored);
}

export function knownWords(cards: CardSummary[], words: SpellWords, ignored: string[]): string[] {
  const out = new Set<string>();
  for (const card of cards) for (const name of [card.name, ...card.aliases]) if (name.trim()) out.add(name.trim());
  for (const w of [...words.project, ...words.device, ...ignored]) out.add(w);
  return [...out].sort();
}

/** Changes whenever the accepted words do (editors check again). */
export function spellKnownKey(cards: CardSummary[], words: SpellWords, ignored: string[]): string {
  return knownWords(cards, words, ignored).join('\n');
}

/** Whether lines show in the editor: turned on and the dictionary is here. */
export function spellingOn(s: ReturnType<typeof get>): boolean {
  return s.view.spellcheck && !!s.spellStatus?.installed;
}

export async function loadSpell() {
  try {
    const status = await api.spellStatus();
    set({ spellStatus: status });
    set({ spellWords: await api.spellWords(projectRoot()) });
  } catch {
    // Spelling is extra: without it the rest works as before.
  }
}

/** What the writer agrees to before anything is fetched. */
export function downloadText(status: SpellStatus): string {
  return `맞춤법 사전을 받습니다 · 약 ${sizeText(status.downloadBytes)} · GPL 라이선스`;
}

/** Asks first, then fetches the dictionary (the only time spelling uses the internet). */
export function offerSpellDictionary() {
  const status = get().spellStatus;
  if (!status || status.installed) return;
  openDialog({
    kind: 'confirm',
    title: '맞춤법 사전 받기',
    message: `${downloadText(status)}\n\n한국어 맞춤법 사전(hunspell-dict-ko ${status.version})을 GitHub에서 한 번 받아 이 기기에 둡니다(풀면 약 ${sizeText(status.installedBytes)}). 사전은 앱과 따로 배포되는 자료로 GPL 3.0을 따르고, 낱말 자료는 국립국어원 사전 자료를 포함한 CC BY-SA 4.0입니다. 라이선스 파일도 함께 둡니다. 원고는 어디에도 보내지 않고, 검사는 이 기기 안에서만 합니다.`,
    confirm: '받기',
    onConfirm: () => void fetchSpellDictionary(),
  });
}

async function fetchSpellDictionary() {
  set({ busy: '맞춤법 사전을 받는 중' });
  try {
    await api.spellDownload();
    set({ spellStatus: await api.spellStatus() });
    showToast({ text: '맞춤법 사전을 받았습니다 · 틀린 곳에 빨간 물결 밑줄이 보입니다' });
  } catch (e) {
    toastError('맞춤법 사전을 받지 못함', e);
  } finally {
    set({ busy: null });
  }
}

/** Takes the dictionary off this device. */
export function removeSpellDictionary() {
  openDialog({
    kind: 'confirm',
    title: '맞춤법 사전 지우기',
    message: '이 기기에서 맞춤법 사전을 지웁니다. 사전에 넣은 낱말은 그대로 둡니다. 다시 쓰려면 사전을 다시 받아야 합니다.',
    confirm: '지우기',
    danger: true,
    onConfirm: async () => {
      try {
        await api.spellRemove();
        set({ spellStatus: await api.spellStatus() });
      } catch (e) {
        toastError('맞춤법 사전을 지우지 못함', e);
      }
    },
  });
}

/** 사전에 넣기: this work's list, or this device's. */
export async function addSpellWord(word: string, place: SpellPlace = 'project') {
  try {
    set({ spellWords: await api.spellWordAdd(projectRoot(), word, place) });
    showToast({ text: `${withObject(word)} ${place === 'device' ? '이 기기' : '이 작품'} 사전에 넣었습니다` });
  } catch (e) {
    toastError('사전에 넣지 못함', e);
  }
}

export async function removeSpellWord(word: string, place: SpellPlace) {
  try {
    set({ spellWords: await api.spellWordRemove(projectRoot(), word, place) });
  } catch (e) {
    toastError('사전에서 빼지 못함', e);
  }
}

/** 무시: not flagged again until the app closes. */
export function ignoreSpellWord(word: string) {
  const ignored = get().spellIgnored;
  if (!ignored.includes(word)) set({ spellIgnored: [...ignored, word] });
}

const suggestions = new Map<string, Promise<string[]>>();

/** Words the writer may have meant (asked once per word). */
export function spellSuggestions(word: string): Promise<string[]> {
  let found = suggestions.get(word);
  if (!found) {
    found = api.spellSuggest(word).catch(() => []);
    suggestions.set(word, found);
  }
  return found;
}

/** Checks paragraph texts for the editor with the accepted words. */
export function checkSpelling(texts: string[]) {
  return api.spellCheck(texts, spellKnown());
}
