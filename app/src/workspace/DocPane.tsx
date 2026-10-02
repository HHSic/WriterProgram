// A document in a tab of the middle column: title, synopsis and the editor.

import { useEffect, useMemo, useRef, useState } from 'react';
import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react';
import { BubbleMenu } from '@tiptap/react/menus';
import { NodeSelection, TextSelection } from '@tiptap/pm/state';
import { api } from '../api';
import type { DocData, MetaPatch } from '../api/types';
import { setCardNames } from '../editor/cards';
import { setIndentRules } from '../editor/indent';
import { MARK_BUTTONS, activeMarks, toggleMark, type MarkKey } from '../editor/markButtons';
import { setWhitespaceMarks } from '../editor/marks';
import { blocksFromNode, countBlocks, countChars } from '../editor/counts';
import { manuscriptExtensions } from '../editor/extensions';
import { showMatch } from '../editor/search';
import type { SaveSession } from '../editor/session';
import { RELOAD, attach, peerOf } from '../editor/shared';
import { useAutoHeight } from '../lib/autoHeight';
import { loadCursor, saveCursor } from '../lib/cursor';
import { errorText } from '../lib/format';
import { registerFlusher } from '../lib/flush';
import { UNTITLED, docNoun, withObject } from '../lib/labels';
import { selectNote } from '../editor/notes';
import {
  addTextNote,
  findDoc,
  focusNote,
  isFocusedTab,
  patchSummary,
  previewCard,
  registerEditor,
  toastError,
  useApp,
} from '../store';
import { touchCapable, touchLike } from '../lib/pointer';
import { useNameIndex } from './CardPanels';
import { DocBanners } from './Copies';
import { EditToolbar } from './EditToolbar';
import { ReadingBar } from './ReadingBar';
import { Ruler } from './Ruler';

export function DocPane({ docId, tabKey, locked }: { docId: string; tabKey: string; locked: boolean }) {
  const root = useApp((s) => s.overview!.root);
  const version = useApp((s) => s.docVersion);
  const [data, setData] = useState<DocData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [slow, setSlow] = useState(false);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setSlow(false);
    // Only show "불러오는 중" when loading is not instant, to avoid a flash.
    const timer = setTimeout(() => alive && setSlow(true), 150);
    api.docLoad(root, docId).then(
      (d) => alive && setData(d),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [root, docId, version]);

  if (error) return <div className="pane-message">이 문서를 열 수 없음 · {error}</div>;
  if (!data) return slow ? <div className="pane-message">불러오는 중</div> : <div className="pane-message" />;
  return <LoadedDoc key={`${docId}:${version}`} root={root} data={data} tabKey={tabKey} locked={locked} />;
}

