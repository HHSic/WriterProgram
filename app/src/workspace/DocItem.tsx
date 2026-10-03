// A chapter's row in the left column: number, title, notes, copies, lock, status and progress to its goal.

import type { DragEvent } from 'react';
import type { DocSummary, ProjectKind } from '../api/types';
import { Icon } from '../components/Icon';
import type { MenuItem, MenuOptions } from '../components/Menu';
import { MoreButton } from '../components/MoreButton';
import { num } from '../lib/format';
import { STATUS_LABEL, UNTITLED, docNoun } from '../lib/labels';
import { pressMenu, type TouchDrag } from '../lib/press';
import { openDocInNewTab, selectDoc } from '../store';
import { CopyBadge } from './Copies';

export function DocItem({
  doc,
  label,
  kind,
  goal,
  countSpaces,
  active,
  memos,
  copies,
  drop,
  menu,
  touchDrag,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
}: {
  doc: DocSummary;
  label: string;
  kind: ProjectKind;
  goal: number | null;
  countSpaces: boolean;
  active: boolean;
  memos: number;
  /** Copies other devices left of it (다른 기기 사본). */
  copies: number;
  drop: 'before' | 'after' | null;
  menu: () => MenuItem[];
  touchDrag: TouchDrag;
  onDragStart: (e: DragEvent) => void;
  onDragOver: (e: DragEvent) => void;
  onDrop: (e: DragEvent) => void;
  onDragEnd: () => void;
}) {
  const chars = countSpaces ? doc.counts.withSpaces : doc.counts.withoutSpaces;
  const pct = goal ? Math.min(100, Math.round((chars / goal) * 100)) : null;
  const title = `${label} ${doc.title || UNTITLED}`;
  // The synopsis a mouse sees on hover shows at the top of the menu.
  const opts = (): MenuOptions => ({ title, subtitle: doc.synopsis || undefined });
  return (
    <div className="doc-row" data-doc={doc.id}>
      <button
        type="button"
        draggable
        className={`doc-item${active ? ' active' : ''}${drop ? ` drop-${drop}` : ''}`}
        title={doc.synopsis || undefined}
        aria-label={`${title}, ${STATUS_LABEL[doc.status]}${doc.locked ? ', 잠금' : ''}, ${num(chars)}자${memos ? `, 열린 메모 ${memos}개` : ''}`}
        aria-current={active ? 'true' : undefined}
        onClick={(e) => void selectDoc(doc.id, e.ctrlKey || e.metaKey)}
        onDoubleClick={() => void openDocInNewTab(doc.id)}
        {...pressMenu(menu, opts, touchDrag)}
        onDragStart={onDragStart}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onDragEnd={onDragEnd}
      >
        <span className="doc-line">
          <span className="doc-no">{label}</span>
          <span className="doc-title">{doc.title || UNTITLED}</span>
          {memos > 0 && (
            <span className="memo-count" title={`열린 메모 ${memos}개`}>
              <Icon name="note" size={11} />
              {memos}
            </span>
          )}
          <CopyBadge count={copies} />
          {doc.locked && (
            <span className="doc-lock" title={`잠근 ${docNoun(kind)}: 고치지 않게 잠가 둠`} aria-hidden="true">
              <Icon name="lock" size={11} />
            </span>
          )}
          <span className={`chip status-${doc.status}`} data-kind={kind}>
            {STATUS_LABEL[doc.status]}
          </span>
        </span>
        <span className="doc-line">
          {pct !== null ? (
            <span className="bar" aria-hidden="true">
              <span className={`bar-fill${pct >= 100 ? ' full' : ''}`} style={{ width: `${pct}%` }} />
            </span>
          ) : (
            <span className="grow" />
          )}
          <span className="doc-count">{num(chars)}</span>
        </span>
      </button>
      <MoreButton items={menu} opts={opts} label={`${title} 메뉴`} />
    </div>
  );
}
