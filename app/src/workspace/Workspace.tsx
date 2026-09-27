import { useEffect, type CSSProperties } from 'react';
import { Icon } from '../components/Icon';
import { flushAll } from '../lib/flush';
import { docNoun, withObject } from '../lib/labels';
import { viewStyle } from '../lib/view';
import { addDoc, useApp } from '../store';
import { CenterHead } from './CenterHead';
import { DocPane } from './DocPane';
import { RightPanel } from './RightPanel';
import { Sidebar } from './Sidebar';
import { StatusBar } from './StatusBar';

export function Workspace() {
  const activeDocId = useApp((s) => s.activeDocId);
  const rightOpen = useApp((s) => s.rightOpen);
  const view = useApp((s) => s.view);
  const kind = useApp((s) => s.overview!.project.kind);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Writers press Ctrl+S out of habit: save right away instead of waiting.
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's' && !e.shiftKey) {
        e.preventDefault();
        void flushAll();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return (
    <div className={`workspace${rightOpen ? '' : ' no-right'}`}>
      <Sidebar />
      <main className="center" style={viewStyle(view) as CSSProperties}>
        <CenterHead />
        {activeDocId ? (
          <DocPane docId={activeDocId} />
        ) : (
          <div className="pane-message">
            <p>왼쪽에서 {withObject(docNoun(kind))} 고르거나 새로 만드세요.</p>
            <button type="button" className="btn" onClick={() => void addDoc({})}>
              <Icon name="plus" size={14} />새 {docNoun(kind)}
            </button>
          </div>
        )}
        <StatusBar />
      </main>
      {rightOpen && <RightPanel />}
    </div>
  );
}
