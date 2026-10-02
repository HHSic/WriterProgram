import { useEffect, type CSSProperties } from 'react';
import { flushAll } from '../lib/flush';
import { viewStyle } from '../lib/view';
import {
  closeActiveTab,
  cycleTab,
  goBack,
  openDialog,
  openFind,
  reopenClosedTab,
  setView,
  splitView,
  stopReading,
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

export function Workspace() {
  const rightOpen = useApp((s) => s.rightOpen);
  const view = useApp((s) => s.view);
  const showMarks = useApp((s) => s.view.showMarks);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Writers press Ctrl+S out of habit: save right away instead of waiting.
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      if (mod && key === 's' && !e.shiftKey) {
        e.preventDefault();
        void flushAll();
      }
      // Ctrl+F: find in this document; Ctrl+Shift+F: the whole project; Ctrl+H: replace.
      if (mod && (key === 'f' || key === 'h')) {
        e.preventDefault();
        const editor = useApp.getState().editor;
        const selected =
          editor && !editor.state.selection.empty
            ? editor.state.doc.textBetween(editor.state.selection.from, editor.state.selection.to, ' ').slice(0, 100)
            : undefined;
        openFind({
          text: selected,
          scope: e.shiftKey ? 'all' : key === 'f' ? 'doc' : undefined,
          focus: key === 'h' ? 'replace' : 'find',
        });
      }
      // Tabs: Ctrl+W closes, Ctrl+Shift+T opens the last closed again, Ctrl+Tab moves.
      if (mod && key === 'w' && !e.shiftKey) {
        e.preventDefault();
        closeActiveTab();
      }
      if (mod && key === 't' && e.shiftKey) {
        e.preventDefault();
        void reopenClosedTab();
      }
      if (e.ctrlKey && e.key === 'Tab') {
        e.preventDefault();
        cycleTab(e.shiftKey ? -1 : 1);
      }
      // Ctrl+\: split the middle in two, or back to one.
      if (mod && e.key === '\\') {
        e.preventDefault();
        if (useApp.getState().panes.length > 1) void unsplit();
        else splitView('row');
      }
      // Ctrl+Shift+8: 빈칸·문단 부호 보이기 (as in Word).
      if (mod && e.shiftKey && e.code === 'Digit8') {
        e.preventDefault();
        setView({ showMarks: !useApp.getState().view.showMarks });
      }
      // Ctrl+Shift+R: 소리 내어 읽기, or stop it. Esc stops it too.
      if (mod && e.shiftKey && !e.altKey && e.code === 'KeyR') {
        e.preventDefault();
        toggleReading();
      }
      if (e.key === 'Escape' && !e.isComposing) {
        const { reading, dialog } = useApp.getState();
        if (reading && reading.status !== 'noVoice' && !dialog) stopReading();
      }
      // F7: 원고 서식 (편집 용지 in 한글). Ctrl+F10: 문자표.
      if (e.key === 'F7' && !mod && !e.altKey) {
        e.preventDefault();
        openDialog({ kind: 'project', tab: 'format' });
      }
      if (mod && e.key === 'F10') {
        e.preventDefault();
        openSymbols();
      }
      // Alt+← / Alt+→: back and forward in the tab.
      if (e.altKey && !mod && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault();
        void goBack(e.key === 'ArrowLeft' ? -1 : 1);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return (
    <div className={`workspace${rightOpen ? '' : ' no-right'}`}>
      <Sidebar />
      <main className={`center${showMarks ? ' show-marks' : ''}`} style={viewStyle(view) as CSSProperties}>
        <CenterHead />
        <Panes />
        <StatusBar />
      </main>
      {rightOpen && <RightPanel />}
    </div>
  );
}
