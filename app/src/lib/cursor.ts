// Where the cursor was in each document, so it comes back there
// ("작품을 다시 열면 … 커서 위치 복원", docs/layout-data.md).

const KEY = 'wp.cursor.';
const LIMIT = 300;

type Cursors = Record<string, [number, number]>;

function read(projectId: string): Cursors {
  try {
    const raw = localStorage.getItem(KEY + projectId);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    return parsed && typeof parsed === 'object' ? (parsed as Cursors) : {};
  } catch {
    return {};
  }
}

export function loadCursor(projectId: string, docId: string): { from: number; to: number } | null {
  const c = read(projectId)[docId];
  return Array.isArray(c) && typeof c[0] === 'number' && typeof c[1] === 'number' ? { from: c[0], to: c[1] } : null;
}

export function saveCursor(projectId: string, docId: string, from: number, to: number) {
  const all = read(projectId);
  delete all[docId];
  all[docId] = [from, to];
  const ids = Object.keys(all);
  for (const id of ids.slice(0, Math.max(0, ids.length - LIMIT))) delete all[id];
  try {
    localStorage.setItem(KEY + projectId, JSON.stringify(all));
  } catch {
    // Not critical.
  }
}
