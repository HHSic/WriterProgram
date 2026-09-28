// Comparing two versions of a text (둘 다 보기): paragraphs are lined up, and
// inside a changed paragraph the changed letters are marked.

/** A stretch of a paragraph, changed or not. */
export interface Piece {
  text: string;
  changed: boolean;
}

export type Row =
  | { kind: 'same'; text: string }
  | { kind: 'changed'; left: Piece[]; right: Piece[] }
  /** Only on the left. */
  | { kind: 'removed'; text: string }
  /** Only on the right. */
  | { kind: 'added'; text: string };

type Op = { kind: 'same'; a: number; b: number } | { kind: 'del'; a: number } | { kind: 'ins'; b: number };

/** Longest common subsequence between `a` and `b`, as edit steps. */
function lcs<T>(a: T[], b: T[], eq: (x: T, y: T) => boolean): Op[] {
  // Common start and end first: edits are usually in one place.
  let start = 0;
  while (start < a.length && start < b.length && eq(a[start], b[start])) start++;
  let endA = a.length;
  let endB = b.length;
  while (endA > start && endB > start && eq(a[endA - 1], b[endB - 1])) {
    endA--;
    endB--;
  }
  const n = endA - start;
  const m = endB - start;
  const ops: Op[] = [];
  for (let i = 0; i < start; i++) ops.push({ kind: 'same', a: i, b: i });
  // lengths[i][j]: LCS of a[start + i..] and b[start + j..].
  const width = m + 1;
  const lengths = new Uint32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lengths[i * width + j] = eq(a[start + i], b[start + j])
        ? lengths[(i + 1) * width + j + 1] + 1
        : Math.max(lengths[(i + 1) * width + j], lengths[i * width + j + 1]);
    }
  }
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (eq(a[start + i], b[start + j])) {
      ops.push({ kind: 'same', a: start + i, b: start + j });
      i++;
      j++;
    } else if (lengths[(i + 1) * width + j] >= lengths[i * width + j + 1]) {
      ops.push({ kind: 'del', a: start + i });
      i++;
    } else {
      ops.push({ kind: 'ins', b: start + j });
      j++;
    }
  }
  for (; i < n; i++) ops.push({ kind: 'del', a: start + i });
  for (; j < m; j++) ops.push({ kind: 'ins', b: start + j });
  for (let k = 0; k < a.length - endA; k++) ops.push({ kind: 'same', a: endA + k, b: endB + k });
  return ops;
}

/** Past this many letter pairs, a changed paragraph is marked as a whole. */
const MAX_LETTER_PAIRS = 400_000;

function pieces(chars: string[], changed: boolean[]): Piece[] {
  const out: Piece[] = [];
  chars.forEach((ch, i) => {
    const last = out[out.length - 1];
    if (last && last.changed === changed[i]) last.text += ch;
    else out.push({ text: ch, changed: changed[i] });
  });
  return out;
}

/** Marks the letters that differ between two versions of a paragraph. */
export function diffLetters(left: string, right: string): { left: Piece[]; right: Piece[] } {
  const a = Array.from(left);
  const b = Array.from(right);
  if (a.length * b.length > MAX_LETTER_PAIRS) {
    return { left: [{ text: left, changed: true }], right: [{ text: right, changed: true }] };
  }
  const inA = a.map(() => true);
  const inB = b.map(() => true);
  for (const op of lcs(a, b, (x, y) => x === y)) {
    if (op.kind === 'same') {
      inA[op.a] = false;
      inB[op.b] = false;
    }
  }
  return { left: pieces(a, inA), right: pieces(b, inB) };
}

/** Lines up the paragraphs of two versions of a text. */
export function diffParagraphs(left: string[], right: string[]): Row[] {
  const rows: Row[] = [];
  let dels: string[] = [];
  let inss: string[] = [];
  const settle = () => {
    // A paragraph taken out next to one put in is the same paragraph changed.
    const pairs = Math.min(dels.length, inss.length);
    for (let k = 0; k < pairs; k++) rows.push({ kind: 'changed', ...diffLetters(dels[k], inss[k]) });
    for (const text of dels.slice(pairs)) rows.push({ kind: 'removed', text });
    for (const text of inss.slice(pairs)) rows.push({ kind: 'added', text });
    dels = [];
    inss = [];
  };
  for (const op of lcs(left, right, (x, y) => x === y)) {
    if (op.kind === 'same') {
      settle();
      rows.push({ kind: 'same', text: left[op.a] });
    } else if (op.kind === 'del') {
      dels.push(left[op.a]);
    } else {
      inss.push(right[op.b]);
    }
  }
  settle();
  return rows;
}

/** How many places differ (a run of changed paragraphs counts once). */
export function placesChanged(rows: Row[]): number {
  let places = 0;
  let inRun = false;
  for (const row of rows) {
    if (row.kind === 'same') inRun = false;
    else if (!inRun) {
      inRun = true;
      places++;
    }
  }
  return places;
}
