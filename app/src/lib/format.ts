export function num(n: number): string {
  return n.toLocaleString('ko-KR');
}

function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function hhmm(d: Date): string {
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** "오늘 14:20", "어제 18:02", "9월 26일 18:02", "2025년 9월 26일". */
export function timeLabel(iso: string, now = new Date()): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  if (sameDay(d, now)) return `오늘 ${hhmm(d)}`;
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (sameDay(d, yesterday)) return `어제 ${hhmm(d)}`;
  if (d.getFullYear() === now.getFullYear()) return `${d.getMonth() + 1}월 ${d.getDate()}일 ${hhmm(d)}`;
  return `${d.getFullYear()}년 ${d.getMonth() + 1}월 ${d.getDate()}일`;
}

/** Keeps the last parts of a long path: "…\문서\WriterProgram\달빛 서점". */
export function shortPath(path: string, keep = 3): string {
  const sep = path.includes('\\') ? '\\' : '/';
  const parts = path.split(sep).filter(Boolean);
  return parts.length <= keep ? path : `…${sep}${parts.slice(-keep).join(sep)}`;
}

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** Today as YYYY-MM-DD in local time. */
export function localDate(d = new Date()): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** File-name friendly text: drops characters Windows does not allow. */
export function fileSafe(s: string): string {
  return s.replace(/[\\/:*?"<>|]/g, '_').trim();
}
