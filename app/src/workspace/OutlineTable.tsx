// 개요 표 (S14): the chapters of one part as a table. Title, synopsis and
// status are edited in the cells; rows are dragged to change the order, and a
// double click opens the chapter.

import { useEffect, useRef, useState, type DragEvent, type ReactNode } from 'react';
import { api } from '../api';
import type { DocStatus, DocSummary, MetaPatch } from '../api/types';
import { Icon } from '../components/Icon';
import { openMenu, type MenuItem } from '../components/Menu';
import { MoreButton } from '../components/MoreButton';
import { pressMenu } from '../lib/press';
import { num, timeLabel } from '../lib/format';
import { registerFlusher } from '../lib/flush';
import { STATUS_LABEL, UNTITLED, docNoun, docNumber, statusesFor, withObject, withSubject } from '../lib/labels';
import {
  moveDoc,
  openNoteCounts,
  patchSummary,
  selectDoc,
  setStatus,
  showInTab,
  toastError,
  useApp,
} from '../store';

const ROW_DRAG = 'application/x-writer-row';
const HIDDEN_KEY = 'wp.tableHidden';

const COLUMNS = [
  { id: 'synopsis', label: '시놉시스' },
  { id: 'status', label: '상태' },
  { id: 'length', label: '분량' },
  { id: 'notes', label: '열린 메모' },
  { id: 'modified', label: '마지막 수정' },
] as const;

type ColumnId = (typeof COLUMNS)[number]['id'];

function loadHidden(): Set<ColumnId> {
  try {
    const raw = JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? '[]') as unknown;
    return new Set(Array.isArray(raw) ? (raw.filter((c) => COLUMNS.some((k) => k.id === c)) as ColumnId[]) : []);
  } catch {
    return new Set();
  }
}

