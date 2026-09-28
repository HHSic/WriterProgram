// The middle column's panes (분할) and their tab bars (가운데 탭). Every tab
// stays mounted while open, so switching back keeps its scroll, cursor and
// undo; only the open one is visible.

import { useEffect, useRef, useState, type DragEvent } from 'react';
import type { Overview } from '../api/types';
import { Icon, type IconName } from '../components/Icon';
import type { MenuItem } from '../components/Menu';
import { UNTITLED, docNoun, docNumber, withObject } from '../lib/labels';
import { pressMenu } from '../lib/press';
import type { Tab, Target } from '../lib/tabs';
import {
  activateTab,
  addDoc,
  closeTab,
  newDocPartId,
  findDoc,
  focusPane,
  reorderTab,
  toggleLock,
  unsplit,
  useApp,
  type Pane,
} from '../store';
import { CardEditor } from './CardEditor';
import { DocPane } from './DocPane';
import { NotesBoard } from './Notes';
import { OutlineTable } from './OutlineTable';

const TAB_DRAG = 'application/x-writer-tab';

export function tabLabel(ov: Overview, target: Target): { label: string; icon: IconName } {
  if (target.kind === 'notes') return { label: '메모함', icon: 'note' };
  if (target.kind === 'table') {
    const part = ov.parts.find((p) => p.id === target.id);
    return { label: `개요 표 · ${part?.title ?? ''}`, icon: 'table' };
  }
  if (target.kind === 'card') {
    const card = ov.cards.find((c) => c.id === target.id);
    return { label: card?.name || '이름 없음', icon: card?.cardType === 'person' ? 'person' : 'diamond' };
  }
  const place = findDoc(ov, target.id);
  if (!place) return { label: UNTITLED, icon: 'doc' };
  const title = place.doc.title || UNTITLED;
  return place.number !== null
    ? { label: `${docNumber(ov.project.kind, place.number)} ${title}`, icon: 'doc' }
    : { label: title, icon: 'pencil' };
}

export function Panes() {
  const panes = useApp((s) => s.panes);
  const focus = useApp((s) => s.focus);
  const split = useApp((s) => s.split);
  const multi = panes.length > 1;
  return (
    <div className={`panes${multi ? ` split-${split}` : ''}`}>
      {panes.map((pane, i) => (
        <PaneView key={pane.key} pane={pane} index={i} focused={multi && i === focus} multi={multi} />
      ))}
    </div>
  );
}

function PaneView({ pane, index, focused, multi }: { pane: Pane; index: number; focused: boolean; multi: boolean }) {
  const kind = useApp((s) => s.overview!.project.kind);
  const showBar = multi || pane.tabs.length > 1;
  return (
    <section
      className={`pane${focused ? ' focused' : ''}${pane.locked ? ' locked' : ''}`}
      aria-label={multi ? `${index === 0 ? '첫째' : '둘째'} 창` : undefined}
      onMouseDownCapture={() => focusPane(index)}
      onFocusCapture={() => focusPane(index)}
    >
      {showBar && <TabBar pane={pane} index={index} multi={multi} />}
      <div className="pane-body">
        {pane.tabs.length === 0 && (
          <div className="pane-message">
            <p>왼쪽에서 {withObject(docNoun(kind))} 고르거나 새로 만드세요.</p>
            <button type="button" className="btn" onClick={() => void addDoc({ partId: newDocPartId() })}>
              <Icon name="plus" size={14} />새 {docNoun(kind)}
            </button>
          </div>
        )}
        {pane.tabs.map((tab) => (
          <div key={tab.key} className={`tab-body${tab.key === pane.active ? '' : ' off'}`} aria-hidden={tab.key !== pane.active}>
            <TabContent tab={tab} locked={pane.locked} />
          </div>
        ))}
      </div>
    </section>
  );
}

function TabContent({ tab, locked }: { tab: Tab; locked: boolean }) {
  const t = tab.target;
  switch (t.kind) {
    case 'card':
      return <CardEditor key={t.id} cardId={t.id} locked={locked} />;
    case 'notes':
      return <NotesBoard />;
    case 'table':
      return <OutlineTable key={t.id} partId={t.id} />;
    case 'doc':
      return <DocPane key={t.id} docId={t.id} tabKey={tab.key} locked={locked} />;
  }
}

