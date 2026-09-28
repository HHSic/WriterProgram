// Left column (탐색): the manuscript tree, planning documents and trash.
// Always the same whatever is open in the middle (docs/layout-data.md).
// Every row's menu opens by right click, by a long press, or by its ⋯ button;
// rows are reordered by mouse drag, by a long press and drag, or from the menu.

import { useState, type DragEvent } from 'react';
import type { DocSummary, PartView, ProjectKind } from '../api/types';
import { Icon } from '../components/Icon';
import { type MenuItem, type MenuOptions } from '../components/Menu';
import { MoreButton } from '../components/MoreButton';
import { num } from '../lib/format';
import { KIND_LABEL, STATUS_LABEL, UNTITLED, docNoun, docNumber, statusesFor, stockCount, withSubject } from '../lib/labels';
import { pressMenu, type TouchDrag } from '../lib/press';
import {
  addDoc,
  addNote,
  addPart,
  leaveProject,
  moveDoc,
  newDocPartId,
  openDialog,
  openDocInNewTab,
  openFind,
  openNoteCounts,
  openNotesBoard,
  openTable,
  removePart,
  renameDoc,
  renamePart,
  selectDoc,
  selectPart,
  setStatus,
  setTarget,
  trashDoc,
  useApp,
} from '../store';
import { CardsSection } from './CardsSection';
import { CopyBadge, countCopies } from './Copies';

const DRAG_TYPE = 'application/x-writer-doc';

type DropTarget = { docId: string; after: boolean } | { partId: string } | null;

