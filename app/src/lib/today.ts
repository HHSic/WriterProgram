// "오늘 쓴 양": characters added today across the project. Each document's
// length is remembered the first time it is seen on a given day.

import { localDate } from './format';

interface Stored {
  date: string;
  base: Record<string, number>;
}

function key(projectId: string) {
  return `wp.today.${projectId}`;
}

function read(projectId: string): Stored {
  const today = localDate();
  try {
    const raw = localStorage.getItem(key(projectId));
    if (raw) {
      const stored = JSON.parse(raw) as Stored;
      if (stored.date === today) return stored;
    }
  } catch {
    // Start fresh below.
  }
  return { date: today, base: {} };
}

function write(projectId: string, stored: Stored) {
  try {
    localStorage.setItem(key(projectId), JSON.stringify(stored));
  } catch {
    // Not critical.
  }
}

/**
 * Remembers today's starting length for documents not seen yet today and
 * returns the characters written today (can be negative after cutting).
 */
export function writtenToday(projectId: string, lengths: Record<string, number>): number {
  const stored = read(projectId);
  let changed = false;
  let total = 0;
  for (const [id, length] of Object.entries(lengths)) {
    if (!(id in stored.base)) {
      stored.base[id] = length;
      changed = true;
    }
    total += length - stored.base[id];
  }
  if (changed) write(projectId, stored);
  return total;
}