function LoadedDoc({ root, data, tabKey, locked }: { root: string; data: DocData; tabKey: string; locked: boolean }) {
  const docId = data.meta.id;
  const projectId = useApp((s) => s.overview!.project.id);
  const sceneSymbol = useApp((s) => s.overview!.project.sceneBreak);
  const isPlanning = useApp((s) => findDoc(s.overview!, docId)?.section === 'planning');
  const sessionRef = useRef<SaveSession | null>(null);
  const countTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const cursorTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const extensions = useMemo(
    () =>
      manuscriptExtensions(sceneSymbol, {
        onCardOpen: previewCard,
        onNoteOpen: focusNote,
        onNoteAdd: () => void addTextNote(),
      }),
    [sceneSymbol],
  );
  const names = useNameIndex();

  const editor = useEditor(
    {
      extensions,
      // The same document open in the other pane may have edits not saved yet.
      content: peerOf(docId)?.getJSON() ?? data.body,
      editable: !locked,
      immediatelyRender: true,
      shouldRerenderOnTransaction: false,
      injectCSS: false,
      enableInputRules: false,
      enablePasteRules: false,
      editorProps: {
        attributes: { class: 'manuscript', spellcheck: 'false', lang: 'ko', 'aria-label': '본문' },
      },
      onUpdate: ({ editor, transaction }) => {
        // Text loaded from disk (another device's) needs no saving.
        if (!transaction.getMeta(RELOAD)) sessionRef.current?.changed();
        clearTimeout(countTimer.current);
        countTimer.current = setTimeout(() => {
          if (isFocusedTab(tabKey)) useApp.setState({ liveCounts: countBlocks(blocksFromNode(editor.state.doc)) });
        }, 150);
      },
      onSelectionUpdate: ({ editor }) => {
        const { from, to, empty } = editor.state.selection;
        if (isFocusedTab(tabKey)) {
          useApp.setState({ selection: empty ? null : countChars(editor.state.doc.textBetween(from, to, '\n', '\n')) });
        }
        clearTimeout(cursorTimer.current);
        cursorTimer.current = setTimeout(() => saveCursor(projectId, docId, from, to), 400);
      },
      // One editor per mount: this component is keyed by document and version.
    },
    [],
  );

  useEffect(() => {
    const { session, detach } = attach(
      root,
      docId,
      editor,
      (outcome) => {
        patchSummary(docId, { counts: outcome.counts, pages: outcome.pages, modified: new Date().toISOString() });
        if (outcome.snapshot) useApp.setState((s) => ({ recordsVersion: s.recordsVersion + 1 }));
      },
      data.rev,
    );
    sessionRef.current = session;
    registerEditor(tabKey, editor, true);
    return () => {
      sessionRef.current = null;
      clearTimeout(countTimer.current);
      clearTimeout(cursorTimer.current);
      detach();
      registerEditor(tabKey, editor, false);
    };
  }, [root, docId, tabKey, editor]); // eslint-disable-line react-hooks/exhaustive-deps

  // While another device's edits wait for the writer's pick, nothing more is typed.
  const conflict = useApp((s) => !!s.conflicts[docId]);
  const editable = !locked && !conflict;
  useEffect(() => {
    if (editor.isEditable !== editable) editor.setEditable(editable, false);
  }, [editor, editable]);

  useEffect(() => setCardNames(editor, names), [editor, names]);

  const showMarks = useApp((s) => s.view.showMarks);
  useEffect(() => setWhitespaceMarks(editor, showMarks), [editor, showMarks]);

  // The manuscript format's first-line rules, for chapters (planning documents follow no rules).
  const rules = useApp((s) => s.overview!.project.manuscriptFormat.indentRules);
  useEffect(() => setIndentRules(editor, isPlanning ? null : (rules ?? null)), [editor, rules, isPlanning]);

  // Back where the cursor was last time, unless going somewhere on purpose.
  useEffect(() => {
    const jump = useApp.getState().pendingJump;
    if (jump?.docId === docId) return;
    const saved = loadCursor(projectId, docId);
    if (!saved) return;
    const size = editor.state.doc.content.size;
    const from = Math.min(saved.from, size);
    const to = Math.min(saved.to, size);
    try {
      editor.view.dispatch(editor.state.tr.setSelection(TextSelection.create(editor.state.doc, from, to)));
    } catch {
      return;
    }
    const timer = setTimeout(() => {
      if (editor.isDestroyed) return;
      const dom = editor.view.domAtPos(from).node;
      const el = dom instanceof HTMLElement ? dom : dom.parentElement;
      el?.scrollIntoView({ block: 'center' });
    }, 0);
    return () => clearTimeout(timer);
  }, [editor, projectId, docId]);

  // Select a note's text asked for from 메모함 once this document is open.
  const pendingNote = useApp((s) => s.pendingNote);
  useEffect(() => {
    if (!pendingNote || !isFocusedTab(tabKey)) return;
    const timer = setTimeout(() => {
      if (editor.isDestroyed) return;
      selectNote(editor, pendingNote);
      useApp.setState({ pendingNote: null });
    }, 0);
    return () => clearTimeout(timer);
  }, [pendingNote, tabKey, editor]);

  // Move to a place asked for by the find panel once this document is open.
  const jump = useApp((s) => s.pendingJump);
  useEffect(() => {
    if (!jump || jump.docId !== docId || !isFocusedTab(tabKey)) return;
    // The editor is usually on screen by now; if not, try again shortly.
    // (Timers, not animation frames: those stop while the window is hidden.)
    let timer: ReturnType<typeof setTimeout> | undefined;
    let tries = 0;
    const attempt = () => {
      tries += 1;
      if (!editor.isDestroyed && editor.view.dom.isConnected) {
        showMatch(editor, jump.block, jump.start, jump.end);
        useApp.setState({ pendingJump: null });
      } else if (tries < 30) {
        timer = setTimeout(attempt, 30);
      }
    };
    attempt();
    return () => clearTimeout(timer);
  }, [jump, docId, tabKey, editor]);

  const toolbarMode = useApp((s) => s.view.toolbar);
  const showRuler = useApp((s) => s.view.ruler);
  const toolbar = toolbarMode === 'always' || (toolbarMode === 'auto' && touchCapable());

  return (
    <>
      <DocBanners docId={docId} />
      <div className="doc-scroll">
        {showRuler && <Ruler editor={editor} planning={isPlanning} />}
        <article className="page">
          <DocHeader root={root} data={data} editor={editor} planning={isPlanning} locked={locked} />
          <EditorContent editor={editor} />
        </article>
        <FormatBubble editor={editor} />
        <ReadingBar editor={editor} />
      </div>
      {toolbar && <EditToolbar editor={editor} />}
    </>
  );
}

