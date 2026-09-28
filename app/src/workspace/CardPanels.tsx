// Right-column views for setting cards: a card preview, where a card appears
// (등장 위치), and the cards appearing in the open chapter (등장 설정).

import { useEffect, useMemo, useState } from 'react';
import type { Editor } from '@tiptap/core';
import { api } from '../api';
import type { Appearance, Card } from '../api/types';
import { castOf, nameIndex, nameKey, type NameIndex } from '../editor/cards';
import { Icon } from '../components/Icon';
import { errorText, num } from '../lib/format';
import { UNTITLED, docNoun, docNumber } from '../lib/labels';
import { findDoc, jumpTo, openCard, previewCard, toastError, useApp } from '../store';

/** Names to highlight, rebuilt only when a highlighted name changes. */
export function useNameIndex(): NameIndex | null {
  const cards = useApp((s) => s.overview!.cards);
  const key = nameKey(cards);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- `key` stands for `cards`
  return useMemo(() => nameIndex(cards), [key]);
}

export function CardPreview({ cardId }: { cardId: string }) {
  const root = useApp((s) => s.overview!.root);
  const types = useApp((s) => s.overview!.cardTypes);
  const summary = useApp((s) => s.overview!.cards.find((c) => c.id === cardId));
  const [card, setCard] = useState<Card | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api.cardLoad(root, cardId).then(
      (c) => alive && setCard(c),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
    // Reload when the card was saved (its summary changes).
  }, [root, cardId, summary]);

  return (
    <div className="card-preview">
      <div className="card-preview-head">
        <span className="section-label">설정 카드</span>
        <button type="button" className="btn small" onClick={() => void openCard(cardId)}>
          편집
        </button>
        <button type="button" className="icon-btn tiny" aria-label="미리보기 닫기" onClick={() => previewCard(null)}>
          <Icon name="close" size={13} />
        </button>
      </div>
      {error && <p className="empty-note error-note">{error}</p>}
      {card && (
        <>
          <h2 className="card-preview-name">{card.name}</h2>
          <p className="meta">
            {types.find((t) => t.id === card.cardType)?.name}
            {card.aliases.length > 0 && ` · ${card.aliases.join(', ')}`}
          </p>
          <dl className="card-preview-fields">
            {card.fields
              .filter(([, v]) => v.trim())
              .map(([k, v], i) => (
                <div key={i}>
                  <dt>{k}</dt>
                  <dd>{v}</dd>
                </div>
              ))}
          </dl>
          {card.description && <p className="card-preview-text">{card.description}</p>}
          <Appearances cardId={cardId} compact />
        </>
      )}
    </div>
  );
}

/** Chapters where a card's names appear; click a line to go there. */
export function Appearances({ cardId, compact = false }: { cardId: string; compact?: boolean }) {
  const ov = useApp((s) => s.overview)!;
  const names = useApp((s) => {
    const card = s.overview!.cards.find((c) => c.id === cardId);
    return card ? [card.name, ...card.aliases].join('	') : '';
  });
  const [list, setList] = useState<Appearance[] | null>(null);

  useEffect(() => {
    let alive = true;
    api.cardAppearances(ov.root, cardId).then(
      (l) => alive && setList(l),
      (e) => alive && toastError('등장 위치를 찾지 못함', e),
    );
    return () => {
      alive = false;
    };
  }, [ov.root, cardId, names]);

  if (!list) return null;
  const noun = docNoun(ov.project.kind);
  if (list.length === 0) return <p className="empty-note">아직 원고에 나오지 않습니다.</p>;
  return (
    <div className="appearances">
      <h3 className="section-label">
        {num(list.length)}개 {noun}에 나옴
      </h3>
      {list.map((a) => {
        const place = findDoc(ov, a.docId);
        const label = place
          ? `${place.number !== null ? `${docNumber(ov.project.kind, place.number)} · ` : ''}${place.doc.title || UNTITLED}`
          : UNTITLED;
        return (
          <section key={a.docId} className="find-doc">
            <h3>
              <span className="ellipsis">{label}</span>
              <span className="count">{a.count}번</span>
            </h3>
            {!compact && (
              <ul>
                {a.samples.map((m) => (
                  <li key={`${m.block}:${m.start}`}>
                    <button
                      type="button"
                      className="find-hit"
                      onClick={() => void jumpTo({ docId: a.docId, block: m.block, start: m.start, end: m.end })}
                    >
                      {m.before}
                      <mark>{m.text}</mark>
                      {m.after}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        );
      })}
    </div>
  );
}

/** 등장 설정: cards whose names appear in the open chapter. */
export function CastTab({ editor }: { editor: Editor }) {
  const cards = useApp((s) => s.overview!.cards);
  const types = useApp((s) => s.overview!.cardTypes);
  const index = useNameIndex();
  const [cast, setCast] = useState<Map<string, number>>(() => castOf(editor.state.doc, index));

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const refresh = () => {
      clearTimeout(timer);
      timer = setTimeout(() => setCast(castOf(editor.state.doc, index)), 300);
    };
    setCast(castOf(editor.state.doc, index));
    editor.on('update', refresh);
    return () => {
      clearTimeout(timer);
      editor.off('update', refresh);
    };
  }, [editor, index]);

  const present = cards.filter((c) => cast.has(c.id)).sort((a, b) => (cast.get(b.id) ?? 0) - (cast.get(a.id) ?? 0));
  if (!cards.length) {
    return <p className="empty-note">설정집에 카드가 없습니다. 왼쪽 설정집에서 인물·장소·용어 카드를 만들면, 원고에 나오는 카드가 여기에 모입니다.</p>;
  }
  if (!present.length) return <p className="empty-note">이 문서에 나오는 설정 카드가 없습니다.</p>;
  return (
    <ul className="cast-list">
      {present.map((c) => (
        <li key={c.id}>
          <button type="button" className="cast-item" onClick={() => previewCard(c.id)}>
            <span className="cast-top">
              <strong>{c.name}</strong>
              <span className="meta">{types.find((t) => t.id === c.cardType)?.name}</span>
              <span className="grow" />
              <span className="count">{cast.get(c.id)}번 나옴</span>
            </span>
            {c.summary && <span className="cast-summary">{c.summary}</span>}
          </button>
        </li>
      ))}
    </ul>
  );
}
