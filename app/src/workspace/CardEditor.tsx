// 설정 카드 편집 (S8), in the middle column.

import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import type { Card } from '../api/types';
import { Icon } from '../components/Icon';
import { openMenu } from '../components/Menu';
import { useAutoHeight } from '../lib/autoHeight';
import { errorText } from '../lib/format';
import { docNoun } from '../lib/labels';
import { useDebouncedSave } from '../lib/useDebouncedSave';
import { patchCardSummary, refreshCardCounts, trashCard, useApp } from '../store';

export function CardEditor({ cardId, locked = false }: { cardId: string; locked?: boolean }) {
  const root = useApp((s) => s.overview!.root);
  const [card, setCard] = useState<Card | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setCard(null);
    api.cardLoad(root, cardId).then(
      (c) => alive && setCard(c),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [root, cardId]);

  if (error) return <div className="pane-message">이 카드를 열 수 없음 · {error}</div>;
  if (!card) return <div className="pane-message" />;
  return <LoadedCard key={cardId} root={root} initial={card} locked={locked} />;
}

/** A card as plain text, for its rescue copy. */
function cardText(card: Card): string {
  const lines = [`# ${card.name}`];
  if (card.aliases.length) lines.push(`다른 이름: ${card.aliases.join(', ')}`);
  for (const [key, value] of card.fields) lines.push(`${key}: ${value}`);
  if (card.description) lines.push('', card.description);
  return `${lines.join('\n')}\n`;
}

function namesOf(card: { name: string; aliases: string[] }): string {
  return [card.name, ...card.aliases].join('\t');
}

function LoadedCard({ root, initial, locked }: { root: string; initial: Card; locked: boolean }) {
  const types = useApp((s) => s.overview!.cardTypes);
  const projectKind = useApp((s) => s.overview!.project.kind);
  const appearsIn = useApp((s) => s.cardCounts[initial.id]);
  const [card, setCard] = useState(initial);
  const [alias, setAlias] = useState('');
  const descriptionRef = useRef<HTMLTextAreaElement>(null);
  // While the name box is empty the card keeps its last name on disk.
  const savedName = useRef(initial.name);
  const savedNames = useRef(namesOf(initial));

  // Saves run one after another, like the manuscript's (editor/session.ts),
  // and report to the 저장 표시 in the top bar as `card:<id>`.
  const { pending, schedule } = useDebouncedSave<Card>(
    {
      key: `card:${initial.id}`,
      prepare: (next) => (next.name.trim() ? next : { ...next, name: savedName.current }),
      rescue: (value) => ({ item: `card-${value.id}`, content: { text: cardText(value) } }),
      save: async (toSave) => {
        const summary = await api.cardSave(root, toSave);
        savedName.current = summary.name;
        patchCardSummary(summary);
        const names = namesOf(summary);
        if (names !== savedNames.current) {
          savedNames.current = names;
          void refreshCardCounts();
        }
      },
    },
    [root],
  );

  // Another device changed this card: take its version, unless it is being
  // changed here too (then this device's save wins, as before).
  const reloads = useApp((s) => s.cardReloads[initial.id] ?? 0);
  const seenReloads = useRef(reloads);
  useEffect(() => {
    if (reloads === seenReloads.current) return;
    seenReloads.current = reloads;
    if (pending.current) return;
    let alive = true;
    api.cardLoad(root, initial.id).then(
      (loaded) => {
        if (!alive || pending.current) return;
        savedName.current = loaded.name;
        savedNames.current = namesOf(loaded);
        setCard(loaded);
      },
      () => {
        // Gone or being written; the list catches up with the next change.
      },
    );
    return () => {
      alive = false;
    };
  }, [reloads, root, initial.id]);

  useAutoHeight(descriptionRef, card.description, 120);

  const change = (patch: Partial<Card>) => {
    const next = { ...card, ...patch };
    setCard(next);
    schedule(next);
  };

  const addAlias = () => {
    const value = alias.trim();
    if (!value || value === card.name || card.aliases.includes(value)) {
      setAlias('');
      return;
    }
    change({ aliases: [...card.aliases, value] });
    setAlias('');
  };

  const setField = (i: number, key: string, value: string) =>
    change({ fields: card.fields.map((f, j): [string, string] => (j === i ? [key, value] : f)) });

  const noun = docNoun(projectKind);

  return (
    <div className="doc-scroll">
      <article className="page card-page">
        {/* 읽기 전용 잠금 turns every field off at once. */}
        <fieldset className="plain" disabled={locked}>
          <div className="card-head">
            <select
              className="card-kind"
              value={card.cardType}
              onChange={(e) => change({ cardType: e.target.value })}
              aria-label="분류"
            >
              {types.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
            <div className="grow" />
            <button
              type="button"
              className="icon-btn"
              aria-label="카드 메뉴"
              onClick={(e) => openMenu(e, [{ label: '휴지통으로', danger: true, onSelect: () => void trashCard(card.id) }])}
            >
              <Icon name="more" />
            </button>
          </div>
          <input
            className="doc-title-input"
            value={card.name}
            placeholder="이름"
            aria-label="이름"
            autoFocus={!card.name}
            onChange={(e) => change({ name: e.target.value })}
            onBlur={() => {
              if (!card.name.trim()) change({ name: savedName.current });
            }}
          />
          <p className="card-appears">
            {appearsIn === undefined ? '' : appearsIn === 0 ? '아직 원고에 나오지 않음' : `${appearsIn}개 ${noun}에 나옴`}
          </p>

          <section className="card-section">
            <h3>다른 이름</h3>
            <div className="chips">
              {card.aliases.map((a) => (
                <span key={a} className="chip-tag">
                  {a}
                  <button
                    type="button"
                    aria-label={`${a} 빼기`}
                    onClick={() => change({ aliases: card.aliases.filter((x) => x !== a) })}
                  >
                    <Icon name="close" size={12} />
                  </button>
                </span>
              ))}
              <input
                className="chip-input"
                value={alias}
                placeholder="다른 이름 적고 Enter"
                aria-label="다른 이름 더하기"
                onChange={(e) => setAlias(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
                    e.preventDefault();
                    addAlias();
                  }
                }}
                onBlur={addAlias}
              />
            </div>
            <label className="check">
              <input type="checkbox" checked={card.highlight} onChange={(e) => change({ highlight: e.target.checked })} />
              원고에서 이 이름들을 표시하기
            </label>
          </section>

          <section className="card-section">
            <h3>항목</h3>
            <div className="card-fields">
              {card.fields.map(([key, value], i) => (
                <div key={i} className="card-field">
                  <input value={key} aria-label="항목 이름" onChange={(e) => setField(i, e.target.value, value)} />
                  <input value={value} aria-label={`${key} 내용`} onChange={(e) => setField(i, key, e.target.value)} />
                  <button
                    type="button"
                    className="icon-btn tiny"
                    aria-label={`${key} 항목 빼기`}
                    onClick={() => change({ fields: card.fields.filter((_, j) => j !== i) })}
                  >
                    <Icon name="close" size={12} />
                  </button>
                </div>
              ))}
            </div>
            <button type="button" className="btn small" onClick={() => change({ fields: [...card.fields, ['', '']] })}>
              <Icon name="plus" size={13} />
              항목 더하기
            </button>
          </section>

          <section className="card-section">
            <h3>서술</h3>
            <textarea
              ref={descriptionRef}
              className="card-description"
              value={card.description}
              placeholder="자유롭게 적으세요"
              aria-label="서술"
              onChange={(e) => change({ description: e.target.value })}
            />
          </section>
        </fieldset>
      </article>
    </div>
  );
}
