import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { Backend, Change, PageEvent } from './types';

const FILE_TYPE_NAMES: Record<string, string> = {
  txt: '텍스트 파일',
  docx: 'Word 문서',
  hwpx: '한글 문서',
};

export const tauriBackend: Backend = {
  isDesktop: true,
  recentList: () => invoke('recent_list'),
  recentRemove: (path) => invoke('recent_remove', { path }),
  defaultLocation: () => invoke('default_location'),
  projectCreate: (opts) => invoke('project_create', { opts }),
  projectOpen: (path) => invoke('project_open', { path }),
  projectOverview: (root) => invoke('project_overview', { root }),
  projectUpdate: (root, patch) => invoke('project_update', { root, patch }),
  partAdd: (root, title) => invoke('part_add', { root, title }),
  partRename: (root, partId, title) => invoke('part_rename', { root, partId, title }),
  partRemove: (root, partId) => invoke('part_remove', { root, partId }),
  docAdd: (root, spec) => invoke('doc_add', { root, spec }),
  docMove: (root, docId, partId, index) => invoke('doc_move', { root, docId, partId, index }),
  docTrash: (root, docId) => invoke('doc_trash', { root, docId }),
  docLoad: (root, docId) => invoke('doc_load', { root, docId }),
  docSave: (root, docId, body, base, force) => invoke('doc_save', { root, docId, body, base: base ?? null, force: force ?? false }),
  docKeep: (root, docId, body, kind) => invoke('doc_keep', { root, docId, body, kind }),
  docUpdateMeta: (root, docId, patch) => invoke('doc_update_meta', { root, docId, patch }),
  snapshotList: (root, docId) => invoke('snapshot_list', { root, docId }),
  snapshotCreate: (root, docId, name) => invoke('snapshot_create', { root, docId, name }),
  snapshotLoad: (root, docId, snapshotId) => invoke('snapshot_load', { root, docId, snapshotId }),
  snapshotRestore: (root, docId, snapshotId) => invoke('snapshot_restore', { root, docId, snapshotId }),
  trashList: (root) => invoke('trash_list', { root }),
  trashRestore: (root, trashId) => invoke('trash_restore', { root, trashId }),
  trashDelete: (root, trashId) => invoke('trash_delete', { root, trashId }),
  exportText: (root, items, opts) => invoke('export_text', { root, items, opts }),
  exportTxt: (root, items, opts, dest, perDoc) => invoke('export_txt', { root, items, opts, dest, perDoc }),
  exportFile: (root, items, opts, format, kind, dest, perDoc) =>
    invoke('export_file', { root, items, opts, format, kind, dest, perDoc }),
  importPreview: (paths, opts) => invoke('import_preview', { paths, opts }),
  importCommit: (root, paths, opts, spec) => invoke('import_commit', { root, paths, opts, spec }),
  search: (root, query) => invoke('search', { root, query }),
  replaceAll: (root, query, replacement) => invoke('replace_all', { root, query, replacement }),
  cardLoad: (root, cardId) => invoke('card_load', { root, cardId }),
  cardCreate: (root, typeId, name) => invoke('card_create', { root, typeId, name }),
  cardSave: (root, card) => invoke('card_save', { root, card }),
  cardTrash: (root, cardId) => invoke('card_trash', { root, cardId }),
  cardAppearances: (root, cardId) => invoke('card_appearances', { root, cardId }),
  cardCounts: (root) => invoke('card_counts', { root }),
  cardTypeAdd: (root, name) => invoke('card_type_add', { root, name }),
  cardTypeUpdate: (root, kind) => invoke('card_type_update', { root, kind }),
  cardTypeRemove: (root, typeId) => invoke('card_type_remove', { root, typeId }),
  noteList: (root) => invoke('note_list', { root }),
  noteCreate: (root, spec) => invoke('note_create', { root, spec }),
  noteSave: (root, note) => invoke('note_save', { root, note }),
  noteTrash: (root, noteId) => invoke('note_trash', { root, noteId }),
  formatCatalog: () => invoke('format_catalog'),
  formatSavePreset: (name, format) => invoke('format_save_preset', { name, format }),
  formatDeletePreset: (name) => invoke('format_delete_preset', { name }),
  formatEstimate: (root, format) => invoke('format_estimate', { root, format }),
  copyLoad: (root, section, file) => invoke('copy_load', { root, section, file }),
  copyResolve: (root, section, file, action) => invoke('copy_resolve', { root, section, file, action }),
  storagePlaces: () => invoke('storage_places'),
  storageOf: (path) => invoke('storage_of', { path }),
  projectMove: (root, dest) => invoke('project_move', { root, dest }),
  watchProject: (root, onChange) => {
    let stopped = false;
    const unlisten = listen<{ root: string; changes: Change[] }>('project-changed', (event) => {
      if (!stopped && event.payload.root === root) onChange(event.payload.changes);
    });
    void invoke('project_watch', { root }).catch(() => {
      // Without a watcher, changes from other devices show when the project opens again.
    });
    return () => {
      stopped = true;
      void unlisten.then((stop) => stop());
      void invoke('project_unwatch', { root }).catch(() => {});
    };
  },
  driveStatus: () => invoke('drive_status'),
  journalSettings: () => invoke('journal_settings'),
  journalSet: (patch) => invoke('journal_set', { enabled: patch.enabled ?? null, noticed: patch.noticed ?? null }),
  journalEvent: (root, event) => invoke('journal_event', { root, event }),
  journalSummary: (root) => invoke('journal_summary', { root }),
  journalVerify: (root) => invoke('journal_verify', { root }),
  driveConnect: (provider) => invoke('drive_connect', { provider }),
  driveCancel: () => invoke('drive_cancel'),
  driveDisconnect: (provider) => invoke('drive_disconnect', { provider }),
  driveProjects: (provider) => invoke('drive_projects', { provider }),
  driveFetch: (provider, folder, dest) => invoke('drive_fetch', { provider, folder, dest }),
  projectLinkGet: (projectId) => invoke('project_link_get', { projectId }),
  projectLink: (projectId, title, provider) => invoke('project_link', { projectId, title, provider }),
  projectUnlink: (projectId) => invoke('project_unlink', { projectId }),
  projectSync: (root, projectId) => invoke('project_sync', { root, projectId }),
  browserOpen: (label, url, at) => invoke('browser_open', { label, url, ...at }),
  browserBounds: (label, at, visible) => invoke('browser_bounds', { label, ...at, visible }),
  browserNavigate: (label, url) => invoke('browser_navigate', { label, url }),
  browserStep: (label, step) => invoke('browser_step', { label, step }),
  browserClose: (label) => invoke('browser_close', { label }),
  browserClip: (label) => invoke('browser_clip', { label }),
  onBrowserPage: (handler) => {
    const unlisten = listen<PageEvent>('browser-page', (event) => handler(event.payload));
    return () => void unlisten.then((stop) => stop());
  },
  reveal: (path) => invoke('reveal', { path }),
  pickFolder: async (title, defaultPath) => {
    const picked = await open({ directory: true, multiple: false, title, defaultPath });
    return typeof picked === 'string' ? picked : null;
  },
  pickFiles: async (title) => {
    const picked = await open({
      multiple: true,
      directory: false,
      title,
      filters: [{ name: '원고 파일 (txt, md, docx, hwpx)', extensions: ['txt', 'md', 'markdown', 'docx', 'hwpx', 'hwp'] }],
    });
    return Array.isArray(picked) ? picked : typeof picked === 'string' ? [picked] : [];
  },
  pickSaveFile: (title, defaultName, extension) =>
    save({ title, defaultPath: defaultName, filters: [{ name: FILE_TYPE_NAMES[extension] ?? extension, extensions: [extension] }] }),
  onCloseRequested: (handler) => {
    void getCurrentWindow().onCloseRequested(async (event) => {
      if (!(await handler())) event.preventDefault();
    });
  },
  setWindowTitle: (title) => {
    void getCurrentWindow().setTitle(title);
  },
};
