// Another device's edits (crates/core/src/copies.rs): the banners over a
// document being changed on two devices or with copies left by a sync
// program, the marks in the tree, and the list of every copy (다른 기기 사본).

import { useMemo, type MouseEvent } from 'react';
import type { CopyAction, CopyInfo, Section } from '../api/types';
import { Modal } from '../components/Modal';
import { openMenu, type MenuItem } from '../components/Menu';
import { num, timeLabel } from '../lib/format';
import { UNTITLED, docNoun, withSubject } from '../lib/labels';
import { closeDialog, findDoc, keepMine, openDialog, openCard, resolveCopy, selectDoc, takeTheirs, useApp } from '../store';

/** How many copies other devices left of each document or card. */
export function countCopies(copies: CopyInfo[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const c of copies) counts.set(c.of, (counts.get(c.of) ?? 0) + 1);
  return counts;
}

/** Small mark on a document or card with copies from other devices. */
export function CopyBadge({ count }: { count: number }) {
  if (!count) return null;
  return (
    <span className="copy-badge" title={`다른 기기에서 생긴 사본 ${count}개`}>
      사본{count > 1 ? ` ${count}` : ''}
    </span>
  );
}

/** "DESKTOP-1AB2C3D에서 · 오늘 14:20" */
export function copyWhere(copy: CopyInfo): string {
  const from = copy.device ? `${copy.device}에서` : '다른 기기에서';
  return copy.modified ? `${from} · ${timeLabel(copy.modified)}` : from;
}

const isDoc = (section: Section) => section === 'manuscript' || section === 'planning';

/** What can be done with a copy, for menus (둘 다 보기 is a button of its own). */
export function copyActions(copy: CopyInfo, noun: string): MenuItem[] {
  const run = (action: CopyAction) => () => void resolveCopy(copy, action);
  const items: MenuItem[] = [{ label: '사본으로 바꾸기', onSelect: run('take') }];
  if (copy.section !== 'notes') {
    items.push({
      label: copy.section === 'cards' ? '둘 다 두기 (사본을 새 카드로)' : `둘 다 두기 (사본을 새 ${noun}로)`,
      onSelect: run('keepBoth'),
    });
  }
  items.push({ separator: true }, { label: '사본 버리기 (휴지통으로)', danger: true, onSelect: run('discard') });
  return items;
}

/**
 * Over a document: another device changed it while it was being changed here
 * (the editor stops until the writer picks), or copies of it are waiting.
 */
export function DocBanners({ docId }: { docId: string }) {
  const conflict = useApp((s) => s.conflicts[docId]);
  const allCopies = useApp((s) => s.overview!.copies);
  const kind = useApp((s) => s.overview!.project.kind);
  const planning = useApp((s) => findDoc(s.overview!, docId)?.section === 'planning');
  const copies = useMemo(() => allCopies.filter((c) => c.of === docId && isDoc(c.section)), [allCopies, docId]);
  const noun = planning ? '문서' : docNoun(kind);

  if (!conflict && !copies.length) return null;
  return (
    <div className="doc-banners">
      {conflict && (
        <div className="doc-banner warn" role="alert">
          <div className="doc-banner-text">
            <strong>다른 기기에서 이 {withSubject(noun)} 고쳐졌음</strong>
            <span>이 기기에서 고친 내용과 달라 저장을 멈췄습니다. 이 기기에서 쓴 글은 기록에 남아 있습니다.</span>
          </div>
          <div className="doc-banner-actions">
            <button type="button" className="btn small primary" onClick={() => openDialog({ kind: 'compare', docId })}>
              둘 다 보기
            </button>
            <button type="button" className="btn small" onClick={() => void takeTheirs(docId)}>
              다른 기기 것 불러오기
            </button>
            <button type="button" className="btn small" onClick={() => void keepMine(docId)}>
              이 기기 것으로 저장
            </button>
          </div>
        </div>
      )}
      {copies.map((copy) => (
        <div key={copy.file} className="doc-banner">
          <div className="doc-banner-text">
            <strong>다른 기기에서 저장한 사본이 있음</strong>
            <span>
              {copyWhere(copy)} · {num(copy.chars)}자
            </span>
          </div>
          <div className="doc-banner-actions">
            <button type="button" className="btn small primary" onClick={() => openDialog({ kind: 'compare', docId, copy })}>
              둘 다 보기
            </button>
            <button
              type="button"
              className="btn small"
              onClick={(e: MouseEvent) => openMenu(e, copyActions(copy, noun), { title: '사본 정리' })}
            >
              정리하기
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

const SECTION_LABEL: Record<Section, string> = {
  manuscript: '원고',
  planning: '기획',
  cards: '설정 카드',
  notes: '메모',
};

/** Every copy left by sync programs, with what can be done with each. */
export function CopiesDialog() {
  const ov = useApp((s) => s.overview)!;
  const notes = useApp((s) => s.notes);
  const noun = docNoun(ov.project.kind);

  const nameOf = (copy: CopyInfo): string => {
    if (isDoc(copy.section)) return findDoc(ov, copy.of)?.doc.title || UNTITLED;
    if (copy.section === 'cards') return ov.cards.find((c) => c.id === copy.of)?.name ?? copy.title;
    const note = notes.find((n) => n.id === copy.of);
    return note?.text.split('\n')[0] || note?.quote || copy.title || '메모';
  };

  const open = (copy: CopyInfo) => {
    closeDialog();
    if (isDoc(copy.section)) void selectDoc(copy.of);
    else if (copy.section === 'cards') void openCard(copy.of);
  };

  return (
    <Modal title="다른 기기 사본" onClose={closeDialog} width={620}>
      <p className="hint">
        두 기기에서 같은 글을 고친 뒤 OneDrive 같은 프로그램이 맞추지 못하면 사본이 생깁니다. 둘을 비교해 하나를
        고르거나, 둘 다 둘 수 있습니다.
      </p>
      {ov.copies.length === 0 && <p className="empty-note">정리할 사본이 없습니다.</p>}
      <ul className="copy-list">
        {ov.copies.map((copy) => (
          <li key={`${copy.section}/${copy.file}`} className="copy-item">
            <div className="grow">
              <button type="button" className="link-btn" onClick={() => open(copy)}>
                <strong>{nameOf(copy)}</strong>
              </button>
              <span className="meta">
                {SECTION_LABEL[copy.section]} · {copyWhere(copy)} · {num(copy.chars)}자
              </span>
            </div>
            {isDoc(copy.section) && (
              <button type="button" className="btn small" onClick={() => openDialog({ kind: 'compare', docId: copy.of, copy })}>
                둘 다 보기
              </button>
            )}
            <button
              type="button"
              className="btn small"
              onClick={(e: MouseEvent) => openMenu(e, copyActions(copy, noun), { title: nameOf(copy) })}
            >
              정리하기
            </button>
          </li>
        ))}
      </ul>
    </Modal>
  );
}
