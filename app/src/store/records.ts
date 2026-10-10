import { api } from '../api';
import type { SnapshotInfo } from '../api/types';
import { timeLabel } from '../lib/format';
import { get, set } from './state';
import { findDoc, openDialog, patchSummary, saveEverything, toastError } from './ui';

/** Restore a kept text after saving; the backend retains the current text first. */
export function restoreRecord(docId: string, record: SnapshotInfo) {
  const root = get().overview?.root;
  if (!root) return;
  const canRestore = () => {
    const s=get();
    const d=s.overview?.root===root?findDoc(s.overview,docId):null;
    return !!d&&!s.conflicts[docId]&&!d.doc.locked;
  };
  if (!canRestore()) return;
  openDialog({
    kind: 'confirm', title: '이 때로 되돌리기', confirm: '되돌리기',
    message: `${timeLabel(record.at)}${record.name ? ` · ${record.name}` : ''}의 원고로 되돌립니다. 지금 원고는 '되돌리기 전' 기록으로 남아서 언제든 다시 돌아올 수 있습니다.`,
    onConfirm: async () => {
      if (!canRestore() || !(await saveEverything()) || !canRestore()) return;
      try {
        await api.snapshotRestore(root, docId, record.id);
        const reloaded = await api.docLoad(root, docId);
        if (get().overview?.root !== root) return;
        patchSummary(docId, { counts: reloaded.counts });
        set(s => ({ docVersion: s.docVersion + 1, recordsVersion: s.recordsVersion + 1 }));
      } catch (e) { toastError('되돌리지 못함', e); }
    },
  });
}
