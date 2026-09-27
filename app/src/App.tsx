import { useEffect } from 'react';
import { api } from './api';
import { MenuHost } from './components/Menu';
import { ToastHost } from './components/ToastHost';
import { StartScreen } from './screens/StartScreen';
import { saveEverything, useApp } from './store';
import { DialogHost } from './workspace/dialogs';
import { Workspace } from './workspace/Workspace';

export function App() {
  const hasProject = useApp((s) => s.overview !== null);
  const theme = useApp((s) => s.view.theme);

  useEffect(() => {
    if (theme === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
  }, [theme]);

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
