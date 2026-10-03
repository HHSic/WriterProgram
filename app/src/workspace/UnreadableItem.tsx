// A row in the left column for a chapter or planning document whose file is
// there but cannot be read: dimmed at its place, "열 수 없음"; pressing it
// opens 고쳐 열기 (MendDialog.tsx).

import type { UnreadableDoc } from '../api/types';
import { Icon } from '../components/Icon';
import { openDialog } from '../store';

export const UNREADABLE_TITLE = '이름을 알 수 없는 문서';

export function UnreadableItem({ item, planning = false }: { item: UnreadableDoc; planning?: boolean }) {
  const title = item.titleGuess.trim() || UNREADABLE_TITLE;
  const open = () => openDialog({ kind: 'mend', docId: item.id });
  if (planning) {
    return (
      <div className="plan-row unreadable" data-doc={item.id}>
        <button type="button" className="plan-item" title={item.reason} aria-label={`${title}, 열 수 없음`} onClick={open}>
          <Icon name="pencil" size={14} />
          <span className="grow ellipsis">{title}</span>
          <span className="chip unreadable-chip">열 수 없음</span>
        </button>
      </div>
    );
  }
  return (
    <div className="doc-row unreadable" data-doc={item.id}>
      <button type="button" className="doc-item" title={item.reason} aria-label={`${title}, 열 수 없음`} onClick={open}>
        <span className="doc-line">
          <span className="doc-title">{title}</span>
          <span className="chip unreadable-chip">열 수 없음</span>
        </span>
      </button>
    </div>
  );
}