export function OutlineTable({ partId }: { partId: string }) {
  const ov = useApp((s) => s.overview)!;
  const notes = useApp((s) => s.notes);
  const [hidden, setHidden] = useState<Set<ColumnId>>(loadHidden);
  const [filter, setFilter] = useState<DocStatus | null>(null);
  const [drop, setDrop] = useState<{ id: string; after: boolean } | null>(null);
  const part = ov.parts.find((p) => p.id === partId);
  if (!part) return <div className="pane-message">이 부를 찾을 수 없습니다.</div>;

  const kind = ov.project.kind;
  const noun = docNoun(kind);
  const numbers = new Map<string, number>();
  ov.parts.flatMap((p) => p.docs).forEach((d, i) => numbers.set(d.id, i + 1));
  const memos = openNoteCounts(notes);
  const rows = part.docs.filter((d) => !filter || d.status === filter);
  const show = (c: ColumnId) => !hidden.has(c);
  const chars = (d: DocSummary) => (ov.project.goal.countSpaces ? d.counts.withSpaces : d.counts.withoutSpaces);
  const byStatus = statusesFor(kind)
    .map((s) => [s, part.docs.filter((d) => d.status === s).length] as const)
    .filter(([, n]) => n > 0);

  const toggleColumn = (c: ColumnId) => {
    const next = new Set(hidden);
    if (next.has(c)) next.delete(c);
    else next.add(c);
    setHidden(next);
    try {
      localStorage.setItem(HIDDEN_KEY, JSON.stringify([...next]));
    } catch {
      // Not critical.
    }
  };

  const onDrop = (e: DragEvent, target: DocSummary) => {
    e.preventDefault();
    const dragged = e.dataTransfer.getData(ROW_DRAG);
    const at = drop;
    setDrop(null);
    if (!dragged || !at || dragged === target.id) return;
    const rest = part.docs.map((d) => d.id).filter((id) => id !== dragged);
    void moveDoc(dragged, part.id, rest.indexOf(target.id) + (at.after ? 1 : 0));
  };

  return (
    <div className="doc-scroll">
      <div className="board outline-table">
        <header className="board-head">
          <div>
            <h1>개요 표</h1>
            <p className="meta">
              {noun} {num(part.docs.length)}개 · {num(part.docs.reduce((t, d) => t + chars(d), 0))}자
              {byStatus.map(([s, n]) => ` · ${STATUS_LABEL[s]} ${n}`).join('')}
            </p>
          </div>
          <span className="grow" />
          <select value={part.id} aria-label="부" onChange={(e) => showInTab({ kind: 'table', id: e.target.value })}>
            {ov.parts.map((p) => (
              <option key={p.id} value={p.id}>
                {p.title}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="btn small"
            onClick={(e) =>
              openMenu(
                e,
                COLUMNS.map((c) => ({ label: c.label, checked: show(c.id), onSelect: () => toggleColumn(c.id) })),
              )
            }
          >
            열 고르기
            <Icon name="chevronDown" size={12} />
          </button>
        </header>

        {byStatus.length > 1 && (
          <div className="tag-filter" role="group" aria-label="상태로 거르기">
            <button type="button" className={`chip-toggle${filter === null ? ' on' : ''}`} onClick={() => setFilter(null)}>
              모두
            </button>
            {byStatus.map(([s, n]) => (
              <button
                key={s}
                type="button"
                className={`chip-toggle${filter === s ? ' on' : ''}`}
                onClick={() => setFilter(filter === s ? null : s)}
              >
                {STATUS_LABEL[s]} {n}
              </button>
            ))}
          </div>
        )}

        {part.docs.length === 0 ? (
          <p className="empty-note">이 부에는 아직 {withSubject(noun)} 없습니다.</p>
        ) : (
          <div className="table-wrap">
            <table className="otable">
              <thead>
                <tr>
                  <th className="c-no">번호</th>
                  <th className="c-title">제목</th>
                  {show('synopsis') && <th className="c-synopsis">시놉시스</th>}
                  {show('status') && <th className="c-status">상태</th>}
                  {show('length') && <th className="c-length">분량</th>}
                  {show('notes') && <th className="c-notes">열린 메모</th>}
                  {show('modified') && <th className="c-modified">마지막 수정</th>}
                </tr>
              </thead>
              <tbody>
                {rows.map((d) => {
                  const goal = d.target ?? ov.project.goal.perDoc;
                  const n = chars(d);
                  return (
                    <Row
                      key={d.id}
                      doc={d}
                      label={docNumber(kind, numbers.get(d.id) ?? 0)}
                      menu={() => {
                        const i = part.docs.findIndex((x) => x.id === d.id);
                        return [
                          { label: '열기', onSelect: () => void selectDoc(d.id) },
                          { label: '새 탭에서 열기', onSelect: () => void selectDoc(d.id, true) },
                          { separator: true },
                          { label: '위로 옮기기', disabled: i <= 0, onSelect: () => void moveDoc(d.id, part.id, i - 1) },
                          {
                            label: '아래로 옮기기',
                            disabled: i >= part.docs.length - 1,
                            onSelect: () => void moveDoc(d.id, part.id, i + 1),
                          },
                        ];
                      }}
                      synopsisCell={show('synopsis')}
                      drop={drop?.id === d.id ? (drop.after ? 'after' : 'before') : null}
                      onDragStart={(e) => {
                        e.dataTransfer.setData(ROW_DRAG, d.id);
                        e.dataTransfer.effectAllowed = 'move';
                      }}
                      onDragOver={(e) => {
                        if (!e.dataTransfer.types.includes(ROW_DRAG)) return;
                        e.preventDefault();
                        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                        setDrop({ id: d.id, after: e.clientY > rect.top + rect.height / 2 });
                      }}
                      onDrop={(e) => onDrop(e, d)}
                      onDragEnd={() => setDrop(null)}
                    >
                      {show('status') && (
                        <td className="c-status">
                          <select
                            className={`cell-select status-${d.status}`}
                            value={d.status}
                            aria-label="상태"
                            onChange={(e) => void setStatus(d.id, e.target.value as DocStatus)}
                          >
                            {statusesFor(kind).map((s) => (
                              <option key={s} value={s}>
                                {STATUS_LABEL[s]}
                              </option>
                            ))}
                          </select>
                        </td>
                      )}
                      {show('length') && (
                        <td className="c-length">
                          <span className="num-text">
                            {num(n)}
                            {goal ? <span className="meta"> / {num(goal)}</span> : null}
                          </span>
                          {goal ? (
                            <span className="bar" aria-hidden="true">
                              <span
                                className={`bar-fill${n >= goal ? ' full' : ''}`}
                                style={{ width: `${Math.min(100, Math.round((n / goal) * 100))}%` }}
                              />
                            </span>
                          ) : null}
                        </td>
                      )}
                      {show('notes') && <td className="c-notes">{memos.get(d.id) ? num(memos.get(d.id)!) : ''}</td>}
                      {show('modified') && <td className="c-modified meta">{d.modified ? timeLabel(d.modified) : ''}</td>}
                    </Row>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
        <p className="hint">행을 끌어 순서를 바꾸고, 두 번 누르면 그 {withObject(noun)} 엽니다.</p>
      </div>
    </div>
  );
}

/** One chapter: number, title and synopsis cells saved as they are typed. */
function Row({
  doc,
  label,
  menu,
  synopsisCell,
  drop,
  children,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
}: {
  doc: DocSummary;
  label: string;
  menu: () => MenuItem[];
  synopsisCell: boolean;
  drop: 'before' | 'after' | null;
  children: ReactNode;
  onDragStart: (e: DragEvent) => void;
  onDragOver: (e: DragEvent) => void;
  onDrop: (e: DragEvent) => void;
  onDragEnd: () => void;
}) {
  const root = useApp((s) => s.overview!.root);
  const [title, setTitle] = useState(doc.title);
  const [synopsis, setSynopsis] = useState(doc.synopsis);
  const titleRef = useRef<HTMLInputElement>(null);
  const synopsisRef = useRef<HTMLTextAreaElement>(null);
  const pending = useRef<MetaPatch>({});
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const flushRef = useRef<() => Promise<void>>(async () => {});

  // Changes made elsewhere (the chapter's own header) come in when not typing here.
  useEffect(() => {
    if (document.activeElement !== titleRef.current) setTitle(doc.title);
  }, [doc.title]);
  useEffect(() => {
    if (document.activeElement !== synopsisRef.current) setSynopsis(doc.synopsis);
  }, [doc.synopsis]);

  useEffect(() => {
    const flush = async () => {
      clearTimeout(timer.current);
      const patch = pending.current;
      pending.current = {};
      if (!Object.keys(patch).length) return;
      try {
        await api.docUpdateMeta(root, doc.id, patch);
      } catch (e) {
        toastError('개요 표를 저장하지 못함', e);
      }
    };
    flushRef.current = flush;
    const unregister = registerFlusher(flush);
    return () => {
      unregister();
      void flush();
    };
  }, [root, doc.id]);

  const change = (patch: MetaPatch) => {
    Object.assign(pending.current, patch);
    patchSummary(doc.id, patch);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void flushRef.current(), 600);
  };

  return (
    <tr
      className={drop ? `drop-${drop}` : undefined}
      draggable
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
      onDoubleClick={(e) => {
        if ((e.target as HTMLElement).closest('input, textarea, select')) return;
        void selectDoc(doc.id);
      }}
    >
      <td className="c-no" {...pressMenu(menu, () => ({ title: `${label} ${doc.title || UNTITLED}` }))}>
        <span className="c-no-inner">
          <button type="button" className="link-btn" title="열기" onClick={() => void selectDoc(doc.id)}>
            {label}
          </button>
          <MoreButton items={menu} opts={() => ({ title: `${label} ${doc.title || UNTITLED}` })} label={`${label} 메뉴`} />
        </span>
      </td>
      <td className="c-title">
        <input
          ref={titleRef}
          className="cell-input"
          value={title}
          placeholder={UNTITLED}
          aria-label={`${label} 제목`}
          maxLength={200}
          onChange={(e) => {
            setTitle(e.target.value);
            change({ title: e.target.value });
          }}
        />
      </td>
      {synopsisCell && (
        <td className="c-synopsis">
          <textarea
            ref={synopsisRef}
            className="cell-input"
            rows={2}
            value={synopsis}
            placeholder="한두 줄로"
            aria-label={`${label} 시놉시스`}
            onChange={(e) => {
              setSynopsis(e.target.value);
              change({ synopsis: e.target.value });
            }}
          />
        </td>
      )}
      {children}
    </tr>
  );
}