/** Where a finger dragging a document is over: a row, or a part heading. */
function dropAt(x: number, y: number, dragged: string): DropTarget {
  const el = document.elementFromPoint(x, y);
  const row = el?.closest<HTMLElement>('[data-doc]');
  if (row && row.dataset.doc !== dragged) {
    const rect = row.getBoundingClientRect();
    return { docId: row.dataset.doc!, after: y > rect.top + rect.height / 2 };
  }
  const part = el?.closest<HTMLElement>('[data-part]');
  return part ? { partId: part.dataset.part! } : null;
}

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
  const [drop, setDrop] = useState<DropTarget>(null);
  const [ghost, setGhost] = useState<{ label: string; x: number; y: number } | null>(null);
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

  // Moving documents ------------------------------------------------------

  /** Moves `dragged` to a drop target: before or after a row, or to the end of a part. */
  const dropOn = (dragged: string, target: DropTarget) => {
    if (!target) return;
    const planning = ov.planning.some((d) => d.id === dragged);
    if ('partId' in target) {
      const part = ov.parts.find((p) => p.id === target.partId);
      if (part && !planning) void moveDoc(dragged, part.id, part.docs.filter((d) => d.id !== dragged).length);
      return;
    }
    if (target.docId === dragged) return;
    const part = ov.parts.find((p) => p.docs.some((d) => d.id === target.docId)) ?? null;
    // Chapters stay in the manuscript and planning documents in 기획.
    if ((part === null) !== planning) return;
    const list = part ? part.docs : ov.planning;
    const rest = list.map((d) => d.id).filter((id) => id !== dragged);
    void moveDoc(dragged, part?.id ?? null, rest.indexOf(target.docId) + (target.after ? 1 : 0));
  };

  const onDragStart = (e: DragEvent, docId: string) => {
    e.dataTransfer.setData(DRAG_TYPE, docId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const onDragOverDoc = (e: DragEvent, docId: string) => {
    if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
    e.preventDefault();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setDrop({ docId, after: e.clientY > rect.top + rect.height / 2 });
  };

  const onDropDoc = (e: DragEvent) => {
    e.preventDefault();
    const dragged = e.dataTransfer.getData(DRAG_TYPE);
    const target = drop;
    setDrop(null);
    if (dragged) dropOn(dragged, target);
  };

  const onDropPart = (e: DragEvent, part: PartView) => {
    e.preventDefault();
    setDrop(null);
    const dragged = e.dataTransfer.getData(DRAG_TYPE);
    if (dragged) dropOn(dragged, { partId: part.id });
  };

  /** A long press and drag with a finger does what a mouse drag does. */
  const touchDrag = (doc: DocSummary, label: string): TouchDrag => ({
    start: (x, y) => setGhost({ label, x, y }),
    move: (x, y) => {
      setGhost((g) => (g ? { ...g, x, y } : g));
      setDrop(dropAt(x, y, doc.id));
    },
    end: (x, y, cancelled) => {
      const target = cancelled ? null : dropAt(x, y, doc.id);
      setGhost(null);
      setDrop(null);
      dropOn(doc.id, target);
    },
  });

  const moveItems = (doc: DocSummary, part: PartView | null): MenuItem[] => {
    const list = part ? part.docs : ov.planning;
    const i = list.findIndex((d) => d.id === doc.id);
    const items: MenuItem[] = [
      { label: '위로 옮기기', disabled: i <= 0, onSelect: () => void moveDoc(doc.id, part?.id ?? null, i - 1) },
      {
        label: '아래로 옮기기',
        disabled: i >= list.length - 1,
        onSelect: () => void moveDoc(doc.id, part?.id ?? null, i + 1),
      },
    ];
    if (part && ov.parts.length > 1) {
      items.push({ heading: '다른 부로 옮기기' });
      for (const other of ov.parts) {
        if (other.id === part.id) continue;
        items.push({
          label: other.title,
          onSelect: () => {
            expand(other.id);
            void moveDoc(doc.id, other.id, other.docs.length);
          },
        });
      }
    }
    return items;
  };

  // Menus -------------------------------------------------------------------

  const docMenu = (doc: DocSummary, part: PartView | null): MenuItem[] => {
    const planning = part === null;
    const items: MenuItem[] = [
      { label: '열기', onSelect: () => void selectDoc(doc.id) },
      { label: '새 탭에서 열기', onSelect: () => void selectDoc(doc.id, true) },
      {
        label: '이름 바꾸기',
        onSelect: () =>
          openDialog({
            kind: 'prompt',
            title: '이름 바꾸기',
            label: '제목',
            value: doc.title,
            confirm: '바꾸기',
            onSubmit: (title) => renameDoc(doc.id, title),
          }),
      },
      {
        label: planning ? '아래에 새 기획 문서' : `아래에 새 ${noun}`,
        onSelect: () => void addDoc({ section: planning ? 'planning' : 'manuscript', after: doc.id }),
      },
      {
        label: planning ? '이 문서에 메모' : `이 ${noun}에 메모`,
        onSelect: () => {
          void (async () => {
            await selectDoc(doc.id);
            await addNote({ anchor: 'doc', target: doc.id });
          })();
        },
      },
    ];
    if (!planning) {
      items.push({
        label: '목표 분량 바꾸기',
        onSelect: () =>
          openDialog({
            kind: 'prompt',
            title: '목표 분량',
            label: `이 ${noun}의 목표 글자 수 (비우면 작품 기본값)`,
            value: doc.target ? String(doc.target) : '',
            confirm: '바꾸기',
            inputMode: 'numeric',
            onSubmit: (value) => {
              const n = Number.parseInt(value.replace(/[^0-9]/g, ''), 10);
              return setTarget(doc.id, Number.isFinite(n) && n > 0 ? n : null);
            },
          }),
      });
    }
    items.push({ separator: true }, ...moveItems(doc, part));
    if (!planning) {
      items.push(
        { separator: true },
        { heading: '상태' },
        ...statusesFor(kind).map(
          (status): MenuItem => ({
            label: STATUS_LABEL[status],
            checked: doc.status === status,
            onSelect: () => void setStatus(doc.id, status),
          }),
        ),
      );
    }
    items.push({ separator: true }, { label: '휴지통으로', danger: true, onSelect: () => void trashDoc(doc.id) });
    return items;
  };

  const partMenu = (part: PartView): MenuItem[] => [
    {
      label: `이 부에 새 ${noun}`,
      onSelect: () => {
        expand(part.id);
        void addDoc({ partId: part.id });
      },
    },
    { label: '개요 표로 보기', onSelect: () => void openTable(part.id) },
    {
      label: '이름 바꾸기',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: '부 이름 바꾸기',
          label: '이름',
          value: part.title,
          confirm: '바꾸기',
          onSubmit: (title) => renamePart(part.id, title),
        }),
    },
    { separator: true },
    {
      label: '부 지우기',
      danger: true,
      disabled: part.docs.length > 0 || ov.parts.length === 1,
      onSelect: () => void removePart(part.id),
    },
  ];

  // Rendering -----------------------------------------------------------------

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
                    {...pressMenu(() => partMenu(part), partOpts)}
                    onDragOver={(e) => {
                      if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
                      e.preventDefault();
                      setDrop({ partId: part.id });
                    }}
                    onDragLeave={() => setDrop(null)}
                    onDrop={(e) => onDropPart(e, part)}
                  >
                    <span className="part-title">{part.title}</span>
                    <span className="count">{part.docs.length}</span>
                  </button>
                  <MoreButton items={() => partMenu(part)} opts={partOpts} label={`${part.title} 메뉴`} />
                </div>
                {open &&
                  part.docs.map((doc) => {
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
                        menu={() => docMenu(doc, part)}
                        touchDrag={touchDrag(doc, `${label} ${doc.title || UNTITLED}`)}
                        onDragStart={(e) => onDragStart(e, doc.id)}
                        onDragOver={(e) => onDragOverDoc(e, doc.id)}
                        onDrop={onDropDoc}
                        onDragEnd={() => setDrop(null)}
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
          {ov.planning.map((doc) => {
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
                  {...pressMenu(() => docMenu(doc, null), opts, touchDrag(doc, title))}
                  onDragStart={(e) => onDragStart(e, doc.id)}
                  onDragOver={(e) => onDragOverDoc(e, doc.id)}
                  onDrop={onDropDoc}
                  onDragEnd={() => setDrop(null)}
                >
                  <Icon name="pencil" size={14} />
                  <span className="grow ellipsis">{title}</span>
                  <CopyBadge count={copyCounts.get(doc.id) ?? 0} />
                </button>
                <MoreButton items={() => docMenu(doc, null)} opts={opts} label={`${title} 메뉴`} />
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

function DocItem({
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
        aria-label={`${title}, ${STATUS_LABEL[doc.status]}, ${num(chars)}자${memos ? `, 열린 메모 ${memos}개` : ''}`}
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

