import { num } from '../lib/format';
import { writtenToday } from '../lib/today';
import { findDoc, useApp } from '../store';

export function StatusBar() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const live = useApp((s) => s.liveCounts);
  const selection = useApp((s) => s.selection);
  const place = activeDocId ? findDoc(ov, activeDocId) : null;
  const counts = live ?? place?.doc.counts ?? null;

  const lengths: Record<string, number> = {};
  for (const part of ov.parts) for (const d of part.docs) lengths[d.id] = d.counts.withSpaces;
  if (activeDocId && live && place?.section === 'manuscript') lengths[activeDocId] = live.withSpaces;
  const today = writtenToday(ov.project.id, lengths);

  const goal = place?.section === 'manuscript' ? (place.doc.target ?? ov.project.goal.perDoc) : null;
  const goalChars = counts ? (ov.project.goal.countSpaces ? counts.withSpaces : counts.withoutSpaces) : 0;

  return (
    <footer className="status-bar" aria-label="분량">
      {counts && (
        <>
          {selection ? (
            <span className="status-item strong">
              선택 {num(selection.withSpaces)}자 · 공백 제외 {num(selection.withoutSpaces)}자
            </span>
          ) : (
            <>
              <span className="status-item">공백 포함 {num(counts.withSpaces)}자</span>
              <span className="status-item">공백 제외 {num(counts.withoutSpaces)}자</span>
            </>
          )}
          <span className="status-item">원고지 {num(counts.manuscriptPages)}매</span>
          {goal ? (
            <span className="status-item">
              목표 {num(goal)}자의 {Math.floor((goalChars / goal) * 100)}%
            </span>
          ) : null}
        </>
      )}
      <span className="grow" />
      <span className="status-item">
        오늘 {today >= 0 ? '+' : '−'}
        {num(Math.abs(today))}자
      </span>
    </footer>
  );
}
