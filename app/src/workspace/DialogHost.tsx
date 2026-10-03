// Shows the one open dialog (store `dialog`).

import { NewProjectDialog } from '../screens/NewProjectDialog';
import { RecoverDialog } from '../screens/RecoverDialog';
import { AiDialog } from './AiDialog';
import { CompareDialog } from './Compare';
import { ConfirmDialog } from './ConfirmDialog';
import { CopiesDialog } from './Copies';
import { DriveImportDialog, DrivesDialog } from './Drives';
import { ExchangesDialog } from './Exchanges';
import { ExportDialog } from './ExportDialog';
import { ImportDialog } from './ImportDialog';
import { MendDialog } from './MendDialog';
import { MoveDialog } from './MoveProject';
import { PromptDialog } from './PromptDialog';
import { ProofDialog } from './ProofDialog';
import { RemovalsDialog } from './RemovalsDialog';
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
      return <ExportDialog key={dialog.toEditor ? 'editor' : 'export'} toEditor={dialog.toEditor} docIds={dialog.docIds} />;
    case 'exchanges':
      return <ExchangesDialog />;
    case 'import':
      return <ImportDialog partId={dialog.partId} newProject={dialog.newProject} />;
    case 'view':
      return <ViewDialog />;
    case 'symbols':
      return <SymbolsDialog />;
    case 'newProject':
      return <NewProjectDialog />;
    case 'project':
      return <ProjectSettingsDialog tab={dialog.tab} />;
    case 'compare':
      return (
        <CompareDialog
          key={`${dialog.docId}/${dialog.copy?.file ?? dialog.rescue?.path ?? ''}`}
          docId={dialog.docId}
          copy={dialog.copy}
          rescue={dialog.rescue}
        />
      );
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
    case 'proof':
      return <ProofDialog />;
    case 'removals':
      return <RemovalsDialog />;
    case 'mend':
      return <MendDialog key={dialog.docId} docId={dialog.docId} />;
    case 'recover':
      return <RecoverDialog path={dialog.path} recovery={dialog.recovery} />;
  }
}
