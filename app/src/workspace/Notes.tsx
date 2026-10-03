// 메모: a note as a small card with its replies and tags (NoteItem), the 메모
// tab for the open document or card (NotesTab), and 메모함 in the middle
// (NotesBoard). See docs/layout-data.md "메모".

import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import type { Editor } from '@tiptap/core';
import { api } from '../api';
import type { Note } from '../api/types';
import { Icon } from '../components/Icon';
import { openMenu } from '../components/Menu';
import { markStarts, markedText, selectNote } from '../editor/notes';
import { useAutoHeight } from '../lib/autoHeight';
import { errorText, num, timeLabel } from '../lib/format';
import { UNTITLED, docNoun, docNumber } from '../lib/labels';
import { useDebouncedSave } from '../lib/useDebouncedSave';
import {
  addNote,
  findDoc,
  focusNote,
  openCard,
  patchNote,
  selectDoc,
  showNote,
  trashNote,
  useApp,
} from '../store';

/** Suggested tags (docs/layout-data.md). */
const TAG_SUGGESTIONS = ['할 일', '복선', '자료조사', '퇴고', '아이디어'];

function AutoTextarea({
  value,
  onChange,
  className,
  placeholder,
  label,
  autoFocus,
  onKeyDown,
  minRows = 1,
}: {
  value: string;
  onChange: (value: string) => void;
  className: string;
  placeholder: string;
  label: string;
  autoFocus?: boolean;
  onKeyDown?: (e: KeyboardEvent<HTMLTextAreaElement>) => void;
  minRows?: number;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useAutoHeight(ref, value);
  return (
    <textarea
      ref={ref}
      className={className}
      value={value}
      rows={minRows}
      placeholder={placeholder}
      aria-label={label}
      autoFocus={autoFocus}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={onKeyDown}
    />
  );
}

/** A note as plain text, for its rescue copy. */
function noteText(note: Note): string {
  const lines = [note.text];
  if (note.quote) lines.unshift(`> ${note.quote}`, '');
  for (const r of note.replies) lines.push('', `- ${r.text}`);
  if (note.tags.length) lines.push('', note.tags.map((t) => `#${t}`).join(' '));
  return `${lines.join('\n')}\n`;
}

/** One note, edited in place; saves itself shortly after each change. */
export function NoteItem({
  note,
  editor,
  focused,
  head,
}: {
  note: Note;
  /** The editor showing the note's document, to read its marked text now. */
  editor: Editor | null;
  focused: boolean;
  head?: ReactNode;
}) {
  const root = useApp((s) => s.overview!.root);
  const notes = useApp((s) => s.notes);
  const allTags = useMemo(() => notes.flatMap((n) => n.tags), [notes]);
  const [draft, setDraft] = useState(note);
  const [reply, setReply] = useState('');
  const [tag, setTag] = useState('');
  const [error, setError] = useState<string | null>(null);
  const box = useRef<HTMLElement>(null);
  const { pending, schedule } = useDebouncedSave<Note>(
    {
      key: `note:${note.id}`,
      save: async (next) => {
        patchNote(await api.noteSave(root, next));
      },
      onSaved: () => setError(null),
      onError: (e) => setError(errorText(e)),
      rescue: (value) => ({ item: `note-${value.id}`, content: { text: noteText(value) } }),
    },
    [root],
  );

  // Take changes saved elsewhere (the same note open in 메모함 and the tab).
  useEffect(() => {
    if (!pending.current) setDraft(note);
  }, [note]);

  useEffect(() => {
    if (focused) box.current?.scrollIntoView({ block: 'nearest' });
  }, [focused]);

  const now = note.anchor === 'text' && editor ? markedText(editor.state.doc, note.id) : undefined;
  const gone = now === null;

  const change = (patch: Partial<Note>, soon = false) => {
    const next = { ...draft, ...patch };
    // Keep the saved text of the marked stretch up to date.
    if (typeof now === 'string') next.quote = now;
    setDraft(next);
    schedule(next, soon ? 0 : 600);
  };

  const addReply = () => {
    const text = reply.trim();
    if (!text) return;
    change({ replies: [...draft.replies, { at: new Date().toISOString(), text }] }, true);
    setReply('');
  };

  const addTag = (value = tag) => {
    const t = value.trim().replace(/^#+/, '').trim();
    setTag('');
    if (!t || draft.tags.includes(t)) return;
    change({ tags: [...draft.tags, t] }, true);
  };

  const suggestions = [...new Set([...TAG_SUGGESTIONS, ...allTags])].filter((t) => !draft.tags.includes(t));
  const listId = `tags-${note.id}`;

  return (
    <article
      ref={box}
      className={`note${focused ? ' focused' : ''}${draft.done ? ' done' : ''}`}
      onFocusCapture={() => {
        if (!focused) focusNote(note.id);
      }}
    >
      {head}
      {note.anchor === 'text' && (
        <button
          type="button"
          className="note-quote"
          title={gone ? '본문에서 이 구간의 글이 지워졌습니다' : '본문에서 보기'}
          onClick={() => {
            focusNote(note.id);
            if (!editor || !selectNote(editor, note.id)) void showNote(note);
          }}
        >
          {gone && <span className="note-gone">지워진 구간</span>}
          <span className="note-quote-text">{now ?? note.quote}</span>
        </button>
      )}
      <AutoTextarea
        className="note-text"
        value={draft.text}
        placeholder="메모를 적으세요"
        label="메모"
        autoFocus={focused && !note.text && !note.replies.length}
        onChange={(text) => change({ text })}
      />
      {draft.replies.length > 0 && (
        <ol className="note-replies">
          {draft.replies.map((r, i) => (
            <li key={`${r.at}-${i}`}>
              <span className="meta">{timeLabel(r.at)}</span>
              <p>{r.text}</p>
            </li>
          ))}
        </ol>
      )}
      <AutoTextarea
        className="note-reply-input"
        value={reply}
        placeholder="덧붙이기 (Enter)"
        label="덧붙이기"
        onChange={setReply}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            addReply();
          }
        }}
      />
      <div className="note-tags">
        {draft.tags.map((t) => (
          <span key={t} className="chip-tag small">
            {t}
            <button type="button" aria-label={`${t} 빼기`} onClick={() => change({ tags: draft.tags.filter((x) => x !== t) }, true)}>
              <Icon name="close" size={10} />
            </button>
          </span>
        ))}
        <input
          className="tag-input"
          value={tag}
          list={listId}
          placeholder="+ 태그"
          aria-label="태그 더하기"
          onChange={(e) => {
            const v = e.target.value;
            // Picking a suggestion from the list adds it right away.
            if (suggestions.includes(v)) addTag(v);
            else setTag(v);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
              e.preventDefault();
              addTag();
            }
          }}
          onBlur={() => addTag()}
        />
        <datalist id={listId}>
          {suggestions.map((t) => (
            <option key={t} value={t} />
          ))}
        </datalist>
      </div>
      <div className="note-foot">
        {error ? (
          <span className="warn-text" title={error}>
            저장하지 못함
          </span>
        ) : (
          <span className="meta">{timeLabel(note.updated)}</span>
        )}
        <span className="grow" />
        <button type="button" className={`btn small${draft.done ? '' : ' ghost'}`} onClick={() => change({ done: !draft.done }, true)}>
          {draft.done ? '다시 열기' : '완료'}
        </button>
        <button
          type="button"
          className="icon-btn tiny"
          aria-label="메모 메뉴"
          onClick={(e) => openMenu(e, [{ label: '휴지통으로', danger: true, onSelect: () => void trashNote(note.id) }])}
        >
          <Icon name="more" size={14} />
        </button>
      </div>
    </article>
  );
}

