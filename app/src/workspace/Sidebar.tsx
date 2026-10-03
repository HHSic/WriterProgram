// Left column (탐색): the manuscript tree, planning documents and trash.
// Always the same whatever is open in the middle (docs/layout-data.md).
// Every row's menu opens by right click, by a long press, or by its ⋯ button;
// rows are reordered by mouse drag, by a long press and drag, or from the menu.

import { useState } from 'react';
import { Icon } from '../components/Icon';
import { type MenuOptions } from '../components/Menu';
import { MoreButton } from '../components/MoreButton';
import { num } from '../lib/format';
import { KIND_LABEL, UNTITLED, docNoun, docNumber, stockCount, withSubject } from '../lib/labels';
import { pressMenu } from '../lib/press';
import { unreadableIn, withUnreadable } from '../lib/unreadable';
import {
  addDoc,
  addPart,
  leaveProject,
  newDocPartId,
  openDialog,
  openDocInNewTab,
  openFind,
  openNoteCounts,
  openNotesBoard,
  openTable,
  selectDoc,
  selectPart,
  useApp,
} from '../store';
import { CardsSection } from './CardsSection';
import { CopyBadge, countCopies } from './Copies';
import { DocItem } from './DocItem';
import { UnreadableItem } from './UnreadableItem';
import { docMenu, partMenu, type SidebarMenuContext } from './sidebarMenus';
import { useDocDrag } from './useDocDrag';

