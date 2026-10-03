// Counts for the status bar. Same rules as crates/core/src/count.rs; both are
// checked against crates/core/tests/fixtures/counts.json and, for counting the
// way a serial platform does, platform-counts.json (docs/platforms.md).

import type { JSONContent } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';
import type { CountRule, Counts } from '../api/types';

/** A body reduced to what counting needs: scene breaks and lines of text. */
export type PlainBlock = { scene: true } | { scene: false; lines: string[] };

function pushText(lines: string[], text: string) {
  text.split('\n').forEach((part, i) => {
    if (i > 0) lines.push('');
    lines[lines.length - 1] += part;
  });
}

export function blocksFromJSON(doc: JSONContent): PlainBlock[] {
  return (doc.content ?? []).map((node): PlainBlock => {
    if (node.type === 'sceneBreak') return { scene: true };
    const lines = [''];
    for (const inline of node.content ?? []) {
      if (inline.type === 'hardBreak') lines.push('');
      else if (inline.type === 'text') pushText(lines, inline.text ?? '');
    }
    return { scene: false, lines };
  });
}

export function blocksFromNode(doc: PmNode): PlainBlock[] {
  const out: PlainBlock[] = [];
  doc.forEach((node) => {
    if (node.type.name === 'sceneBreak') {
      out.push({ scene: true });
      return;
    }
    const lines = [''];
    node.forEach((child) => {
      if (child.type.name === 'hardBreak') lines.push('');
      else if (child.isText) pushText(lines, child.text ?? '');
    });
    out.push({ scene: false, lines });
  });
  return out;
}

const SPACES = new Set([
  '\t', '\n', '\u000b', '\u000c', '\r', ' ', '\u0085', ' ', ' ',
  ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
  ' ', ' ', ' ', ' ', '　', '﻿',
]);

export function isSpace(c: string): boolean {
  return SPACES.has(c);
}

/** Half cells a character takes on 원고지: digits and lowercase Latin share a cell. */
function halfCells(c: string): number {
  const code = c.codePointAt(0) ?? 0;
  return (code >= 0x30 && code <= 0x39) || (code >= 0x61 && code <= 0x7a) ? 1 : 2;
}

/** Punctuation that hangs in the margin instead of starting a line. */
const HANGS = new Set(['.', ',', '!', '?', ':', ';', ')', ']', '}', '”', '’', '」', '』', '》', '〉']);
const LINE_HALF_CELLS = 40;

function paperLines(text: string, indent: boolean): number {
  let lines = 1;
  let col = indent ? 2 : 0;
  let prev: string | null = null;
  for (const c of text) {
    const space = isSpace(c);
    if (space && (prev === '.' || prev === ',')) {
      prev = c;
      continue;
    }
    const width = space ? 2 : halfCells(c);
    if (col + width > LINE_HALF_CELLS) {
      if (HANGS.has(c)) {
        prev = c;
        continue;
      }
      lines += 1;
      col = 0;
      if (space) {
        prev = c;
        continue;
      }
    }
    col += width;
    prev = c;
  }
  return lines;
}

export const ZERO_COUNTS: Counts = {
  withSpaces: 0,
  withoutSpaces: 0,
  manuscriptLines: 0,
  manuscriptPages: 0,
  plainMarks: 0,
  wide: 0,
  htmlExtra: 0,
};

/** Straight marks 노벨피아 leaves out of its count. */
const PLAIN_MARKS = new Set(['.', ',', '!', '?', "'", '"']);
const HTML_EXTRA: Record<string, number> = { '<': 3, '>': 3, '&': 4 };

export function countBlocks(blocks: PlainBlock[]): Counts {
  let withSpaces = 0;
  let withoutSpaces = 0;
  let plainMarks = 0;
  let wide = 0;
  let htmlExtra = 0;
  let lines = 0;
  for (const block of blocks) {
    if (block.scene) {
      lines += 1;
      continue;
    }
    block.lines.forEach((line, i) => {
      for (const c of line) {
        withSpaces += 1;
        if (!isSpace(c)) withoutSpaces += 1;
        if (PLAIN_MARKS.has(c)) plainMarks += 1;
        if ((c.codePointAt(0) ?? 0) > 0xffff) wide += 1;
        htmlExtra += HTML_EXTRA[c] ?? 0;
      }
      lines += paperLines(line, i === 0);
    });
  }
  if (withSpaces === 0) return { ...ZERO_COUNTS };
  return {
    withSpaces,
    withoutSpaces,
    manuscriptLines: lines,
    manuscriptPages: Math.ceil(lines / 10),
    plainMarks,
    wide,
    htmlExtra,
  };
}

/** Characters as a platform with `rule` counts them. Line breaks never count. */
export function countByRule(c: Counts, rule: CountRule): number {
  let n = rule.spaces ? c.withSpaces : c.withoutSpaces;
  if (rule.skipMarks) n = Math.max(0, n - c.plainMarks);
  if (rule.wideTwice) n += c.wide;
  if (rule.htmlEscapes) n += c.htmlExtra;
  return n;
}

/** Characters in a piece of plain text, for a selection. Line breaks do not count. */
export function countChars(text: string): { withSpaces: number; withoutSpaces: number } {
  let withSpaces = 0;
  let withoutSpaces = 0;
  for (const c of text) {
    if (c === '\n') continue;
    withSpaces += 1;
    if (!isSpace(c)) withoutSpaces += 1;
  }
  return { withSpaces, withoutSpaces };
}
