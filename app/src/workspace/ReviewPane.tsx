// 교정본 검토: a corrected file read against what was sent, as a middle tab
// (target kind "review", one per exchange). Per chapter, its text as it is
// now with the editor's changes in place (deleted text struck through,
// inserted text underlined), 살펴볼 곳 softly highlighted and the editor's
// notes beside; a list to accept or reject each change, or a whole class.
// Nothing changes the manuscript until 반영하기 (`exchange_apply`).

import { useEffect, useMemo, useRef, useState } from 'react';
import type { JSONContent } from '@tiptap/core';
import { api } from '../api';
import type { ChapterReview, Correction, EditorNote, ExchangeInfo, Review, ReviewLook } from '../api/types';
import { Icon } from '../components/Icon';
import { blocksFromJSON } from '../editor/counts';
import { errorText, num } from '../lib/format';
import { UNTITLED, docNoun, withSubject } from '../lib/labels';
import {
  CLASSES,
  CLASS_LABEL,
  type ChangeView,
  type Choices,
  type Piece,
  SCENE,
  acceptClass,
  acceptRemaining,
  canAccept,
  changeView,
  chooseChange,
  chooseNote,
  classCounts,
  clearClass,
  dayLabel,
  decisionsOf,
  howText,
  keepOpen,
  remaining,
  reviewParagraphs,
  sentChaptersText,
  visible,
} from '../lib/review';
import {
  applyReview,
  focusNote,
  keepReviewDraft,
  reviewDraft,
  saveEverything,
  selectDoc,
  takeBackCorrected,
  useApp,
} from '../store';

/** Why a change cannot go in by itself (겹침), in one sentence. */
const OVERLAP_TEXT = '보낸 뒤에 이 문단을 고치셔서 그대로 넣으면 고치신 글이 지워질 수 있으니, 원고를 열어 직접 고쳐 주세요.';

interface Loaded {
  review: Review;
  ex: ExchangeInfo | null;
  /** Each chapter's paragraphs as they are now. */
  texts: Record<string, string[]>;
}

function paragraphTexts(body: JSONContent): string[] {
  return blocksFromJSON(body).map((b) => (b.scene ? SCENE : b.lines.join('\n')));
}