/** Marks the focused note's text in the editors more strongly. */
function FocusStyle({ id }: { id: string | null }) {
  if (!id || !/^[A-Za-z0-9_-]+$/.test(id)) return null;
  return <style>{`.ProseMirror mark[data-memo="${id}"] { background: var(--memo-focus); box-shadow: 0 1px 0 var(--amber); }`}</style>;
}

/** 메모 tab: notes on the open document (its text and itself), or on a card. */
export function NotesTab({ on, targetId, editor }: { on: 'doc' | 'card'; targetId: string; editor: Editor | null }) {
  const notes = useApp((s) => s.notes);
  const focusId = useApp((s) => s.focusNoteId);
  const kind = useApp((s) => s.overview!.project.kind);
  const planning = useApp((s) => (on === 'doc' ? findDoc(s.overview!, targetId)?.section === 'planning' : false));
  const [showDone, setShowDone] = useState(false);
  const [, setTick] = useState(0);

  // The order and the marked text follow the text as it is written.
  useEffect(() => {
    if (!editor) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const refresh = () => {
      clearTimeout(timer);
      timer = setTimeout(() => setTick((t) => t + 1), 300);
    };
    editor.on('update', refresh);
    return () => {
      clearTimeout(timer);
      editor.off('update', refresh);
    };
  }, [editor]);

  const mine = notes.filter((n) =>
    on === 'card' ? n.anchor === 'card' && n.target === targetId : (n.anchor === 'doc' || n.anchor === 'text') && n.target === targetId,
  );
  const starts = editor ? markStarts(editor.state.doc) : new Map<string, number>();
  const order = (n: Note) => (n.anchor === 'doc' ? -1 : (starts.get(n.id) ?? Number.MAX_SAFE_INTEGER));
  const sorted = [...mine].sort((a, b) => order(a) - order(b) || a.created.localeCompare(b.created));
  const visible = sorted.filter((n) => showDone || !n.done || n.id === focusId);
  const doneCount = mine.filter((n) => n.done).length;
  const noun = on === 'card' ? '카드' : planning ? '문서' : docNoun(kind);

  return (
    <div className="notes-tab">
      <FocusStyle id={focusId} />
      <button type="button" className="btn small" onClick={() => void addNote({ anchor: on, target: targetId })}>
        <Icon name="plus" size={13} />이 {noun}에 메모
      </button>
      {mine.length === 0 && (
        <p className="empty-note">
          {on === 'card'
            ? '이 카드에 붙은 메모가 없습니다.'
            : `본문에서 글을 고르고 떠오른 도구줄의 "메모"(또는 Ctrl+Alt+M)를 누르면 그 구간에 메모가 붙습니다. 글을 고쳐도 메모는 구간을 따라갑니다.`}
        </p>
      )}
      {visible.map((n) => (
        <NoteItem
          key={n.id}
          note={n}
          editor={editor}
          focused={n.id === focusId}
          head={n.anchor === 'doc' ? <span className="note-on">{noun} 전체에 붙은 메모</span> : undefined}
        />
      ))}
      {doneCount > 0 && (
        <label className="check small-check">
          <input type="checkbox" checked={showDone} onChange={(e) => setShowDone(e.target.checked)} />
          완료한 메모 {num(doneCount)}개도 보기
        </label>
      )}
    </div>
  );
}