export function Sidebar() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const shownDocId = activeDocId;
  const notes = useApp((s) => s.notes);
  const notesOpen = useApp((s) => s.activeTarget?.kind === 'notes');
  const openNotes = notes.filter((n) => !n.done).length;
  const memoCounts = openNoteCounts(notes);
  const copyCounts = countCopies(ov.copies);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const kind = ov.project.kind;
  const noun = docNoun(kind);

  const selectedPartId = useApp((s) => s.selectedPartId);
  // 새 회차 goes to the part picked in the tree, or else the open chapter's part.
  const newPart =
    ov.parts.find((p) => p.id === selectedPartId) ?? ov.parts.find((p) => p.docs.some((d) => d.id === activeDocId));

  const toggle = (partId: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(partId)) next.delete(partId);
      else next.add(partId);
      return next;
    });

  const expand = (partId: string) =>
    setCollapsed((prev) => {
      if (!prev.has(partId)) return prev;
      const next = new Set(prev);
      next.delete(partId);
      return next;
    });

  const addChapter = () => {
    const partId = newDocPartId();
    if (partId) expand(partId);
    void addDoc({ partId });
  };

  const { drop, ghost, onDragStart, onDragOverDoc, onDropDoc, onDragOverPart, onDropPart, clearDrop, touchDrag } = useDocDrag(ov);
  const menus: SidebarMenuContext = { ov, expand };

  const numbers = new Map<string, number>();
  ov.parts.flatMap((p) => p.docs).forEach((d, i) => numbers.set(d.id, i + 1));
  const stock = stockCount(ov.parts);

  return (
    <aside className="sidebar" aria-label="탐색">
      <div className="col-head sidebar-head">
        <div className="logo small" aria-hidden="true">
          글
        </div>
        <div className="sidebar-title">
          <strong title={ov.project.title}>{ov.project.title}</strong>
          <span>
            {KIND_LABEL[kind]} ·{' '}
            {kind === 'print' ? `원고지 ${num(ov.total.manuscriptPages)}매` : `${num(ov.total.withSpaces)}자`}
            {ov.totalPages != null && ` · 예상 ${num(ov.totalPages)}쪽`}
          </span>
        </div>
        <button type="button" className="icon-btn" aria-label="작품 목록으로" title="작품 목록으로" onClick={() => void leaveProject()}>
          <Icon name="swap" size={14} />
        </button>
      </div>

      <div className="sidebar-scroll">
        <label className="sidebar-search">
          <Icon name="search" size={15} />
          <input
            type="search"
            placeholder="작품 전체 찾기"
            aria-label="작품 전체 찾기"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
                e.preventDefault();
                openFind({ text: e.currentTarget.value, scope: 'all', focus: 'find' });
              }
            }}
          />
        </label>
        <section aria-label="원고" className="tree">
          <div className="section-head">
            <span className="section-label">원고</span>
            {kind === 'webnovel' && stock > 0 && <span className="stock-chip">비축 {stock}화</span>}
            <button type="button" className="icon-btn tiny" aria-label="부 추가" title="부 추가" onClick={() => void addPart()}>
              <Icon name="plus" size={13} />
            </button>
          </div>

          {ov.parts.map((part) => {
            const open = !collapsed.has(part.id);
            const partOpts = (): MenuOptions => ({ title: part.title, subtitle: `${noun} ${part.docs.length}개` });
            return (
              <div key={part.id} className="part">
                <div className="part-row" data-part={part.id}>
                  <button
                    type="button"
                    className="part-toggle"
                    aria-label={open ? `${part.title} 접기` : `${part.title} 펼치기`}
                    aria-expanded={open}
                    onClick={() => toggle(part.id)}
                  >
                    <Icon name={open ? 'chevronDown' : 'chevronRight'} size={14} />
                  </button>
                  <button
                    type="button"
                    className={`part-head${selectedPartId === part.id ? ' selected' : ''}${
                      drop && 'partId' in drop && drop.partId === part.id ? ' drop-into' : ''
                    }`}
                    aria-pressed={selectedPartId === part.id}
                    title={`누르면 새 ${withSubject(noun)} 이 부에 들어갑니다 · 두 번 누르면 개요 표`}
                    onClick={() => {
                      selectPart(part.id);
                      expand(part.id);
                    }}
                    onDoubleClick={() => void openTable(part.id)}
                    {...pressMenu(() => partMenu(menus, part), partOpts)}
                    onDragOver={(e) => onDragOverPart(e, part)}
                    onDragLeave={clearDrop}
                    onDrop={(e) => onDropPart(e, part)}
                  >
                    <span className="part-title">{part.title}</span>
                    <span className="count">{part.docs.length}</span>
                  </button>
                  <MoreButton items={() => partMenu(menus, part)} opts={partOpts} label={`${part.title} 메뉴`} />
                </div>
                {open &&
                  withUnreadable(part.docs, unreadableIn(ov.unreadable, part.id)).map((row) => {
                    if (row.kind === 'unreadable') return <UnreadableItem key={row.item.id} item={row.item} />;
                    const doc = row.doc;
                    const label = docNumber(kind, numbers.get(doc.id) ?? 0);
                    return (
                      <DocItem
                        key={doc.id}
                        doc={doc}
                        label={label}
                        kind={kind}
                        goal={doc.target ?? ov.project.goal.perDoc}
                        countSpaces={ov.project.goal.countSpaces}
                        active={doc.id === shownDocId}
                        memos={memoCounts.get(doc.id) ?? 0}
                        copies={copyCounts.get(doc.id) ?? 0}
                        drop={drop && 'docId' in drop && drop.docId === doc.id ? (drop.after ? 'after' : 'before') : null}
                        menu={() => docMenu(menus, doc, part)}
                        touchDrag={touchDrag(doc, `${label} ${doc.title || UNTITLED}`)}
                        onDragStart={(e) => onDragStart(e, doc.id)}
                        onDragOver={(e) => onDragOverDoc(e, doc.id)}
                        onDrop={onDropDoc}
                        onDragEnd={clearDrop}
                      />
                    );
                  })}
              </div>
            );
          })}
        </section>

        <section aria-label="기획" className="tree planning">
          <div className="section-head">
            <span className="section-label">기획</span>
            <button
              type="button"
              className="icon-btn tiny"
              aria-label="새 기획 문서"
              title="새 기획 문서"
              onClick={() => void addDoc({ section: 'planning', title: '새 기획 문서' })}
            >
              <Icon name="plus" size={13} />
            </button>
          </div>
          {withUnreadable(ov.planning, unreadableIn(ov.unreadable, null)).map((row) => {
            if (row.kind === 'unreadable') return <UnreadableItem key={row.item.id} item={row.item} planning />;
            const doc = row.doc;
            const title = doc.title || UNTITLED;
            const opts = (): MenuOptions => ({ title });
            return (
              <div key={doc.id} className="plan-row" data-doc={doc.id}>
                <button
                  type="button"
                  draggable
                  className={`plan-item${doc.id === shownDocId ? ' active' : ''}${
                    drop && 'docId' in drop && drop.docId === doc.id ? (drop.after ? ' drop-after' : ' drop-before') : ''
                  }`}
                  onClick={(e) => void selectDoc(doc.id, e.ctrlKey || e.metaKey)}
                  onDoubleClick={() => void openDocInNewTab(doc.id)}
                  {...pressMenu(() => docMenu(menus, doc, null), opts, touchDrag(doc, title))}
                  onDragStart={(e) => onDragStart(e, doc.id)}
                  onDragOver={(e) => onDragOverDoc(e, doc.id)}
                  onDrop={onDropDoc}
                  onDragEnd={clearDrop}
                >
                  <Icon name="pencil" size={14} />
                  <span className="grow ellipsis">{title}</span>
                  <CopyBadge count={copyCounts.get(doc.id) ?? 0} />
                </button>
                <MoreButton items={() => docMenu(menus, doc, null)} opts={opts} label={`${title} 메뉴`} />
              </div>
            );
          })}
        </section>

        <CardsSection />

        <section aria-label="메모함" className="tree planning">
          <button
            type="button"
            className={`plan-item${notesOpen ? ' active' : ''}`}
            title="작품 메모와 모든 열린 메모"
            onClick={() => void openNotesBoard()}
          >
            <Icon name="note" size={14} />
            <span className="grow">메모함</span>
            {openNotes > 0 && <span className="count">{openNotes}</span>}
          </button>
        </section>
      </div>

      <div className="sidebar-foot">
        <button
          type="button"
          className="btn dashed grow"
          title={newPart ? `${newPart.title} 끝에 새 ${noun}` : undefined}
          onClick={addChapter}
        >
          <Icon name="plus" size={14} />새 {noun}
        </button>
        <button
          type="button"
          className="btn square"
          aria-label={`휴지통, ${ov.trashCount}개`}
          title="휴지통"
          onClick={() => openDialog({ kind: 'trash' })}
        >
          <Icon name="trash" size={15} />
          {ov.trashCount > 0 && <span className="count">{ov.trashCount}</span>}
        </button>
      </div>

      {ghost && (
        <div className="drag-ghost" style={{ left: ghost.x, top: ghost.y }} aria-hidden="true">
          {ghost.label}
        </div>
      )}
    </aside>
  );
}
