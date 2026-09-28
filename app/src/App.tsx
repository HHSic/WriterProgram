import { useEffect } from 'react';
import { api } from './api';
import { MenuHost } from './components/Menu';
import { ToastHost } from './components/ToastHost';
import { applyColors } from './lib/colors';
import { StartScreen } from './screens/StartScreen';
import { saveEverything, useApp } from './store';
import { DialogHost } from './workspace/dialogs';
import { Workspace } from './workspace/Workspace';

export function App() {
  const hasProject = useApp((s) => s.overview !== null);
  const theme = useApp((s) => s.view.theme);
  const palette = useApp((s) => s.view.palette);
  const accent = useApp((s) => s.view.accent);
  const customColor = useApp((s) => s.view.customColor);

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
    // Finish the last save before the window closes; keep it open if saving fails.
    api.onCloseRequested(saveEverything);
  }, []);

  return (
    <>
      {hasProject ? <Workspace /> : <StartScreen />}
      <DialogHost />
      <MenuHost />
      <ToastHost />
    </>
  );
}