interface Group {
  key: string;
  title: string;
  open?: () => void;
  notes: Note[];
}

/** 메모함: every note, grouped by what it is on. */
export function NotesBoard() {
  const ov = useApp((s) => s.overview)!;
  const notes = useApp((s) => s.notes);
  const focusId = useApp((s) => s.focusNoteId);
  const [showDone, setShowDone] = useState(false);
  const [tag, setTag] = useState<string | null>(null);
  const kind = ov.project.kind;

  const tags = useMemo(() => [...new Set(notes.flatMap((n) => n.tags))].sort((a, b) => a.localeCompare(b)), [notes]);
  const shown = notes.filter((n) => (showDone || !n.done || n.id === focusId) && (!tag || n.tags.includes(tag)));
  const openCount = notes.filter((n) => !n.done).length;

  const groups: Group[] = [];
  const project = shown.filter((n) => n.anchor === 'project');
  groups.push({ key: 'project', title: '작품 메모', notes: project });
  const onDoc = (id: string) => shown.filter((n) => (n.anchor === 'doc' || n.anchor === 'text') && n.target === id);
  let n = 0;
  const seen = new Set<string>();
  for (const part of ov.parts) {
    for (const d of part.docs) {
      n += 1;
      seen.add(d.id);
      const list = onDoc(d.id);
      if (list.length) {
        groups.push({
          key: d.id,
          title: `${docNumber(kind, n)} · ${d.title || UNTITLED}`,
          open: () => void selectDoc(d.id),
          notes: list,
        });
      }
    }
  }
  for (const d of ov.planning) {
    seen.add(d.id);
    const list = onDoc(d.id);
    if (list.length) groups.push({ key: d.id, title: `기획 · ${d.title || UNTITLED}`, open: () => void selectDoc(d.id), notes: list });
  }
  for (const c of ov.cards) {
    seen.add(c.id);
    const list = shown.filter((x) => x.anchor === 'card' && x.target === c.id);
    if (list.length) groups.push({ key: c.id, title: `설정집 · ${c.name}`, open: () => void openCard(c.id), notes: list });
  }
  const lost = shown.filter((x) => x.anchor !== 'project' && !seen.has(x.target));
  if (lost.length) groups.push({ key: 'lost', title: '지운 문서·카드의 메모', notes: lost });

  return (
    <div className="doc-scroll">
      <FocusStyle id={focusId} />
      <div className="board">
        <header className="board-head">
          <div>
            <h1>메모함</h1>
            <p className="meta">{openCount ? `아직 열린 메모 ${num(openCount)}개` : '열린 메모가 없습니다'}</p>
          </div>
          <span className="grow" />
          <button type="button" className="btn primary small" onClick={() => void addNote({ anchor: 'project' })}>
            <Icon name="plus" size={13} />
            작품 메모
          </button>
        </header>
        <div className="board-filters">
          {tags.length > 0 && (
            <div className="tag-filter" role="group" aria-label="태그로 거르기">
              <button type="button" className={`chip-toggle${tag === null ? ' on' : ''}`} onClick={() => setTag(null)}>
                모두
              </button>
              {tags.map((t) => (
                <button key={t} type="button" className={`chip-toggle${tag === t ? ' on' : ''}`} onClick={() => setTag(tag === t ? null : t)}>
                  {t}
                </button>
              ))}
            </div>
          )}
          <label className="check small-check">
            <input type="checkbox" checked={showDone} onChange={(e) => setShowDone(e.target.checked)} />
            완료한 메모도 보기
          </label>
        </div>
        {groups.map((g) =>
          g.notes.length || g.key === 'project' ? (
            <section key={g.key} className="board-group">
              <h2>
                <span className="ellipsis">{g.title}</span>
                {g.open && (
                  <button type="button" className="link-btn" onClick={g.open}>
                    열기
                  </button>
                )}
              </h2>
              {g.key === 'project' && !g.notes.length && (
                <p className="empty-note">어디에도 붙지 않은 메모입니다. 떠오른 장면이나 아이디어를 적어 두세요.</p>
              )}
              <div className="board-notes">
                {g.notes.map((note) => (
                  <NoteItem key={note.id} note={note} editor={null} focused={note.id === focusId} />
                ))}
              </div>
            </section>
          ) : null,
        )}
      </div>
    </div>
  );
}
