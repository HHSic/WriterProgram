// 소리 내어 읽기 (read aloud, for revising): the device's own voices
// (speechSynthesis; on Windows the installed 한국어 voice) read the chapter a
// sentence at a time, lighting up the sentence being read. Nothing leaves the
// device and no AI is involved.

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { blockText } from './search';
import { splitSentences } from './sentences';

/** One thing to read: a sentence, or a scene break read as a short pause
 * (`text` null). `from`/`to` are document positions. */
export interface ReadStep {
  from: number;
  to: number;
  text: string | null;
}

const SAYABLE = /[\p{L}\p{N}]/u;

/**
 * What to read from `from` to `to`. With no selection (`from === to`) it is
 * the rest of the chapter from the sentence the cursor is in; with one, just
 * the selected text. Scene breaks become pauses; bits with nothing to say
 * ("……", a lone quote) are left out.
 */
export function readingPlan(doc: PmNode, from: number, to: number): ReadStep[] {
  const fromCursor = from === to;
  const steps: ReadStep[] = [];
  doc.descendants((node, pos) => {
    const end = pos + node.nodeSize;
    if (end <= from || (!fromCursor && pos >= to)) return false;
    if (node.type.name === 'sceneBreak') {
      if (pos >= from && (fromCursor || end <= to)) steps.push({ from: pos, to: end, text: null });
      return false;
    }
    if (!node.isTextblock) return true;
    const { text, positions } = blockText(node, pos);
    for (const s of splitSentences(text)) {
      let a = s.start;
      let b = s.end;
      if (fromCursor) {
        // Sentences that end before the cursor are behind it.
        if (positions[b - 1] + 1 <= from) continue;
      } else {
        while (a < b && positions[a] < from) a += 1;
        while (b > a && positions[b - 1] + 1 > to) b -= 1;
        while (a < b && /\s/.test(text[a])) a += 1;
        while (b > a && /\s/.test(text[b - 1])) b -= 1;
        if (a >= b) continue;
      }
      const said = text.slice(a, b);
      if (!SAYABLE.test(said)) continue;
      steps.push({ from: positions[a], to: positions[b - 1] + 1, text: said.replace(/\n/g, ' ') });
    }
    return false;
  });
  // A pause first, last or twice in a row says nothing.
  const kept: ReadStep[] = [];
  for (const s of steps) {
    if (s.text === null && (kept.length === 0 || kept[kept.length - 1].text === null)) continue;
    kept.push(s);
  }
  while (kept.length && kept[kept.length - 1].text === null) kept.pop();
  return kept;
}

// ---------------------------------------------------------------------------
// Lighting up the sentence being read

const readingKey = new PluginKey<DecorationSet>('readAloud');

/** Marks the sentence (or scene break) being read. */
export const ReadAloudHighlight = Extension.create({
  name: 'readAloudHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: readingKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, prev) {
            const meta = tr.getMeta(readingKey) as ReadStep | null | undefined;
            if (meta === undefined) return prev.map(tr.mapping, tr.doc);
            if (!meta) return DecorationSet.empty;
            const deco =
              meta.text === null
                ? Decoration.node(meta.from, meta.to, { class: 'reading-now' })
                : Decoration.inline(meta.from, meta.to, { class: 'reading-now' });
            return DecorationSet.create(tr.doc, [deco]);
          },
        },
        props: {
          decorations: (state) => readingKey.getState(state),
        },
      }),
    ];
  },
});

function mark(editor: Editor, step: ReadStep | null) {
  if (editor.isDestroyed) return;
  editor.view.dispatch(editor.state.tr.setMeta(readingKey, step).setMeta('addToHistory', false));
}

/** Scrolls so the step is in view, a third of the way down, if it is not already. */
function reveal(editor: Editor, pos: number) {
  if (editor.isDestroyed) return;
  const view = editor.view;
  let rect: { top: number; bottom: number };
  try {
    rect = view.coordsAtPos(pos);
  } catch {
    return;
  }
  const scroller = view.dom.closest('.doc-scroll');
  const box = scroller ? scroller.getBoundingClientRect() : { top: 0, bottom: window.innerHeight, height: window.innerHeight };
  const room = 48;
  if (rect.top >= box.top + room && rect.bottom <= box.bottom - room) return;
  const by = rect.top - (box.top + box.height / 3);
  if (scroller) scroller.scrollBy({ top: by, behavior: 'smooth' });
  else window.scrollBy({ top: by, behavior: 'smooth' });
}

// ---------------------------------------------------------------------------
// Voices

export function speechAvailable(): boolean {
  return typeof window !== 'undefined' && 'speechSynthesis' in window && typeof SpeechSynthesisUtterance !== 'undefined';
}

type VoiceLike = Pick<SpeechSynthesisVoice, 'lang' | 'voiceURI' | 'localService'>;

