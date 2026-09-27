import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { Backend } from './types';

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
  docSave: (root, docId, body) => invoke('doc_save', { root, docId, body }),
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
  reveal: (path) => invoke('reveal', { path }),
  pickFolder: async (title, defaultPath) => {
    const picked = await open({ directory: true, multiple: false, title, defaultPath });
    return typeof picked === 'string' ? picked : null;
  },
  pickSaveFile: (title, defaultName) =>
    save({ title, defaultPath: defaultName, filters: [{ name: '텍스트 파일', extensions: ['txt'] }] }),
  onCloseRequested: (handler) => {
    void getCurrentWindow().onCloseRequested(async (event) => {
      if (!(await handler())) event.preventDefault();
    });
  },
  setWindowTitle: (title) => {
    void getCurrentWindow().setTitle(title);
  },
};
