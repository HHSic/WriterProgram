// First-line indent rules on screen (crates/core/src/indent.rs): paragraphs
// the manuscript format leaves flush (a chapter's first paragraph, the one
// after a scene break, paragraphs set in with margins, dialogue) or sets in
// as on 원고지 (dialogue with every line one cell in). A paragraph's own first
// line (data-indent) is drawn by CSS and always wins.

import { Extension, type Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import type { IndentRules } from '../api/types';

/** Marks that open a line of dialogue (same as indent.rs). */
const QUOTES = ['“', '"', '「', '『', '‘', "'", '《', '«'];

export function isDialogue(paragraph: PmNode): boolean {
  const text = paragraph.textContent.trimStart();
  return text.length > 0 && QUOTES.includes(text[0]);
}

export type IndentKind = 'flush' | 'hang' | null;

/** What the rules make of each top-level paragraph, by position. */
export function indentKinds(doc: PmNode, rules: IndentRules | null): Map<number, IndentKind> {
  const out = new Map<number, IndentKind>();
  if (!rules) return out;
  let first = true;
  let afterScene = false;
  doc.forEach((node, pos) => {
    if (node.type.name !== 'paragraph') {
      if (node.type.name === 'sceneBreak') afterScene = true;
      return;
    }
    const own = node.attrs.indent !== null && node.attrs.indent !== undefined;
    if (!own) {
      const margined = Number(node.attrs.left) > 0 || Number(node.attrs.right) > 0;
      const dialogue = isDialogue(node);
      const flush =
        (rules.chapterFirst && first) || (rules.afterScene && afterScene) || (rules.margined && margined) || (rules.dialogue && dialogue);
      if (flush) out.set(pos, 'flush');
      else if (rules.dialogueHang && dialogue) out.set(pos, 'hang');
    }
    if (node.content.size > 0) {
      first = false;
      afterScene = false;
    }
  });
  return out;
}

interface IndentState {
  rules: IndentRules | null;
  decorations: DecorationSet;
}

const indentKey = new PluginKey<IndentState>('indentRules');

function decorate(doc: PmNode, rules: IndentRules | null): DecorationSet {
  const decorations: Decoration[] = [];
  for (const [pos, kind] of indentKinds(doc, rules)) {
    const node = doc.nodeAt(pos);
    if (!node || !kind) continue;
    decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: kind === 'flush' ? 'indent-flush' : 'indent-hang' }));
  }
  return DecorationSet.create(doc, decorations);
}

export const IndentRulesExtension = Extension.create({
  name: 'indentRules',

  addProseMirrorPlugins() {
    return [
      new Plugin<IndentState>({
        key: indentKey,
        state: {
          init: () => ({ rules: null, decorations: DecorationSet.empty }),
          apply(tr, prev, _old, state) {
            const meta = tr.getMeta(indentKey) as { rules: IndentRules | null } | undefined;
            if (meta) return { rules: meta.rules, decorations: decorate(state.doc, meta.rules) };
            // Rules look at neighbours, so the whole chapter is looked at again;
            // a chapter is a few hundred paragraphs at most.
            if (!prev.rules || !tr.docChanged) return prev;
            return { rules: prev.rules, decorations: decorate(state.doc, prev.rules) };
          },
        },
        props: {
          decorations: (state) => indentKey.getState(state)?.decorations,
        },
      }),
    ];
  },
});

/** Follows the manuscript format's rules (none: every paragraph gets the indent). */
export function setIndentRules(editor: Editor, rules: IndentRules | null) {
  if (editor.isDestroyed) return;
  const now = indentKey.getState(editor.state)?.rules ?? null;
  if (JSON.stringify(now) === JSON.stringify(rules)) return;
  editor.view.dispatch(editor.state.tr.setMeta(indentKey, { rules }).setMeta('addToHistory', false));
}