function TabBar({ pane, index, multi }: { pane: Pane; index: number; multi: boolean }) {
  const ov = useApp((s) => s.overview)!;
  const [dropBefore, setDropBefore] = useState<string | null>(null);
  const strip = useRef<HTMLDivElement>(null);

  // Keep the open tab in sight when there are more tabs than room.
  useEffect(() => {
    strip.current?.querySelector('.ctab.on')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }, [pane.active]);

  const tabMenu = (tab: Tab): MenuItem[] => {
    const i = pane.tabs.findIndex((t) => t.key === tab.key);
    return [
      { label: '닫기', onSelect: () => void closeTab(index, tab.key) },
      {
        label: '다른 탭 모두 닫기',
        disabled: pane.tabs.length < 2,
        onSelect: () => {
          void (async () => {
            for (const other of pane.tabs) if (other.key !== tab.key) await closeTab(index, other.key);
          })();
        },
      },
      { separator: true },
      // Moving without a mouse drag.
      { label: '왼쪽으로 옮기기', disabled: i <= 0, onSelect: () => reorderTab(index, tab.key, pane.tabs[i - 1].key) },
      {
        label: '오른쪽으로 옮기기',
        disabled: i >= pane.tabs.length - 1,
        onSelect: () => reorderTab(index, tab.key, pane.tabs[i + 2]?.key ?? null),
      },
    ];
  };

  const onDrop = (e: DragEvent, before: string | null) => {
    e.preventDefault();
    setDropBefore(null);
    const key = e.dataTransfer.getData(TAB_DRAG);
    if (key) reorderTab(index, key, before);
  };

  return (
    <div className="tab-bar">
      <div
        ref={strip}
        className="tab-strip"
        onWheel={(e) => {
          if (e.deltaY) e.currentTarget.scrollLeft += e.deltaY;
        }}
        role="tablist"
        aria-label="열린 탭"
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes(TAB_DRAG)) e.preventDefault();
        }}
        onDrop={(e) => onDrop(e, null)}
      >
        {pane.tabs.map((tab) => {
          const { label, icon } = tabLabel(ov, tab.target);
          const on = tab.key === pane.active;
          return (
            <div
              key={tab.key}
              role="tab"
              tabIndex={on ? 0 : -1}
              aria-selected={on}
              title={label}
              draggable
              className={`ctab${on ? ' on' : ''}${dropBefore === tab.key ? ' drop-before' : ''}`}
              onClick={() => activateTab(index, tab.key)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  activateTab(index, tab.key);
                }
              }}
              onMouseDown={(e) => {
                // Middle button closes; keep it from starting to scroll.
                if (e.button === 1) e.preventDefault();
              }}
              onAuxClick={(e) => {
                if (e.button === 1) void closeTab(index, tab.key);
              }}
              {...pressMenu(() => tabMenu(tab), () => ({ title: label }))}
              onDragStart={(e) => {
                e.dataTransfer.setData(TAB_DRAG, tab.key);
                e.dataTransfer.effectAllowed = 'move';
              }}
              onDragOver={(e) => {
                if (!e.dataTransfer.types.includes(TAB_DRAG)) return;
                e.preventDefault();
                e.stopPropagation();
                setDropBefore(tab.key);
              }}
              onDragLeave={() => setDropBefore(null)}
              onDrop={(e) => {
                e.stopPropagation();
                onDrop(e, tab.key);
              }}
              onDragEnd={() => setDropBefore(null)}
            >
              <Icon name={icon} size={13} />
              <span className="ctab-label">{label}</span>
              <button
                type="button"
                className="ctab-close"
                aria-label={`${label} 닫기`}
                title="닫기 (Ctrl+W)"
                onClick={(e) => {
                  e.stopPropagation();
                  void closeTab(index, tab.key);
                }}
              >
                <Icon name="close" size={11} />
              </button>
            </div>
          );
        })}
      </div>
      <button
        type="button"
        className={`icon-btn tiny${pane.locked ? ' on' : ''}`}
        aria-pressed={pane.locked}
        aria-label={pane.locked ? '읽기 전용 풀기' : '읽기 전용으로 잠그기'}
        title={pane.locked ? '읽기 전용 풀기' : '읽기 전용으로 잠그기 (보기만 하고 고치지 않게)'}
        onClick={() => toggleLock(index)}
      >
        <Icon name={pane.locked ? 'lock' : 'unlock'} size={13} />
      </button>
      {multi && (
        <button
          type="button"
          className="icon-btn tiny"
          aria-label="이 창 닫기"
          title="이 창 닫기 (열린 탭은 다른 창으로 옮김)"
          onClick={() => {
            focusPane(index === 0 ? 1 : 0);
            void unsplit();
          }}
        >
          <Icon name="close" size={13} />
        </button>
      )}
    </div>
  );
}
