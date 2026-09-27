// Scenes of the open document, found from scene breaks (개요 탭).

import type { Node as PmNode } from '@tiptap/pm/model';

export interface Scene {
  /** From 1. */
  index: number;
  /** Document position where the scene starts. */
  pos: number;
  /** Opening words of the scene. */
  opening: string;
  chars: number;
}

function opening(text: string): string {
  const trimmed = text.trim();
  const end = trimmed.search(/[.?!…。](\s|”|’|$)/);
  const sentence = end >= 0 ? trimmed.slice(0, end + 1) : trimmed;
  const chars = [...sentence];
  return chars.length > 42 ? `${chars.slice(0, 40).join('')}…` : sentence;
}

export function scenesOf(doc: PmNode): Scene[] {
  const scenes: Scene[] = [];
  let current: Scene | null = null;
  doc.forEach((node, offset) => {
    if (node.type.name === 'sceneBreak') {
      current = null;
      return;
    }
    if (!current) {
      current = { index: scenes.length + 1, pos: offset, opening: '', chars: 0 };
      scenes.push(current);
    }
    const text = node.textContent;
    if (!current.opening && text.trim()) current.opening = opening(text);
    current.chars += [...text].length;
  });
  return scenes;
}

/** Index of the scene that contains `pos`. */
export function sceneAt(scenes: Scene[], pos: number): number {
  let found = 0;
  for (const scene of scenes) {
    if (scene.pos <= pos) found = scene.index;
  }
  return found;
}