function DocHeader({
  root,
  data,
  editor,
  planning,
  locked,
}: {
  root: string;
  data: DocData;
  editor: Editor;
  planning: boolean;
  locked: boolean;
}) {
  const docId = data.meta.id;
  const kind = useApp((s) => s.overview!.project.kind);
  // The overview has the latest title and synopsis, also when they are
  // changed in the other pane.
  const summary = useApp((s) => findDoc(s.overview!, docId)?.doc);
  const [title, setTitle] = useState(data.meta.title);
  const [synopsis, setSynopsis] = useState(data.meta.synopsis);
  const pending = useRef<MetaPatch>({});
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const titleRef = useRef<HTMLInputElement>(null);
  const synopsisRef = useRef<HTMLTextAreaElement>(null);
  const flushRef = useRef<() => Promise<void>>(async () => {});

  useEffect(() => {
    if (summary && document.activeElement !== titleRef.current) setTitle(summary.title);
  }, [summary?.title]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (summary && document.activeElement !== synopsisRef.current) setSynopsis(summary.synopsis);
  }, [summary?.synopsis]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const flush = async () => {
      clearTimeout(timer.current);
      const patch = pending.current;
      pending.current = {};
      if (Object.keys(patch).length === 0) return;
      try {
        await api.docUpdateMeta(root, docId, patch);
      } catch (e) {
        toastError('제목·시놉시스를 저장하지 못함', e);
      }
    };
    flushRef.current = flush;
    const unregister = registerFlusher(flush);
    return () => {
      unregister();
      void flush();
    };
  }, [root, docId]);

  useAutoHeight(synopsisRef, synopsis);

  const change = (patch: MetaPatch) => {
    Object.assign(pending.current, patch);
    patchSummary(docId, patch);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void flushRef.current(), 600);
  };

  return (
    <header className="doc-head">
      <input
        ref={titleRef}
        className="doc-title-input"
        value={title}
        placeholder={UNTITLED}
        aria-label="제목"
        autoFocus={!data.meta.title && !locked}
        readOnly={locked}
        maxLength={200}
        onChange={(e) => {
          setTitle(e.target.value);
          change({ title: e.target.value });
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
            e.preventDefault();
            if (planning) editor.commands.focus('start');
            else synopsisRef.current?.focus();
          }
        }}
      />
      {!planning && (
        <label className="synopsis">
          <span className="synopsis-label">시놉시스</span>
          <textarea
            ref={synopsisRef}
            rows={1}
            value={synopsis}
            readOnly={locked}
            placeholder={`이 ${withObject(docNoun(kind))} 한두 줄로`}
            onChange={(e) => {
              setSynopsis(e.target.value);
              change({ synopsis: e.target.value });
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault();
                editor.commands.focus('start');
              }
            }}
          />
        </label>
      )}
    </header>
  );
}

/** The marks in the floating bar, in its order. */
const BUBBLE_MARKS: MarkKey[] = ['bold', 'italic', 'underline', 'strike', 'dot'];

/** Small toolbar that appears over selected text. */
function FormatBubble({ editor }: { editor: Editor }) {
  const active = useEditorState({ editor, selector: ({ editor: e }) => activeMarks(e) });

  return (
    <BubbleMenu
      editor={editor}
      className="bubble"
      options={{ placement: 'top', offset: 8 }}
      shouldShow={({ state, editor: e, view }) =>
        // After a finger the phone shows its own selection menu; the tools are
        // in the 편집 도구줄 instead.
        !touchLike() &&
        e.isEditable &&
        view.hasFocus() &&
        !state.selection.empty &&
        !(state.selection instanceof NodeSelection)
      }
    >
      <button
        type="button"
        className="bubble-memo"
        aria-label="메모 달기 (Ctrl+Alt+M)"
        title="메모 달기 (Ctrl+Alt+M)"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => void addTextNote()}
      >
        메모
      </button>
      <span className="bubble-sep" aria-hidden="true" />
      {BUBBLE_MARKS.map((key) => {
        const b = MARK_BUTTONS[key];
        const label = `${b.name} (${b.keys})`;
        return (
          <button
            key={key}
            type="button"
            className={active[key] ? 'on' : ''}
            aria-label={label}
            aria-pressed={active[key]}
            title={label}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => toggleMark(editor, key)}
          >
            {b.glyph}
          </button>
        );
      })}
    </BubbleMenu>
  );
}
