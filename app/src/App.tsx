import { useEffect } from 'react';
import { api } from './api';
import { MenuHost } from './components/Menu';
import { CornerNote } from './components/CornerNote';
import { ToastHost } from './components/ToastHost';
import { installJournal } from './editor/journal';
import { applyColors } from './lib/colors';
import { StartScreen } from './screens/StartScreen';
import { checkForUpdateDaily, confirmClose, stampOnClosing, useApp } from './store';
import { CloseAskDialog } from './workspace/CloseAskDialog';
import { DialogHost } from './workspace/dialogs';
import { Workspace } from './workspace/Workspace';

export function App() {
  const hasProject = useApp((s) => s.overview !== null);
  const theme = useApp((s) => s.view.theme);
  const palette = useApp((s) => s.view.palette);
  const accent = useApp((s) => s.view.accent);
  const customColor = useApp((s) => s.view.customColor);
  const busy = useApp((s) => s.busy);

  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') delete root.dataset.theme;
    else root.dataset.theme = theme;
    // 화면 색 needs to know light or dark; with 시스템 따라 it follows changes.
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () =>
      applyColors(root, { palette, accent, customColor }, theme === 'dark' || (theme === 'system' && media.matches));
    apply();
    if (theme !== 'system') return;
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [theme, palette, accent, customColor]);

  useEffect(() => {
    // On phones the keyboard covers the bottom of the page without resizing
    // it; --kb is how much it covers, for the 편집 도구줄.
    const vv = window.visualViewport;
    if (!vv) return;
    const update = () => {
      const covered = Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
      document.documentElement.style.setProperty('--kb', `${Math.round(covered)}px`);
    };
    update();
    vv.addEventListener('resize', update);
    vv.addEventListener('scroll', update);
    return () => {
      vv.removeEventListener('resize', update);
      vv.removeEventListener('scroll', update);
    };
  }, []);

  useEffect(() => {
    // Finish the last save before the window closes; when something could
    // not be saved, the writer picks: try again, keep it in the rescue
    // folder and close, or stay. Then the creation journal writes the saves
    // it was still gathering.
    api.onCloseRequested(async () => {
      if (!(await confirmClose())) return false;
      await api.journalFlush(null).catch(() => {});
      // The day's last writing gets its date proof before the app goes.
      const root = useApp.getState().overview?.root;
      if (root) await stampOnClosing(root);
      return true;
    });
    installJournal();
    // A new version, looked for once a day a little after start.
    const timer = window.setTimeout(() => void checkForUpdateDaily(), 8000);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <>
      {hasProject ? <Workspace /> : <StartScreen />}
      <DialogHost />
      <CloseAskDialog />
      <MenuHost />
      <ToastHost />
      <CornerNote />
      {busy && (
        <div className="busy-cover" role="status" aria-live="polite">
          <div className="busy-box">{busy}</div>
        </div>
      )}
    </>
  );
}
