// Right column (맥락): follows what is open in the middle. Its tabs depend on
// that (docs/layout-data.md); a card preview sits over them until closed.

import { useEffect, useState } from 'react';
import type { Editor } from '@tiptap/core';
import { api } from '../api';
import type { SnapshotInfo } from '../api/types';
import { blocksFromJSON } from '../editor/counts';
import { sceneAt, scenesOf, type Scene } from '../editor/outline';
import { Icon } from '../components/Icon';
import { openMenu } from '../components/Menu';
import { num, timeLabel } from '../lib/format';
import { RECORD_KIND_LABEL, docNoun, formatName, withObject } from '../lib/labels';
import {
  applyFormat,
  findDoc,
  leaveProject,
  openDialog,
  patchSummary,
  saveEverything,
  toastError,
  useApp,
  type RightTab,
} from '../store';
import type { MenuItem } from '../components/Menu';
import { Appearances, CardPreview, CastTab } from './CardPanels';
import { NotesTab } from './Notes';
import { SearchTab } from './SearchTab';

type TabList = { id: RightTab; label: string }[];

const CHAPTER_TABS: TabList = [
  { id: 'outline', label: '개요' },
  { id: 'notes', label: '메모' },
  { id: 'cast', label: '등장 설정' },
  { id: 'records', label: '기록' },
  { id: 'find', label: '찾기' },
];
const PLANNING_TABS: TabList = [
  { id: 'outline', label: '개요' },
  { id: 'notes', label: '메모' },
  { id: 'records', label: '기록' },
  { id: 'find', label: '찾기' },
];
const CARD_TABS: TabList = [
  { id: 'cast', label: '등장 위치' },
  { id: 'notes', label: '메모' },
  { id: 'find', label: '찾기' },
];
/** 메모함 and 개요 표 show their own details in the middle. */
const BOARD_TABS: TabList = [{ id: 'find', label: '찾기' }];

export function RightPanel() {
  const rightTab = useApp((s) => s.rightTab);
  const editor = useApp((s) => s.editor);
  const activeDocId = useApp((s) => s.activeDocId);
  const activeCardId = useApp((s) => s.activeCardId);
  const board = useApp((s) => s.activeTarget?.kind === 'notes' || s.activeTarget?.kind === 'table');
  const previewCardId = useApp((s) => s.previewCardId);
  const planning = useApp((s) => (s.activeDocId ? findDoc(s.overview!, s.activeDocId)?.section === 'planning' : false));
  const kind = useApp((s) => s.overview!.project.kind);
  const tabs = board ? BOARD_TABS : activeCardId ? CARD_TABS : planning ? PLANNING_TABS : CHAPTER_TABS;
  // A tab missing here (e.g. 기록 while a card is open) shows the first one,
  // and comes back when its kind of work is open again.
  const tab = tabs.some((t) => t.id === rightTab) ? rightTab : tabs[0].id;
  const preview = previewCardId && previewCardId !== activeCardId ? previewCardId : null;

  return (
    <aside className="right" aria-label="맥락">
      <div className="col-head right-head">
        <FormatChip />
        <div className="grow" />
        <button type="button" className="btn dark" onClick={() => openDialog({ kind: 'export' })}>
          <Icon name="download" size={15} />
          내보내기
        </button>
        <button
          type="button"
          className="icon-btn"
          aria-label="더 보기"
          onClick={(e) =>
            openMenu(e, [
              { label: '작품 설정', onSelect: () => openDialog({ kind: 'project' }) },
              { label: '보기 설정', onSelect: () => openDialog({ kind: 'view' }) },
              { label: '휴지통', onSelect: () => openDialog({ kind: 'trash' }) },
              { separator: true },
              { label: '작품 목록으로', onSelect: () => void leaveProject() },
            ])
          }
        >
          <Icon name="more" />
        </button>
      </div>
      <div className="tabs" role="tablist" aria-label="맥락 패널">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={!preview && tab === t.id}
            className={`tab${!preview && tab === t.id ? ' on' : ''}`}
            onClick={() => useApp.setState({ rightTab: t.id, previewCardId: null })}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="right-body">
        {preview ? (
          <CardPreview key={preview} cardId={preview} />
        ) : tab === 'find' ? (
          <SearchTab />
        ) : activeCardId ? (
          tab === 'notes' ? (
            <NotesTab key={activeCardId} on="card" targetId={activeCardId} editor={null} />
          ) : (
            <Appearances key={activeCardId} cardId={activeCardId} />
          )
        ) : !activeDocId ? (
          <p className="empty-note">왼쪽에서 {withObject(docNoun(kind))} 고르세요.</p>
        ) : tab === 'outline' ? (
          editor ? <OutlineTab editor={editor} /> : null
        ) : tab === 'notes' ? (
          <NotesTab key={activeDocId} on="doc" targetId={activeDocId} editor={editor} />
        ) : tab === 'cast' ? (
          editor ? <CastTab editor={editor} /> : null
        ) : (
          <RecordsTab docId={activeDocId} />
        )}
      </div>
    </aside>
  );
}

