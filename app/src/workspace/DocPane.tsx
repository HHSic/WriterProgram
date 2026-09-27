// The open document in the middle column: title, synopsis and the editor.

import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react';
import { BubbleMenu } from '@tiptap/react/menus';
import { NodeSelection } from '@tiptap/pm/state';
import { api } from '../api';
import type { DocData, MetaPatch } from '../api/types';
import { blocksFromNode, countBlocks, countChars } from '../editor/counts';
import { manuscriptExtensions } from '../editor/extensions';
import { SaveSession } from '../editor/session';
import { errorText } from '../lib/format';
import { registerFlusher } from '../lib/flush';
import { UNTITLED, docNoun, withObject } from '../lib/labels';
import { findDoc, patchSummary, toastError, useApp } from '../store';

export function DocPane({ docId }: { docId: string }) {
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
  return <LoadedDoc key={`${docId}:${version}`} root={root} data={data} />;
}

function LoadedDoc({ root, data }: { root: string; data: DocData }) {
  const docId = data.meta.id;
  const sceneSymbol = useApp((s) => s.overview!.project.sceneBreak);
  const isPlanning = useApp((s) => findDoc(s.overview!, docId)?.section === 'planning');
  const sessionRef = useRef<SaveSession | null>(null);
  const countTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const extensions = useMemo(() => manuscriptExtensions(sceneSymbol), [sceneSymbol]);

  const editor = useEditor({
    extensions,
    content: data.body,
    immediatelyRender: true,
    shouldRerenderOnTransaction: false,
    injectCSS: false,
    enableInputRules: false,
    enablePasteRules: false,
    editorProps: {
      attributes: { class: 'manuscript', spellcheck: 'false', lang: 'ko', 'aria-label': '본문' },
    },
    onUpdate: ({ editor }) => {
      sessionRef.current?.changed();
      clearTimeout(countTimer.current);
      countTimer.current = setTimeout(() => {
        useApp.setState({ liveCounts: countBlocks(blocksFromNode(editor.state.doc)) });
      }, 150);
    },
    onSelectionUpdate: ({ editor }) => {
      const { from, to, empty } = editor.state.selection;
      useApp.setState({ selection: empty ? null : countChars(editor.state.doc.textBetween(from, to, '\n', '\n')) });
    },
  });

  useEffect(() => {
    const session = new SaveSession(root, docId, () => editor.getJSON(), (outcome) => {
      patchSummary(docId, { counts: outcome.counts });
      if (outcome.snapshot) useApp.setState((s) => ({ recordsVersion: s.recordsVersion + 1 }));
    });
    sessionRef.current = session;
    useApp.setState({ editor, liveCounts: data.counts, selection: null });
    return () => {
      sessionRef.current = null;
      clearTimeout(countTimer.current);
      void session.dispose();
      if (useApp.getState().editor === editor) useApp.setState({ editor: null });
    };
  }, [root, docId, editor, data.counts]);

  return (
    <div className="doc-scroll">
      <article className="page">
        <DocHeader root={root} data={data} editor={editor} planning={isPlanning} />
        <EditorContent editor={editor} />
      </article>
      <FormatBubble editor={editor} />
    </div>
  );
}

function DocHeader({ root, data, editor, planning }: { root: string; data: DocData; editor: Editor; planning: boolean }) {
  const docId = data.meta.id;
  const kind = useApp((s) => s.overview!.project.kind);
  const [title, setTitle] = useState(data.meta.title);
  const [synopsis, setSynopsis] = useState(data.meta.synopsis);
  const pending = useRef<MetaPatch>({});
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const synopsisRef = useRef<HTMLTextAreaElement>(null);
  const flushRef = useRef<() => Promise<void>>(async () => {});

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

  useEffect(() => {
    const el = synopsisRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  }, [synopsis]);

  const change = (patch: MetaPatch) => {
    Object.assign(pending.current, patch);
    patchSummary(docId, patch);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void flushRef.current(), 600);
  };

  return (
    <header className="doc-head">
      <input
        className="doc-title-input"
        value={title}
        placeholder={UNTITLED}
        aria-label="제목"
        autoFocus={!data.meta.title}
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

/** Small toolbar that appears over selected text. */
function FormatBubble({ editor }: { editor: Editor }) {
  const active = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e.isActive('bold'),
      italic: e.isActive('italic'),
      underline: e.isActive('underline'),
      strike: e.isActive('strike'),
      dot: e.isActive('dot'),
    }),
  });

  const buttons: { key: keyof typeof active; label: string; glyph: ReactNode; run: () => void }[] = [
    { key: 'bold', label: '굵게 (Ctrl+B)', glyph: <b>가</b>, run: () => editor.chain().focus().toggleBold().run() },
    { key: 'italic', label: '기울임 (Ctrl+I)', glyph: <i>가</i>, run: () => editor.chain().focus().toggleItalic().run() },
    { key: 'underline', label: '밑줄 (Ctrl+U)', glyph: <u>가</u>, run: () => editor.chain().focus().toggleUnderline().run() },
    { key: 'strike', label: '취소선 (Ctrl+Shift+S)', glyph: <s>가</s>, run: () => editor.chain().focus().toggleStrike().run() },
    { key: 'dot', label: '방점 (Ctrl+Shift+D)', glyph: <span className="dot">가</span>, run: () => editor.chain().focus().toggleDot().run() },
  ];

  return (
    <BubbleMenu
      editor={editor}
      className="bubble"
      options={{ placement: 'top', offset: 8 }}
      shouldShow={({ state, editor: e }) => e.isEditable && !state.selection.empty && !(state.selection instanceof NodeSelection)}
    >
      {buttons.map((b) => (
        <button
          key={b.key}
          type="button"
          className={active[b.key] ? 'on' : ''}
          aria-label={b.label}
          aria-pressed={active[b.key]}
          title={b.label}
          onMouseDown={(e) => e.preventDefault()}
          onClick={b.run}
        >
          {b.glyph}
        </button>
      ))}
    </BubbleMenu>
  );
}
