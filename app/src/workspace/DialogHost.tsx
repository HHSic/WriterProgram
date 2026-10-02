// Shows the one open dialog (store `dialog`).

import { NewProjectDialog } from '../screens/NewProjectDialog';
import { AiDialog } from './AiDialog';
import { CompareDialog } from './Compare';
import { ConfirmDialog } from './ConfirmDialog';
import { CopiesDialog } from './Copies';
import { DriveImportDialog, DrivesDialog } from './Drives';
import { ExportDialog } from './ExportDialog';
import { ImportDialog } from './ImportDialog';
import { MoveDialog } from './MoveProject';
import { PromptDialog } from './PromptDialog';
import { ProjectSettingsDialog } from './settings/ProjectSettingsDialog';
import { SymbolsDialog } from './Symbols';
import { TrashDialog } from './TrashDialog';
import { ViewDialog } from './ViewDialog';
import { useApp } from '../store';

export function DialogHost() {
  const dialog = useApp((s) => s.dialog);
  if (!dialog) return null;
  switch (dialog.kind) {
    case 'prompt':
      return <PromptDialog key={dialog.title} dialog={dialog} />;
    case 'confirm':
      return <ConfirmDialog dialog={dialog} />;
    case 'trash':
      return <TrashDialog />;
    case 'export':
      return <ExportDialog />;
    case 'import':
      return <ImportDialog partId={dialog.partId} />;
    case 'view':
      return <ViewDialog />;
    case 'symbols':
      return <SymbolsDialog />;
    case 'newProject':
      return <NewProjectDialog />;
    case 'project':
      return <ProjectSettingsDialog tab={dialog.tab} />;
    case 'compare':
      return <CompareDialog key={`${dialog.docId}/${dialog.copy?.file ?? ''}`} docId={dialog.docId} copy={dialog.copy} />;
    case 'copies':
      return <CopiesDialog />;
    case 'move':
      return <MoveDialog />;
    case 'drives':
      return <DrivesDialog />;
    case 'driveImport':
      return <DriveImportDialog />;
    case 'ai':
      return <AiDialog />;
  }
}