/** Current manuscript format, with a quick switch between presets. */
function FormatChip() {
  const format = useApp((s) => s.overview!.project.manuscriptFormat);
  const catalog = useApp((s) => s.catalog);
  const name = formatName(format, catalog);
  const items = (): MenuItem[] => {
    const list: MenuItem[] = [{ heading: '원고 서식' }];
    for (const b of catalog?.builtin ?? []) {
      list.push({ label: b.name, checked: name === b.name, onSelect: () => void applyFormat(structuredClone(b.format)) });
    }
    for (const u of catalog?.user ?? []) {
      list.push({ label: u.name, checked: name === u.name, onSelect: () => void applyFormat(structuredClone(u.format)) });
    }
    list.push({ separator: true }, { label: '원고 서식 자세히…', onSelect: () => openDialog({ kind: 'project', tab: 'format' }) });
    return list;
  };
  return (
    <button type="button" className="format-chip" title="원고 서식: 내보낸 파일과 예상 쪽수에 쓰입니다" onClick={(e) => openMenu(e, items())}>
      <Icon name="doc" size={14} />
      <span className="ellipsis">{name}</span>
      <Icon name="chevronDown" size={12} />
    </button>
  );
}

function OutlineTab({ editor }: { editor: Editor }) {
  const [scenes, setScenes] = useState<Scene[]>(() => scenesOf(editor.state.doc));
  const [current, setCurrent] = useState(0);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const refresh = () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        const next = scenesOf(editor.state.doc);
        setScenes(next);
        setCurrent(sceneAt(next, editor.state.selection.from));
      }, 200);
    };
    refresh();
    editor.on('update', refresh);
    editor.on('selectionUpdate', refresh);
    return () => {
      clearTimeout(timer);
      editor.off('update', refresh);
      editor.off('selectionUpdate', refresh);
    };
  }, [editor]);

  const go = (scene: Scene) => {
    const pos = Math.min(scene.pos + 1, editor.state.doc.content.size);
    editor.chain().focus().setTextSelection(pos).run();
    const dom = editor.view.nodeDOM(scene.pos);
    if (dom instanceof HTMLElement) dom.scrollIntoView({ block: 'start', behavior: 'smooth' });
  };

  if (scenes.length <= 1 && !scenes[0]?.opening) {
    return <p className="empty-note">장면을 나누면 여기에 장면 목록이 생깁니다. 빈 줄에 ***를 쓰고 Enter를 누르세요.</p>;
  }

  return (
    <ol className="scene-list">
      {scenes.map((scene) => (
        <li key={scene.index}>
          <button type="button" className={`scene${scene.index === current ? ' current' : ''}`} onClick={() => go(scene)}>
            <span className="scene-no">장면 {scene.index}</span>
            <span className="scene-open">{scene.opening || '빈 장면'}</span>
            <span className="scene-count">{num(scene.chars)}자</span>
          </button>
        </li>
      ))}
    </ol>
  );
}

