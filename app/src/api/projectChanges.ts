// Successful filesystem mutations also dirty the linked .wpm package.
const listeners = new Set<(root: string) => void>();

export function onProjectMutation(listener: (root: string) => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

export function projectMutated(root: string) {
  for (const listener of listeners) listener(root);
}

export const PROJECT_MUTATIONS = new Set([
  'project_update', 'part_add', 'part_rename', 'part_remove',
  'doc_add', 'doc_move', 'doc_trash', 'doc_save', 'doc_keep', 'doc_update_meta', 'doc_mend',
  'snapshot_create', 'snapshot_restore', 'records_tidy', 'trash_restore', 'trash_delete',
  'exchange_send', 'exchange_read', 'exchange_apply', 'import_commit', 'replace_all',
  'card_create', 'card_save', 'card_trash', 'card_type_add', 'card_type_update', 'card_type_remove',
  'note_create', 'note_save', 'note_trash', 'copy_resolve', 'project_sync',
  'journal_event', 'journal_flush', 'journal_anchor',
  'interpret_save',
]);
