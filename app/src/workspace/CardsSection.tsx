// 설정집 in the left column: kinds with their cards. One click shows a card
// over the right panel; a double click opens it in the middle.

import { useState } from 'react';
import { api } from '../api';
import type { CardType } from '../api/types';
import { Icon } from '../components/Icon';
import { type MenuItem, type MenuOptions } from '../components/Menu';
import { MoreButton } from '../components/MoreButton';
import { CopyBadge, countCopies } from './Copies';
import { pressMenu } from '../lib/press';
import { docNoun } from '../lib/labels';
import {
  createCard,
  openCard,
  openDialog,
  previewCard,
  refreshOverview,
  toastError,
  trashCard,
  useApp,
} from '../store';

export function CardsSection() {
  const ov = useApp((s) => s.overview)!;
  const activeCardId = useApp((s) => s.activeCardId);
  const previewCardId = useApp((s) => s.previewCardId);
  const counts = useApp((s) => s.cardCounts);
  const copyCounts = countCopies(ov.copies);
  const noun = docNoun(ov.project.kind);
  const [open, setOpen] = useState<Set<string>>(() => new Set(['person']));

  const toggle = (id: string) =>
    setOpen((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const newCard = (kind: CardType) =>
    openDialog({
      kind: 'prompt',
      title: `새 ${kind.name} 카드`,
      label: '이름',
      value: '',
      confirm: '만들기',
      onSubmit: (name) => {
        setOpen((prev) => new Set(prev).add(kind.id));
        return createCard(kind.id, name);
      },
    });

  const addKind = () =>
    openDialog({
      kind: 'prompt',
      title: '분류 추가',
      label: '분류 이름 (예: 세력, 아이템, 스킬)',
      value: '',
      confirm: '추가',
      onSubmit: async (name) => {
        try {
          await api.cardTypeAdd(ov.root, name);
          await refreshOverview();
        } catch (e) {
          toastError('분류를 만들지 못함', e);
        }
      },
    });

  const kindMenu = (kind: CardType): MenuItem[] => [
    { label: `새 ${kind.name} 카드`, onSelect: () => newCard(kind) },
    {
      label: '이름 바꾸기',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: '분류 이름 바꾸기',
          label: '이름',
          value: kind.name,
          confirm: '바꾸기',
          onSubmit: async (name) => {
            try {
              await api.cardTypeUpdate(ov.root, { ...kind, name });
              await refreshOverview();
            } catch (e) {
              toastError('분류 이름을 바꾸지 못함', e);
            }
          },
        }),
    },
    {
      label: '새 카드의 기본 항목',
      onSelect: () =>
        openDialog({
          kind: 'prompt',
          title: `${kind.name} 기본 항목`,
          label: '쉼표로 나눠 적기 (예: 나이, 직업, 말투)',
          value: kind.fields.join(', '),
          confirm: '저장',
          onSubmit: async (value) => {
            try {
              await api.cardTypeUpdate(ov.root, { ...kind, fields: value.split(',').map((f) => f.trim()) });
              await refreshOverview();
            } catch (e) {
              toastError('기본 항목을 바꾸지 못함', e);
            }
          },
        }),
    },
    { separator: true },
    {
      label: '분류 지우기',
      danger: true,
      disabled: ov.cards.some((c) => c.cardType === kind.id),
      onSelect: async () => {
        try {
          await api.cardTypeRemove(ov.root, kind.id);
          await refreshOverview();
        } catch (e) {
          toastError('분류를 지우지 못함', e);
        }
      },
    },
  ];

  return (
    <section aria-label="설정집" className="tree planning">
      <div className="section-head">
        <span className="section-label">설정집</span>
        <button type="button" className="icon-btn tiny" aria-label="분류 추가" title="분류 추가" onClick={addKind}>
          <Icon name="plus" size={13} />
        </button>
      </div>
      {ov.cardTypes.map((kind) => {
        const cards = ov.cards.filter((c) => c.cardType === kind.id);
        const expanded = open.has(kind.id);
        const kindOpts = (): MenuOptions => ({ title: kind.name, subtitle: `카드 ${cards.length}개` });
        return (
          <div key={kind.id} className="part">
            <div className="kind-row">
              <button
                type="button"
                className="part-head grow"
                aria-expanded={expanded}
                onClick={() => toggle(kind.id)}
                {...pressMenu(() => kindMenu(kind), kindOpts)}
              >
                <Icon name={expanded ? 'chevronDown' : 'chevronRight'} size={14} />
                <span className="part-title">{kind.name}</span>
                <span className="count">{cards.length}</span>
              </button>
              <button
                type="button"
                className="icon-btn tiny"
                aria-label={`새 ${kind.name} 카드`}
                title={`새 ${kind.name} 카드`}
                onClick={() => newCard(kind)}
              >
                <Icon name="plus" size={12} />
              </button>
              <MoreButton items={() => kindMenu(kind)} opts={kindOpts} label={`${kind.name} 메뉴`} />
            </div>
            {expanded &&
              cards.map((card) => {
                const about =
                  [
                    card.summary,
                    counts[card.id] === undefined
                      ? ''
                      : counts[card.id]
                        ? `${counts[card.id]}개 ${noun}에 나옴`
                        : '아직 원고에 나오지 않음',
                  ]
                    .filter(Boolean)
                    .join('\n') || undefined;
                const cardMenu = (): MenuItem[] => [
                  { label: '미리보기', onSelect: () => previewCard(card.id) },
                  { label: '편집', onSelect: () => void openCard(card.id) },
                  { separator: true },
                  { label: '휴지통으로', danger: true, onSelect: () => void trashCard(card.id) },
                ];
                const cardOpts = (): MenuOptions => ({ title: card.name, subtitle: about });
                return (
                  <div key={card.id} className="card-row">
                    <button
                      type="button"
                      className={`card-item${card.id === activeCardId || card.id === previewCardId ? ' active' : ''}`}
                      title={about}
                      onClick={() => previewCard(card.id)}
                      onDoubleClick={() => void openCard(card.id)}
                      {...pressMenu(cardMenu, cardOpts)}
                    >
                      <span className="grow ellipsis">{card.name}</span>
                      <CopyBadge count={copyCounts.get(card.id) ?? 0} />
                    </button>
                    <MoreButton items={cardMenu} opts={cardOpts} label={`${card.name} 메뉴`} />
                  </div>
                );
              })}
          </div>
        );
      })}
    </section>
  );
}