export function ReviewPane({ exchangeId, active }: { exchangeId: string; active: boolean }) {
  const root = useApp((s) => s.overview!.root);
  const version = useApp((s) => s.exchangesVersion);
  const [data, setData] = useState<Loaded | 'none' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [choices, setChoicesState] = useState<Choices>(() => reviewDraft(exchangeId));
  const [chapterId, setChapterId] = useState<string | null>(null);
  const [focus, setFocus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const setChoices = (next: Choices) => {
    setChoicesState(next);
    keepReviewDraft(exchangeId, next);
  };

  // (Re)read when the tab is shown and after anything about this exchange
  // changed: the writer may have written on since.
  useEffect(() => {
    if (!active) return;
    let alive = true;
    void (async () => {
      try {
        await saveEverything();
        const review = await api.exchangeReview(root, exchangeId);
        if (!review) {
          if (alive) setData('none');
          return;
        }
        const ex = (await api.exchangeList(root)).find((e) => e.id === exchangeId) ?? null;
        const texts: Record<string, string[]> = {};
        for (const chapter of review.chapters) {
          if (!chapter.found || chapter.gone) continue;
          try {
            texts[chapter.docId] = paragraphTexts((await api.docLoad(root, chapter.docId)).body);
          } catch {
            // Shown as gone.
          }
        }
        if (!alive) return;
        setError(null);
        setData({ review, ex, texts });
        const kept = keepOpen(reviewDraft(exchangeId), review.chapters);
        setChoicesState(kept);
        keepReviewDraft(exchangeId, kept);
        useApp.setState((s) => ({ reviewFiles: { ...s.reviewFiles, [exchangeId]: review.file } }));
      } catch (e) {
        if (alive) setError(errorText(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [root, exchangeId, version, active]);

  if (error) return <div className="pane-message">교정본 검토를 열 수 없음 · {error}</div>;
  if (data === 'none') {
    return (
      <div className="pane-message">
        <p>이 보낸 원고에는 아직 읽은 교정본이 없습니다.</p>
        <button type="button" className="btn" onClick={() => void takeBackCorrected(exchangeId)}>
          교정본 가져오기…
        </button>
      </div>
    );
  }
  if (!data) return <div className="pane-message" />;

  const { review, ex, texts } = data;
  const chapter =
    review.chapters.find((c) => c.docId === chapterId) ??
    review.chapters.find((c) => c.changes.some((x) => x.state === 'pending')) ??
    review.chapters.find((c) => c.found) ??
    review.chapters[0];
  const heading = (c: ChapterReview) => ex?.chapters.find((s) => s.docId === c.docId)?.heading || c.title || UNTITLED;
  const all = review.chapters.flatMap((c) => c.changes);
  const pending = all.filter((c) => c.state === 'pending').length;
  const left = remaining(all, choices);
  const picks = Object.keys(choices.changes).length + Object.keys(choices.notes).length;

  const progress =
    pending === 0
      ? '바뀐 곳을 모두 정했습니다. 받아들인 곳은 원고에 들어갔습니다.'
      : picks > 0
        ? `고른 것 ${num(picks)}개는 ‘반영하기’를 눌러야 원고에 들어갑니다. 아직 정하지 않은 곳이 ${num(left)}군데 남았습니다.`
        : `바뀐 곳 ${num(pending)}군데가 남았습니다. 받아들이거나 되돌릴 곳을 고른 뒤 ‘반영하기’를 누르세요.`;

  const apply = async () => {
    if (busy || !picks) return;
    setBusy(true);
    await applyReview(exchangeId, decisionsOf(choices));
    setBusy(false);
  };

  return (
    <div className="review">
      <header className="review-head">
        <div className="review-title">
          <h2 className="ellipsis" title={review.file}>
            {review.file}
          </h2>
          <p>
            {ex ? `${dayLabel(ex.created)}에 보낸 원고(${sentChaptersText(ex)})와 비교했습니다.` : '보낸 원고와 비교했습니다.'}{' '}
            {dayLabel(review.at)}에 가져옴.
          </p>
          <p className="review-progress">{progress}</p>
        </div>
        <div className="review-actions">
          <button type="button" className="btn" onClick={() => void takeBackCorrected(exchangeId)}>
            다른 교정본 가져오기…
          </button>
          <button type="button" className="btn primary" disabled={!picks || busy} onClick={() => void apply()}>
            {picks ? `반영하기 · ${num(picks)}개` : '반영하기'}
          </button>
        </div>
      </header>

      {review.chapters.length > 1 && (
        <div className="review-chapters" role="tablist" aria-label="보낸 원고">
          {review.chapters.map((c) => {
            const n = remaining(c.changes, choices);
            const on = c.docId === chapter.docId;
            return (
              <button
                key={c.docId}
                type="button"
                role="tab"
                aria-selected={on}
                className={`review-chapter${on ? ' on' : ''}${c.found ? '' : ' missing'}`}
                onClick={() => {
                  setChapterId(c.docId);
                  setFocus(null);
                }}
              >
                <span className="ellipsis">{heading(c)}</span>
                {n > 0 && <span className="review-count">{num(n)}</span>}
              </button>
            );
          })}
        </div>
      )}

      <ChapterView
        key={chapter.docId}
        chapter={chapter}
        texts={texts[chapter.docId] ?? null}
        choices={choices}
        setChoices={setChoices}
        focus={focus}
        setFocus={setFocus}
      />
    </div>
  );
}

function ChapterView({
  chapter,
  texts,
  choices,
  setChoices,
  focus,
  setFocus,
}: {
  chapter: ChapterReview;
  texts: string[] | null;
  choices: Choices;
  setChoices: (c: Choices) => void;
  focus: string | null;
  setFocus: (id: string | null) => void;
}) {
  const ov = useApp((s) => s.overview!);
  const noun = docNoun(ov.project.kind);
  const page = useRef<HTMLDivElement>(null);
  const side = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => (texts ? reviewParagraphs(texts, chapter) : []), [texts, chapter]);
  const lookIds = useMemo(() => new Set(chapter.looks.map((l) => l.id)), [chapter]);

  // The picked item in sight on both sides.
  useEffect(() => {
    if (!focus) return;
    const sel = `[data-rv="${focus}"]`;
    page.current?.querySelector(sel)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    side.current?.querySelector(sel)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, [focus]);

  const open = chapter.changes.filter((c) => c.state === 'pending');
  const decided = chapter.changes.filter((c) => c.state !== 'pending');
  const counts = classCounts(chapter.changes, choices);
  const overlaps = open.filter((c) => c.overlap).length;
  const openChapter = () => void selectDoc(chapter.docId, true);

  return (
    <div className="review-body">
      <div className="review-text" ref={page}>
        {!chapter.found && <p className="review-notice">이 교정본에서는 이 {withSubject(noun)} 찾지 못했습니다. 다른 파일로 보냈다면 그 교정본을 가져오세요.</p>}
        {chapter.gone && <p className="review-notice">이 {withSubject(noun)} 휴지통으로 가서 고칠 수 없습니다. 휴지통에서 되살리면 다시 볼 수 있습니다.</p>}
        {chapter.found && !chapter.gone && overlaps > 0 && (
          <p className="review-notice">
            보낸 뒤에 이 {noun}의 문단 몇 곳을 고치셨습니다. 그 문단에 걸린 {num(overlaps)}군데는 본문에 보이지 않고 오른쪽 목록에만 있습니다.
          </p>
        )}
        {texts && (
          <article className="review-page" aria-label="교정본과 비교한 본문">
            {rows.map((pieces, i) =>
              texts[i] === SCENE ? (
                <p key={i} className="rv-scene" aria-label="장면 나눔">
                  {ov.project.sceneBreak}
                </p>
              ) : (
                <p key={i} className="rv-para">
                  {pieces.map((piece, k) => (
                    <PieceView
                      key={k}
                      piece={piece}
                      view={piece.change ? viewOf(chapter, piece.change, choices) : 'open'}
                      focus={focus}
                      lookIds={lookIds}
                      onPick={setFocus}
                    />
                  ))}
                  {pieces.length === 0 && <br />}
                </p>
              ),
            )}
          </article>
        )}
      </div>

      <aside className="review-side" ref={side} aria-label="바뀐 곳 목록">
        {open.length > 0 && (
          <section className="rv-section">
            <h3>종류별로</h3>
            <div className="rv-classes">
              {CLASSES.map((cls) => {
                const n = open.filter((c) => c.class === cls).length;
                if (!n) return null;
                const { appliable, accepting } = counts[cls];
                return (
                  <div key={cls} className="rv-class-row">
                    <span className="grow">
                      {CLASS_LABEL[cls]} <span className="rv-n">{num(n)}군데</span>
                    </span>
                    {appliable === 0 ? (
                      <span className="rv-picked">직접 고칠 곳만 남음</span>
                    ) : accepting < appliable ? (
                      <button type="button" className="btn small" onClick={() => setChoices(acceptClass(choices, chapter.changes, cls))}>
                        모두 받아들이기
                      </button>
                    ) : (
                      <button type="button" className="btn small ghost" onClick={() => setChoices(clearClass(choices, chapter.changes, cls))}>
                        <Icon name="check" size={13} />
                        모두 받아들임 · 취소
                      </button>
                    )}
                  </div>
                );
              })}
              {remaining(chapter.changes, choices) > 0 && open.some((c) => canAccept(c) && !choices.changes[c.id]) && (
                <button type="button" className="btn small dashed" onClick={() => setChoices(acceptRemaining(choices, chapter.changes))}>
                  남은 곳 모두 받아들이기
                </button>
              )}
            </div>
          </section>
        )}

        <section className="rv-section">
          <h3>바뀐 곳</h3>
          {open.length === 0 && <p className="empty-note">이 {noun}에는 정할 곳이 남지 않았습니다.</p>}
          <ol className="rv-list">
            {open.map((c) => (
              <ChangeItem
                key={c.id}
                change={c}
                view={changeView(c, choices)}
                on={focus === c.id}
                onFocus={() => setFocus(c.id)}
                onChoose={(choice) => setChoices(chooseChange(choices, c.id, choice))}
                onOpen={openChapter}
                noun={noun}
              />
            ))}
          </ol>
          {decided.length > 0 && (
            <details className="rv-decided">
              <summary>이미 정한 곳 {num(decided.length)}군데</summary>
              <ol className="rv-list">
                {decided.map((c) => (
                  <ChangeItem key={c.id} change={c} view={c.state as ChangeView} on={focus === c.id} onFocus={() => setFocus(c.id)} noun={noun} />
                ))}
              </ol>
            </details>
          )}
        </section>

        {chapter.looks.length > 0 && (
          <section className="rv-section">
            <h3>살펴볼 곳</h3>
            <p className="hint">편집자가 고치지 않고 표시만 해 둔 곳입니다. 글은 바뀌지 않습니다.</p>
            <ul className="rv-list">
              {chapter.looks.map((l) => (
                <LookItem key={l.id} look={l} on={focus === l.id} onFocus={() => setFocus(l.id)} />
              ))}
            </ul>
          </section>
        )}

        {chapter.notes.length > 0 && (
          <section className="rv-section">
            <h3>편집자 메모</h3>
            <ul className="rv-list">
              {chapter.notes.map((n) => (
                <NoteItem
                  key={n.id}
                  note={n}
                  choice={choices.notes[n.id] ?? null}
                  on={focus === n.id}
                  onFocus={() => setFocus(n.id)}
                  onChoose={(choice) => setChoices(chooseNote(choices, n.id, choice))}
                  docId={chapter.docId}
                  noun={noun}
                />
              ))}
            </ul>
          </section>
        )}
      </aside>
    </div>
  );
}

function viewOf(chapter: ChapterReview, id: string, choices: Choices): ChangeView {
  const c = chapter.changes.find((x) => x.id === id);
  return c ? changeView(c, choices) : 'open';
}

function PieceView({
  piece,
  view,
  focus,
  lookIds,
  onPick,
}: {
  piece: Piece;
  view: ChangeView;
  focus: string | null;
  lookIds: Set<string>;
  onPick: (id: string) => void;
}) {
  const marks = piece.starts.map((id) => (
    <button
      key={id}
      type="button"
      className={`rv-pin ${lookIds.has(id) ? 'look' : 'note'}${focus === id ? ' on' : ''}`}
      data-rv={id}
      aria-label={lookIds.has(id) ? '살펴볼 곳' : '편집자 메모'}
      title={lookIds.has(id) ? '살펴볼 곳' : '편집자 메모'}
      onClick={() => onPick(id)}
    >
      {lookIds.has(id) ? '●' : <Icon name="note" size={11} />}
    </button>
  ));
  const cls = [
    piece.looks.length ? 'rv-look' : '',
    piece.notes.length ? 'rv-noted' : '',
    piece.looks.includes(focus ?? '') || piece.notes.includes(focus ?? '') ? 'on' : '',
  ]
    .filter(Boolean)
    .join(' ');
  if (piece.kind === 'text') {
    return (
      <>
        {marks}
        {cls ? <span className={cls}>{piece.text}</span> : piece.text}
      </>
    );
  }
  const Tag = piece.kind === 'del' ? 'del' : 'ins';
  const on = focus === piece.change;
  return (
    <>
      {marks}
      <Tag
        className={`rv-${piece.kind} ${view}${on ? ' on' : ''}${piece.para ? ' para' : ''}${piece.text.trim() ? '' : ' blank'}${cls ? ` ${cls}` : ''}`}
        data-rv={piece.kind === 'del' || !piece.para ? piece.change : undefined}
        title={piece.kind === 'del' ? (piece.para ? '문단 나눔을 지움' : '지운 글') : '넣은 글'}
        onClick={() => piece.change && onPick(piece.change)}
      >
        {piece.kind === 'ins' ? visible(piece.text) : piece.text}
      </Tag>
    </>
  );
}

function ChangeItem({
  change: c,
  view,
  on,
  onFocus,
  onChoose,
  onOpen,
  noun,
}: {
  change: Correction;
  view: ChangeView;
  on: boolean;
  onFocus: () => void;
  onChoose?: (choice: 'accept' | 'reject' | null) => void;
  onOpen?: () => void;
  noun: string;
}) {
  const how = howText(c);
  return (
    <li className={`rv-item ${view}${on ? ' on' : ''}`} data-rv={c.id} onClick={onFocus}>
      <div className="rv-line">
        <span className={`rv-class ${c.class}`}>{CLASS_LABEL[c.class]}</span>
        <span className="rv-quote">
          {c.lead && <span className="rv-around">{visible(c.lead)}</span>}
          {c.before && <del>{visible(c.before)}</del>}
          {c.after && <ins>{visible(c.after)}</ins>}
          {c.trail && <span className="rv-around">{visible(c.trail)}</span>}
        </span>
      </div>
      {how && <div className="rv-how">{how}</div>}
      {c.state === 'pending' && c.overlap && <p className="rv-overlap">{OVERLAP_TEXT}</p>}
      <div className="rv-buttons">
        {view === 'open' && onChoose && (
          <>
            <button type="button" className="btn small" disabled={!canAccept(c)} onClick={() => onChoose('accept')}>
              받아들이기
            </button>
            <button type="button" className="btn small ghost" onClick={() => onChoose('reject')}>
              되돌리기
            </button>
            {c.overlap && onOpen && (
              <button type="button" className="btn small ghost" onClick={onOpen}>
                {noun} 열기
              </button>
            )}
          </>
        )}
        {(view === 'accept' || view === 'reject') && onChoose && (
          <>
            <span className="rv-picked">
              <Icon name="check" size={13} />
              {view === 'accept' ? '받아들이기로 함' : '원래 글로 두기로 함'}
            </span>
            <button type="button" className="btn small ghost" onClick={() => onChoose(null)}>
              취소
            </button>
          </>
        )}
        {view === 'accepted' && <span className="rv-done">받아들임</span>}
        {view === 'rejected' && <span className="rv-done">원래 글로 둠</span>}
      </div>
    </li>
  );
}

function LookItem({ look: l, on, onFocus }: { look: ReviewLook; on: boolean; onFocus: () => void }) {
  const kinds = [l.highlight && '형광펜', l.underline && '밑줄', l.color && '글자색'].filter(Boolean).join(', ');
  return (
    <li className={`rv-item look${on ? ' on' : ''}`} data-rv={l.id} onClick={onFocus}>
      <div className="rv-line">
        <span className="rv-class look">{kinds || '표시'}</span>
        <span className="rv-quote">“{l.quote}”</span>
      </div>
      {!l.now && <div className="rv-how">지금 원고에서는 이 글을 찾지 못했습니다.</div>}
    </li>
  );
}

function NoteItem({
  note: n,
  choice,
  on,
  onFocus,
  onChoose,
  docId,
  noun,
}: {
  note: EditorNote;
  choice: 'keep' | 'drop' | null;
  on: boolean;
  onFocus: () => void;
  onChoose: (choice: 'keep' | 'drop' | null) => void;
  docId: string;
  noun: string;
}) {
  const showMemo = async () => {
    await selectDoc(docId, true);
    if (n.memo) focusNote(n.memo);
  };
  return (
    <li className={`note rv-note${on ? ' focused' : ''}${n.state !== 'pending' ? ' done' : ''}`} data-rv={n.id} onClick={onFocus}>
      <div className="note-on">
        {n.author || '편집자'}
        {n.date && ` · ${n.date}`}
      </div>
      {n.quote ? <blockquote className="rv-note-quote">{n.quote}</blockquote> : <div className="rv-how">{noun} 전체에 대한 메모</div>}
      <p className="rv-note-text">{n.text}</p>
      <div className="rv-buttons">
        {n.state === 'pending' && !choice && (
          <>
            <button type="button" className="btn small" onClick={() => onChoose('keep')}>
              메모로 남기기
            </button>
            <button type="button" className="btn small ghost" onClick={() => onChoose('drop')}>
              버리기
            </button>
          </>
        )}
        {n.state === 'pending' && choice && (
          <>
            <span className="rv-picked">
              <Icon name="check" size={13} />
              {choice === 'keep' ? '메모로 남기기로 함' : '버리기로 함'}
            </span>
            <button type="button" className="btn small ghost" onClick={() => onChoose(null)}>
              취소
            </button>
          </>
        )}
        {n.state === 'accepted' && (
          <>
            <span className="rv-done">메모로 남김</span>
            {n.memo && (
              <button type="button" className="btn small ghost" onClick={() => void showMemo()}>
                메모 보기
              </button>
            )}
          </>
        )}
        {n.state === 'rejected' && <span className="rv-done">버림</span>}
      </div>
    </li>
  );
}
