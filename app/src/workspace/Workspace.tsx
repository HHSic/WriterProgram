import { useEffect, type CSSProperties } from 'react';
import { keyInComposition } from '../editor/composition';
import { flushAll } from '../lib/flush';
import { keysText, matches, shortcutOf, type ShortcutId } from '../lib/shortcuts';
import { viewStyle } from '../lib/view';
import {
  closeActiveTab,
  cycleTab,
  goBack,
  openDialog,
  openFind,
  reopenClosedTab,
  setFocusMode,
  setView,
  splitView,
  stopReading,
  toggleFocusMode,
  toggleReading,
  unsplit,
  useApp,
} from '../store';
import { CenterHead } from './CenterHead';
import { Panes } from './Panes';
import { openSymbols } from './Symbols';
import { RightPanel } from './RightPanel';
import { Sidebar } from './Sidebar';
import { StatusBar } from './StatusBar';

/** Ctrl+F, Ctrl+Shift+F, Ctrl+H, Ctrl+Shift+H: the find panel, with the selected text. */
function find(scope: 'doc' | 'all' | undefined, focus: 'find' | 'replace') {
  const editor = useApp.getState().editor;
  const selected =
    editor && !editor.state.selection.empty
      ? editor.state.doc.textBetween(editor.state.selection.from, editor.state.selection.to, ' ').slice(0, 100)
      : undefined;
  openFind({ text: selected, scope, focus });
}

/** What the window does for each shortcut it handles (the editor handles the rest itself). */
const WINDOW_KEYS: Partial<Record<ShortcutId, () => void>> = {
  // Writers press Ctrl+S out of habit: save right away instead of waiting.
  save: () => void flushAll(),
  find: () => find('doc', 'find'),
  findAll: () => find('all', 'find'),
  replace: () => find(undefined, 'replace'),
  replaceAll: () => find('all', 'replace'),
  closeTab: () => closeActiveTab(),
  reopenTab: () => void reopenClosedTab(),
  nextTab: () => cycleTab(1),
  prevTab: () => cycleTab(-1),
  split: () => {
    if (useApp.getState().panes.length > 1) void unsplit();
    else splitView('row');
  },
  showMarks: () => setView({ showMarks: !useApp.getState().view.showMarks }),
  readAloud: () => toggleReading(),
  format: () => openDialog({ kind: 'project', tab: 'format' }),
  symbols: () => openSymbols(),
  back: () => void goBack(-1),
  forward: () => void goBack(1),
  focusMode: () => toggleFocusMode(),
  shortcuts: () => openDialog({ kind: 'shortcuts' }),
};
const WINDOW_IDS = Object.keys(WINDOW_KEYS) as ShortcutId[];

export function Workspace() {
  const rightOpen = useApp((s) => s.rightOpen);
  const focusMode = useApp((s) => s.focusMode);
  const view = useApp((s) => s.view);
  const showMarks = useApp((s) => s.view.showMarks);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const id = shortcutOf(e, WINDOW_IDS);
      if (id) {
        e.preventDefault();
        WINDOW_KEYS[id]?.();
        return;
      }
      // Esc stops reading aloud, or else leaves 집중 모드; never the Esc that
      // cancels a 한글 syllable being composed. Open dialogs take their own Esc.
      if (matches(e, 'escape')) {
        const { reading, dialog, focusMode: inFocus, editor } = useApp.getState();
        if (dialog || keyInComposition(e, editor?.view)) return;
        if (reading && reading.status !== 'noVoice') stopReading();
        else if (inFocus) setFocusMode(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      setFocusMode(false);
    };
  }, []);

  return (
    <div className={`workspace${rightOpen ? '' : ' no-right'}${focusMode ? ' focus-mode' : ''}`}>
      {/* 집중 모드 only hides the columns (styles/focus.css), so they come back as they were. */}
      <Sidebar />
      <main className={`center${showMarks ? ' show-marks' : ''}`} style={viewStyle(view) as CSSProperties}>
        <CenterHead />
        <Panes />
        <StatusBar />
      </main>
      {rightOpen && <RightPanel />}
      {focusMode && (
        <button
          type="button"
          className="focus-exit"
          title={`집중 모드 나가기 (Esc 또는 ${keysText('focusMode')})`}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => setFocusMode(false)}
        >
          집중 모드 나가기
        </button>
      )}
    </div>
  );
}
