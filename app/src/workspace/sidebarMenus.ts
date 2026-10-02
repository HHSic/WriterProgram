// The menus of the left column's rows: a chapter or planning document, and a part (부).

import type { DocSummary, Overview, PartView } from '../api/types';
import type { MenuItem } from '../components/Menu';
import { STATUS_LABEL, docNoun, statusesFor } from '../lib/labels';
import {
  addDoc,
  addNote,
  moveDoc,
  openDialog,
  openTable,
  removePart,
  renameDoc,
  renamePart,
  selectDoc,
  setStatus,
  setTarget,
  trashDoc,
} from '../store';

/** What the menus need from the sidebar. */
export interface SidebarMenuContext {
  ov: Overview;
  /** Opens a folded part, so a document moved or added there shows. */
  expand: (partId: string) => void;
}

export function moveItems({ ov, expand }: SidebarMenuContext, doc: DocSummary, part: PartView | null): MenuItem[] {
  const list = part ? part.docs : ov.planning;
  const i = list.findIndex((d) => d.id === doc.id);
  const items: MenuItem[] = [
    { label: '위로 옮기기', disabled: i <= 0, onSelect: () => void moveDoc(doc.id, part?.id ?? null, i - 1) },
    {
      label: '아래로 옮기기',
      disabled: i >= list.length - 1,
      onSelect: () => void moveDoc(doc.id, part?.id ?? null, i + 1),
    },
  ];
  if (part && ov.parts.length > 1) {
    items.push({ heading: '다른 부로 옮기기' });
    for (const other of ov.parts) {
      if (other.id === part.id) continue;
      items.push({
        label: other.title,
        onSelect: () => {
          expand(other.id);
          void moveDoc(doc.id, other.id, other.docs.length);
        },
      });
    }
  }
  return items;
}

export function docMenu(ctx: SidebarMenuContext, doc: DocSummary, part: PartView | null): MenuItem[] {
  const kind = ctx.ov.project.kind;
  const noun = docNoun(kind);
  const planning = part === null;
  const items: MenuItem[] = [
    { label: '열기', onSelect: () => void selectDoc(doc.id) },
    { label: '새 탭에서 열기', onSelect: () => void selectDoc(doc.id, true) },
    {
      label: '이름 바꾸기',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: '이름 바꾸기',
          label: '제목',
          value: doc.title,
          confirm: '바꾸기',
          onSubmit: (title) => renameDoc(doc.id, title),
        }),
    },
    {
      label: planning ? '아래에 새 기획 문서' : `아래에 새 ${noun}`,
      onSelect: () => void addDoc({ section: planning ? 'planning' : 'manuscript', after: doc.id }),
    },
    {
      label: planning ? '이 문서에 메모' : `이 ${noun}에 메모`,
      onSelect: () => {
        void (async () => {
          await selectDoc(doc.id);
          await addNote({ anchor: 'doc', target: doc.id });
        })();
      },
    },
  ];
  if (!planning) {
    items.push({
      label: '목표 분량 바꾸기',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: '목표 분량',
          label: `이 ${noun}의 목표 글자 수 (비우면 작품 기본값)`,
          value: doc.target ? String(doc.target) : '',
          confirm: '바꾸기',
          inputMode: 'numeric',
          onSubmit: (value) => {
            const n = Number.parseInt(value.replace(/[^0-9]/g, ''), 10);
            return setTarget(doc.id, Number.isFinite(n) && n > 0 ? n : null);
          },
        }),
    });
  }
  items.push({ separator: true }, ...moveItems(ctx, doc, part));
  if (!planning) {
    items.push(
      { separator: true },
      { heading: '상태' },
      ...statusesFor(kind).map(
        (status): MenuItem => ({
          label: STATUS_LABEL[status],
          checked: doc.status === status,
          onSelect: () => void setStatus(doc.id, status),
        }),
      ),
    );
  }
  items.push({ separator: true }, { label: '휴지통으로', danger: true, onSelect: () => void trashDoc(doc.id) });
  return items;
}

export function partMenu({ ov, expand }: SidebarMenuContext, part: PartView): MenuItem[] {
  const noun = docNoun(ov.project.kind);
  return [
    {
      label: `이 부에 새 ${noun}`,
      onSelect: () => {
        expand(part.id);
        void addDoc({ partId: part.id });
      },
    },
    {
      label: `이 부로 ${noun} 가져오기…`,
      onSelect: () => {
        expand(part.id);
        openDialog({ kind: 'import', partId: part.id });
      },
    },
    { label: '개요 표로 보기', onSelect: () => void openTable(part.id) },
    {
      label: '이름 바꾸기',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: '부 이름 바꾸기',
          label: '이름',
          value: part.title,
          confirm: '바꾸기',
          onSubmit: (title) => renamePart(part.id, title),
        }),
    },
    { separator: true },
    {
      label: '부 지우기',
      danger: true,
      disabled: part.docs.length > 0 || ov.parts.length === 1,
      onSelect: () => void removePart(part.id),
    },
  ];
}