function RecordsTab({ docId }: { docId: string }) {
  const root = useApp((s) => s.overview!.root);
  const version = useApp((s) => s.recordsVersion);
  const [records, setRecords] = useState<SnapshotInfo[] | null>(null);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string>('');

  useEffect(() => {
    let alive = true;
    api.snapshotList(root, docId).then(
      (list) => alive && setRecords(list),
      (e) => alive && toastError('기록을 읽지 못함', e),
    );
    return () => {
      alive = false;
    };
  }, [root, docId, version]);

  useEffect(() => {
    setPreview('');
    if (!selected) return;
    let alive = true;
    api.snapshotLoad(root, docId, selected).then(
      (d) => {
        if (!alive) return;
        const text = blocksFromJSON(d.body)
          .map((b) => (b.scene ? '◆' : b.lines.join('\n')))
          .join('\n\n');
        setPreview(text.length > 1200 ? `${text.slice(0, 1200)}…` : text);
      },
      (e) => alive && toastError('기록을 읽지 못함', e),
    );
    return () => {
      alive = false;
    };
  }, [root, docId, selected]);

  const bump = () => useApp.setState((s) => ({ recordsVersion: s.recordsVersion + 1 }));

  const keep = async () => {
    if (!(await saveEverything())) return;
    try {
      await api.snapshotCreate(root, docId, name);
      setNaming(false);
      setName('');
      bump();
    } catch (e) {
      toastError('보관하지 못함', e);
    }
  };

  const restore = (record: SnapshotInfo) =>
    openDialog({
      kind: 'confirm',
      title: '이 때로 되돌리기',
      message: `${timeLabel(record.at)}의 원고로 되돌립니다. 지금 원고는 '되돌리기 전' 기록으로 남아서 언제든 다시 돌아올 수 있습니다.`,
      confirm: '되돌리기',
      onConfirm: async () => {
        if (!(await saveEverything())) return;
        try {
          await api.snapshotRestore(root, docId, record.id);
          const reloaded = await api.docLoad(root, docId);
          patchSummary(docId, { counts: reloaded.counts });
          setSelected(null);
          useApp.setState((s) => ({ docVersion: s.docVersion + 1, recordsVersion: s.recordsVersion + 1 }));
        } catch (e) {
          toastError('되돌리지 못함', e);
        }
      },
    });

  const ov = useApp((s) => s.overview!);
  const current = findDoc(ov, docId)?.doc.counts.withSpaces ?? 0;

  return (
    <div className="records">
      {naming ? (
        <form
          className="keep-form"
          onSubmit={(e) => {
            e.preventDefault();
            void keep();
          }}
        >
          <input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="이름 (예: 1교 보내기 전)" aria-label="기록 이름" />
          <button type="submit" className="btn primary small">
            보관
          </button>
          <button type="button" className="btn small" onClick={() => setNaming(false)}>
            취소
          </button>
        </form>
      ) : (
        <button type="button" className="btn" onClick={() => setNaming(true)}>
          <Icon name="clock" size={15} />
          지금 원고 보관
        </button>
      )}

      {records !== null && records.length === 0 && (
        <p className="empty-note">아직 기록이 없습니다. 쓰는 동안 10분마다 자동으로 남고, 원할 때 직접 보관할 수 있습니다.</p>
      )}

      <ul className="record-list">
        {records?.map((r) => {
          const diff = r.counts.withSpaces - current;
          return (
            <li key={r.id}>
              <button
                type="button"
                className={`record${selected === r.id ? ' on' : ''}`}
                onClick={() => setSelected(selected === r.id ? null : r.id)}
                aria-expanded={selected === r.id}
              >
                <span className="record-top">
                  <strong>{timeLabel(r.at)}</strong>
                  <span className={`record-kind kind-${r.kind}`}>{RECORD_KIND_LABEL[r.kind]}</span>
                </span>
                {r.name && <span className="record-name">{r.name}</span>}
                <span className="record-count">
                  {num(r.counts.withSpaces)}자{diff !== 0 && ` · 지금보다 ${num(Math.abs(diff))}자 ${diff > 0 ? '많음' : '적음'}`}
                </span>
              </button>
              {selected === r.id && (
                <div className="record-preview">
                  <p>{preview || ' '}</p>
                  <button type="button" className="btn small" onClick={() => restore(r)}>
                    <Icon name="restore" size={14} />이 때로 되돌리기
                  </button>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
