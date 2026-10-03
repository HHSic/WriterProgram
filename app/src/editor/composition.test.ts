import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EditorState, Transaction } from '@tiptap/pm/state';
import { DecorationSet, type EditorView } from '@tiptap/pm/view';
import type { CardSummary, IndentRules } from '../api/types';
import { cardPlugin, nameIndex, setCardNames } from './cards';
import { COMPOSITION_END, afterComposition, onCompositionEnd, type ComposingView } from './composition';
import { indentPlugin, setIndentRules } from './indent';
import { buildRegex, searchPlugin, setHighlight } from './search';
import { chapter, composing, fakeEditor } from './testing';

const RULES: IndentRules = { chapterFirst: false, afterScene: false, margined: false, dialogue: true, dialogueHang: false };
const SEOHA: CardSummary = { id: 'a', cardType: 'person', name: '서하', aliases: [], highlight: true, summary: '' };

/** Each plugin's decorations as text: "class@text". */
function shown(state: EditorState): string[] {
  const out: string[] = [];
  for (const plugin of state.plugins) {
    const set = plugin.props.decorations?.call(plugin, state);
    if (!(set instanceof DecorationSet)) continue;
    for (const d of set.find()) {
      const attrs = (d as unknown as { type: { attrs: { class: string } } }).type.attrs;
      const text = state.doc.textBetween(d.from, d.to, '/');
      out.push(`${attrs.class}@${text}`);
    }
  }
  return out.sort();
}

function editor(...blocks: (string | null)[]) {
  const e = fakeEditor(chapter(...blocks), [cardPlugin(() => {}), indentPlugin(), searchPlugin()]);
  setCardNames(e, nameIndex([SEOHA]));
  setIndentRules(e, RULES);
  setHighlight(e, buildRegex({ text: '서하', regex: false, wholeWord: false }));
  return e;
}

/** Types `steps` as an input method would: each replaces the syllable being composed. */
function compose(e: ReturnType<typeof editor>, at: number, steps: string[]) {
  let len = 0;
  for (const s of steps) {
    e.view.dispatch(composing(e.state.tr.insertText(s, at, at + len)));
    len = s.length;
  }
}

function end(e: ReturnType<typeof editor>) {
  e.view.dispatch(e.state.tr.setMeta(COMPOSITION_END, true));
}

describe('decorations while composing', () => {
  it('are not worked out again until the composition ends', () => {
    const e = editor('어제 ', '본문');
    const before = shown(e.state);
    // 어제 서하: ㅅ → 서, then ㅎ → 하 in a second composition.
    compose(e, 4, ['ㅅ', '서']);
    compose(e, 5, ['ㅎ', '하']);
    expect(e.state.doc.textContent).toBe('어제 서하본문');
    expect(shown(e.state)).toEqual(before);
    end(e);
    expect(shown(e.state)).toEqual(['card-name@서하', 'search-hit@서하']);
  });

  it('indent follows a quote typed at a paragraph start only after the end', () => {
    const e = editor('첫 문단', '말했다');
    // Position 7 is the start of the second paragraph's text.
    compose(e, 7, ['“']);
    expect(shown(e.state)).toEqual([]);
    end(e);
    expect(shown(e.state)).toEqual(['indent-flush@“말했다']);
  });

  it('a plain edit after composing also catches up', () => {
    const e = editor('어제 ');
    compose(e, 4, ['서']);
    compose(e, 5, ['하']);
    e.view.dispatch(e.state.tr.insertText(' ', 6));
    expect(shown(e.state)).toEqual(['card-name@서하', 'search-hit@서하']);
  });

  it('decorations move along with the text typed before them', () => {
    const e = editor('서하가 왔다', '“서하야.”');
    const before = shown(e.state);
    expect(before).toEqual(['card-name@서하', 'card-name@서하', 'indent-flush@“서하야.”', 'search-hit@서하', 'search-hit@서하']);
    compose(e, 1, ['ㄱ', '그', '그ㄴ', '그녀']);
    expect(e.state.doc.child(0).textContent).toBe('그녀서하가 왔다');
    expect(shown(e.state)).toEqual(before);
    end(e);
    // 그녀서하가: 서하 is still a word before 가.
    expect(shown(e.state)).toEqual(before);
  });

  it('an end with nothing waiting changes nothing', () => {
    const e = editor('서하');
    const state = e.state;
    end(e);
    for (const p of state.plugins) expect(p.getState(e.state)).toBe(p.getState(state));
  });
});

describe('the end of a composition', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function view(composingNow: boolean) {
    const sent: Transaction[] = [];
    const e = editor('');
    const v = {
      composing: composingNow,
      isDestroyed: false,
      get state() {
        return e.state;
      },
      dispatch: (tr: Transaction) => sent.push(tr),
    };
    return { v: v as unknown as ComposingView & { composing: boolean }, sent };
  }

  it('is sent a task after compositionend, once the composed text is read', () => {
    const { v, sent } = view(false);
    onCompositionEnd(v);
    expect(sent).toHaveLength(0);
    vi.runAllTimers();
    expect(sent).toHaveLength(1);
    expect(sent[0].getMeta(COMPOSITION_END)).toBe(true);
    expect(sent[0].docChanged).toBe(false);
  });

  it('is left to the next syllable when one has started', () => {
    const { v, sent } = view(false);
    onCompositionEnd(v);
    v.composing = true;
    vi.runAllTimers();
    expect(sent).toHaveLength(0);
  });

  it('lets waiting work run after the composition, not during the next syllable', () => {
    const dom = new EventTarget();
    const v = { composing: true, isDestroyed: false, dom } as unknown as Pick<EditorView, 'composing' | 'isDestroyed' | 'dom'> & {
      composing: boolean;
    };
    const run = vi.fn();
    afterComposition(v, run);
    expect(run).not.toHaveBeenCalled();
    // One syllable ends and the next starts at once.
    dom.dispatchEvent(new Event('compositionend'));
    vi.runAllTimers();
    expect(run).not.toHaveBeenCalled();
    v.composing = false;
    dom.dispatchEvent(new Event('compositionend'));
    vi.runAllTimers();
    expect(run).toHaveBeenCalledTimes(1);
    // Not composing: runs at once.
    afterComposition(v, run);
    expect(run).toHaveBeenCalledTimes(2);
  });
});
