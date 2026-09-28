// Find in the editor: highlight matches of the current search and move to a
// match found by the Rust side (crates/core/src/search.rs).

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

export interface FindOptions {
  text: string;
  regex: boolean;
  wholeWord: boolean;
}

/** Same meaning as the Rust matcher: plain text is escaped, whole words
 * must not touch letters or digits on either side. */
export function buildRegex(q: FindOptions): RegExp | null {
  if (!q.text) return null;
  let source = q.regex ? q.text : q.text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (q.wholeWord) source = `(?<![\\p{L}\\p{N}_])(?:${source})(?![\\p{L}\\p{N}_])`;
  try {
    return new RegExp(source, 'gu');
  } catch {
    return null;
  }
}

/** A text block's text (line breaks as "\n") and the document position of
 * each UTF-16 unit. */
export function blockText(block: PmNode, blockPos: number): { text: string; positions: number[] } {
  let text = '';
  const positions: number[] = [];
  block.forEach((child, offset) => {
    const start = blockPos + 1 + offset;
    if (child.isText) {
      const t = child.text ?? '';
      for (let i = 0; i < t.length; i += 1) positions.push(start + i);
      text += t;
    } else {
      positions.push(start);
      text += '\n';
    }
  });
  return { text, positions };
}

/** Document range of UTF-16 offsets [start, end) in top-level block `block`. */
export function rangeInBlock(doc: PmNode, block: number, start: number, end: number): { from: number; to: number } | null {
  if (block < 0 || block >= doc.childCount || end <= start) return null;
  let pos = 0;
  for (let i = 0; i < block; i += 1) pos += doc.child(i).nodeSize;
  const node = doc.child(block);
  if (!node.isTextblock) return null;
  const { positions } = blockText(node, pos);
  if (end > positions.length) return null;
  return { from: positions[start], to: positions[end - 1] + 1 };
}

export const searchKey = new PluginKey<SearchState>('search');

interface SearchState {
  regex: RegExp | null;
  /** Document position of the match to show as current. */
  current: number | null;
  decorations: DecorationSet;
}

function decorate(doc: PmNode, regex: RegExp | null, current: number | null): DecorationSet {
  if (!regex) return DecorationSet.empty;
  const decorations: Decoration[] = [];
  let pos = 0;
  doc.forEach((block) => {
    if (block.isTextblock) {
      const { text, positions } = blockText(block, pos);
      for (const m of text.matchAll(regex)) {
        if (!m[0] || m[0].includes('\n') || m.index === undefined) continue;
        const from = positions[m.index];
        const to = positions[m.index + m[0].length - 1] + 1;
        decorations.push(Decoration.inline(from, to, { class: from === current ? 'search-hit current' : 'search-hit' }));
      }
    }
    pos += block.nodeSize;
  });
  return DecorationSet.create(doc, decorations);
}

/** Highlights matches of the find panel in the open document. */
export const SearchHighlight = Extension.create({
  name: 'searchHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key: searchKey,
        state: {
          init: () => ({ regex: null, current: null, decorations: DecorationSet.empty }),
          apply(tr, prev, _old, state) {
            const meta = tr.getMeta(searchKey) as Partial<SearchState> | undefined;
            if (!meta && !tr.docChanged) return prev;
            const regex = meta && 'regex' in meta ? (meta.regex ?? null) : prev.regex;
            // Changing to another search drops the marked match unless one is given.
            const current =
              meta && 'current' in meta
                ? (meta.current ?? null)
                : meta && prev.regex && meta.regex?.source !== prev.regex.source
                  ? null
                  : prev.current;
            return { regex, current, decorations: decorate(state.doc, regex, current) };
          },
        },
        props: {
          decorations: (state) => searchKey.getState(state)?.decorations,
        },
      }),
    ];
  },
});

/** Sets the search to highlight. `current` marks one match; leave it out to keep the one marked. */
export function setHighlight(editor: Editor, regex: RegExp | null, current?: number | null) {
  if (editor.isDestroyed) return;
  const meta: Partial<SearchState> = current === undefined ? { regex } : { regex, current };
  editor.view.dispatch(editor.state.tr.setMeta(searchKey, meta).setMeta('addToHistory', false));
}

/** Selects a match and scrolls it into the middle of the view. */
export function showMatch(editor: Editor, block: number, start: number, end: number): boolean {
  const range = rangeInBlock(editor.state.doc, block, start, end);
  if (!range) return false;
  editor.chain().setTextSelection(range).run();
  const state = searchKey.getState(editor.state);
  setHighlight(editor, state?.regex ?? null, range.from);
  const dom = editor.view.domAtPos(range.from).node;
  const el = dom instanceof HTMLElement ? dom : dom.parentElement;
  el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  return true;
}

/** Replaces one match in the editor. The text must still be what was found. */
export function replaceMatch(
  editor: Editor,
  match: { block: number; start: number; end: number; text: string },
  replacement: string,
): boolean {
  const range = rangeInBlock(editor.state.doc, match.block, match.start, match.end);
  if (!range) return false;
  if (editor.state.doc.textBetween(range.from, range.to, '\n') !== match.text) return false;
  return editor
    .chain()
    .focus()
    .command(({ tr }) => {
      tr.insertText(replacement, range.from, range.to);
      return true;
    })
    .run();
}