/** The Korean voices, the ones on the device first. */
export function koreanVoices<V extends VoiceLike>(all: readonly V[]): V[] {
  return all
    .filter((v) => v.lang.toLowerCase().startsWith('ko'))
    .sort((a, b) => Number(b.localService) - Number(a.localService));
}

/** The chosen Korean voice, or the first one when it is gone or none was chosen. */
export function pickVoice<V extends VoiceLike>(all: readonly V[], voiceURI: string): V | null {
  const ko = koreanVoices(all);
  return ko.find((v) => v.voiceURI === voiceURI) ?? ko[0] ?? null;
}

/** The installed voices. The list can arrive a little after the page loads. */
export function loadVoices(): Promise<SpeechSynthesisVoice[]> {
  if (!speechAvailable()) return Promise.resolve([]);
  const synth = window.speechSynthesis;
  const now = synth.getVoices();
  if (now.length) return Promise.resolve(now);
  return new Promise((resolve) => {
    const done = () => {
      clearTimeout(timer);
      synth.removeEventListener('voiceschanged', done);
      resolve(synth.getVoices());
    };
    const timer = setTimeout(done, 1500);
    synth.addEventListener('voiceschanged', done);
  });
}

/** Says one line (보기 설정's 들어 보기). */
export function sayOnce(text: string, voice: SpeechSynthesisVoice, rate: number) {
  const synth = window.speechSynthesis;
  synth.cancel();
  const u = new SpeechSynthesisUtterance(text);
  u.voice = voice;
  u.lang = voice.lang;
  u.rate = rate;
  synth.speak(u);
}

// ---------------------------------------------------------------------------
// Reading

export type ReaderState = 'reading' | 'paused' | 'done';

export interface ReaderHooks {
  voice: SpeechSynthesisVoice;
  /** Speed, read again before each sentence so a change applies from the next one. */
  rate: () => number;
  onState: (state: ReaderState) => void;
  /** The voice failed (not the reader being stopped). */
  onError: (reason: string) => void;
}

/** A scene break is read as this long a pause, at normal speed. */
const SCENE_PAUSE_MS = 700;

/**
 * Reads `steps` aloud in `editor`, one utterance per sentence. Pausing cancels
 * the voice and resuming says the sentence again from its start: the browser's
 * own pause is unreliable (some voices never come back from it).
 */
export class Reader {
  private index = 0;
  /** Bumped whenever the voice is cut off, so its late events are ignored. */
  private turn = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** Kept so the browser does not drop the utterance (and its end event) early. */
  private utterance: SpeechSynthesisUtterance | null = null;
  private state: ReaderState = 'reading';

  constructor(
    readonly editor: Editor,
    private readonly steps: ReadStep[],
    private readonly hooks: ReaderHooks,
  ) {}

  start() {
    this.editor.on('update', this.onEdit);
    this.editor.on('destroy', this.onGone);
    this.say();
  }

  pause() {
    if (this.state !== 'reading') return;
    this.silence();
    this.state = 'paused';
    this.hooks.onState('paused');
  }

  resume() {
    if (this.state !== 'paused') return;
    this.state = 'reading';
    this.hooks.onState('reading');
    this.say();
  }

  /** Stops for good. With `placeCursor` the cursor goes to the start of the
   * sentence being read, ready to fix it. */
  stop(placeCursor = false) {
    if (this.state === 'done') return;
    this.silence();
    this.state = 'done';
    this.editor.off('update', this.onEdit);
    this.editor.off('destroy', this.onGone);
    if (!this.editor.isDestroyed) {
      mark(this.editor, null);
      const at = this.steps.slice(0, this.index + 1).reverse().find((s) => s.text !== null);
      if (placeCursor && at) this.editor.chain().focus().setTextSelection(at.from).run();
    }
    this.hooks.onState('done');
  }

  private onEdit = () => this.stop();
  private onGone = () => this.stop();

  private silence() {
    this.turn += 1;
    clearTimeout(this.timer);
    if (this.utterance) this.utterance.onend = this.utterance.onerror = null;
    this.utterance = null;
    window.speechSynthesis.cancel();
  }

  private say() {
    const step = this.steps[this.index];
    if (!step) {
      this.stop();
      return;
    }
    const turn = this.turn;
    mark(this.editor, step);
    reveal(this.editor, step.from);
    const rate = this.hooks.rate();
    if (step.text === null) {
      this.timer = setTimeout(() => turn === this.turn && this.next(), SCENE_PAUSE_MS / rate);
      return;
    }
    const u = new SpeechSynthesisUtterance(step.text);
    u.voice = this.hooks.voice;
    u.lang = this.hooks.voice.lang;
    u.rate = rate;
    u.onend = () => {
      if (turn === this.turn) this.next();
    };
    u.onerror = (e) => {
      if (turn !== this.turn || e.error === 'interrupted' || e.error === 'canceled') return;
      this.stop();
      this.hooks.onError(e.error);
    };
    this.utterance = u;
    window.speechSynthesis.speak(u);
  }

  private next() {
    this.index += 1;
    this.say();
  }
}
