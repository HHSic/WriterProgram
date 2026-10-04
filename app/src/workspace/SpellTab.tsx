// 맞춤법 탭: words the dictionary does not know across the chapter, the part
// or the whole work, listed like find results. Click one to go there; ⋯ (or
// a right click) gives what the writer may have meant, 사전에 넣기 and 무시.
// Locked chapters (완료 회차 잠금) are listed but never changed.

import { useEffect, useMemo, useState, type MouseEvent } from 'react';
import { api } from '../api';
import type { SearchMatch, SpellStatus, SpellWorkCheck } from '../api/types';
import { Icon } from '../components/Icon';
import { replaceMatch } from '../editor/search';
import { errorText, num } from '../lib/format';
import { flushAll } from '../lib/flush';
import { UNTITLED, docNoun, docNumber } from '../lib/labels';
import { skippedText, unreadableIn } from '../lib/unreadable';
import {
  downloadText,
  findDoc,
  jumpTo,
  offerSpellDictionary,
  showToast,
  spellKnown,
  spellKnownKey,
  useApp,
  type FindScope,
} from '../store';
import { openSpellMenuAt } from './spellMenu';

type Found = SearchMatch & { docId: string; locked: boolean };

const SCOPES: { id: FindScope; label: string }[] = [
  { id: 'doc', label: '이 문서' },
  { id: 'part', label: '이 부' },
  { id: 'all', label: '작품 전체' },
];

/** Before the dictionary is on this device: what it is and the button to fetch it. */
export function SpellOffer({ status }: { status: SpellStatus }) {
  return (
    <div className="spell-offer">
      <p>
        맞춤법 검사를 쓰려면 한국어 맞춤법 사전을 한 번 받아야 합니다. 받은 뒤에는 인터넷 없이 이 기기 안에서만 검사하고, 원고는 어디에도
        보내지 않습니다.
      </p>
      <button type="button" className="btn small primary" onClick={offerSpellDictionary}>
        맞춤법 사전 받기
      </button>
      <span className="hint">{downloadText(status)} (hunspell-dict-ko)</span>
    </div>
  );
}

export function SpellTab() {
  const status = useApp((s) => s.spellStatus);
  if (!status) return <p className="empty-note">맞춤법 사전을 확인하는 중</p>;
  if (!status.installed) {
    return (
      <div className="find spell-tab">
        <SpellOffer status={status} />
      </div>
    );
  }
  return <SpellResults />;
}

function SpellResults() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const editor = useApp((s) => s.editor);
  const docVersion = useApp((s) => s.docVersion);
  const knownKey = useApp((s) => spellKnownKey(s.overview!.cards, s.spellWords, s.spellIgnored));
  const [scope, setScope] = useState<FindScope>('all');
  const [result, setResult] = useState<SpellWorkCheck | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [runs, setRuns] = useState(0);
  const [current, setCurrent] = useState(-1);

  const scopeKey = (() => {
    if (scope === 'all') return '';
    if (scope === 'doc') return activeDocId ?? '-';
    const place = activeDocId ? findDoc(ov, activeDocId) : null;
    if (!place) return '-';
    const unreadable = unreadableIn(ov.unreadable, place.part?.id ?? null).map((u) => u.id);
    return [...(place.part ? place.part.docs : ov.planning).map((d) => d.id), ...unreadable].join(',') || '-';
  })();
  const docIds = useMemo(() => (scopeKey === '' ? undefined : scopeKey === '-' ? [] : scopeKey.split(',')), [scopeKey]);

  useEffect(() => {
    let alive = true;
    setRunning(true);
    const timer = setTimeout(async () => {
      await flushAll();
      try {
        const found = await api.spellCheckWork(ov.root, spellKnown(), docIds);
        if (!alive) return;
        setResult(found);
        setError(null);
        setCurrent(-1);
      } catch (e) {
        if (!alive) return;
        setResult(null);
        setError(errorText(e));
      } finally {
        if (alive) setRunning(false);
      }
    }, 300);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [ov.root, docIds, knownKey, runs, docVersion]);

  const flat: Found[] = useMemo(
    () => result?.docs.flatMap((d) => d.issues.map((m) => ({ ...m, docId: d.docId, locked: d.locked }))) ?? [],
    [result],
  );

  const go = (index: number) => {
    const found = flat[index];
    if (!found) return;
    setCurrent(index);
    void jumpTo({ docId: found.docId, block: found.block, start: found.start, end: found.end });
  };

  /** Takes one item off the list at once; the next check brings the rest up to date. */
  const drop = (found: Found) => {
    setResult((r) =>
      r && {
        ...r,
        total: r.total - 1,
        docs: r.docs
          .map((d) =>
            d.docId === found.docId ? { ...d, issues: d.issues.filter((m) => !(m.block === found.block && m.start === found.start)) } : d,
          )
          .filter((d) => d.issues.length),
      },
    );
  };

  const fix = (found: Found) => (text: string) => {
    if (found.docId === activeDocId && editor) {
      if (!replaceMatch(editor, found, text)) {
        showToast({ text: '그 사이 글이 바뀌어 다시 검사했습니다' });
        setRuns((n) => n + 1);
        return;
      }
    } else {
      void jumpTo({ docId: found.docId, block: found.block, start: found.start, end: found.end, replace: { text: found.text, with: text } });
    }
    drop(found);
    setTimeout(() => setRuns((n) => n + 1), 1500);
  };

  const menu = (e: MouseEvent, found: Found) => {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const [x, y] = e.type === 'click' ? [rect.left, rect.bottom + 4] : [e.clientX, e.clientY];
    openSpellMenuAt(x, y, found.text, found.locked ? null : fix(found));
  };

  const noun = docNoun(ov.project.kind);
  const docLabel = (docId: string) => {
    const p = findDoc(ov, docId);
    if (!p) return UNTITLED;
    const title = p.doc.title || UNTITLED;
    return p.number !== null ? `${docNumber(ov.project.kind, p.number)} · ${title}` : title;
  };

  let index = -1;
  return (
    <div className="find spell-tab">
      <div className="segmented small">
        {SCOPES.map((s) => (
          <label key={s.id} className={scope === s.id ? 'on' : ''}>
            <input type="radio" checked={scope === s.id} onChange={() => setScope(s.id)} />
            {s.label}
          </label>
        ))}
      </div>
      <div className="row find-actions">
        <button type="button" className="btn small" disabled={running} onClick={() => setRuns((n) => n + 1)}>
          다시 검사
        </button>
      </div>
      <p className="hint spell-hint">
        설정집의 이름과 다른 이름은 맞는 말로 칩니다. 이름·소리 흉내 말처럼 사전에 없는 낱말은 ⋯에서 ‘사전에 넣기’를 고르세요.
      </p>

      {error && <p className="empty-note error-note">{error}</p>}
      {running && !result && <p className="empty-note">검사하는 중</p>}
      {result && result.total === 0 && <p className="empty-note">사전에 없는 낱말을 찾지 못했습니다.</p>}
      {result && result.skipped > 0 && <p className="empty-note">{skippedText(result.skipped, '문서')}</p>}
      {result && result.total > 0 && (
        <p className="find-summary">
          살펴볼 곳 {num(result.total)}곳 · 문서 {result.docs.length}개{result.truncated ? ' (앞부분만 보여 줌)' : ''}
          {running ? ' · 다시 검사하는 중' : ''}
        </p>
      )}
      <div className="find-results">
        {result?.docs.map((d) => (
          <section key={d.docId} className="find-doc">
            <h3>
              <span className="ellipsis">{docLabel(d.docId)}</span>
              {d.locked && (
                <span className="find-locked" title={`잠근 ${noun}: 고치지 않음`}>
                  <Icon name="lock" size={11} />
                  잠금
                </span>
              )}
              <span className="count">{d.issues.length}곳</span>
            </h3>
            <ul>
              {d.issues.map((m) => {
                index += 1;
                const i = index;
                const found = flat[i];
                return (
                  <li key={`${m.block}:${m.start}`} className="spell-hit-row">
                    <button
                      type="button"
                      className={`find-hit${i === current ? ' on' : ''}`}
                      onClick={() => go(i)}
                      onContextMenu={(e) => found && menu(e, found)}
                    >
                      {m.before}
                      <mark className="spell-word">{m.text}</mark>
                      {m.after}
                    </button>
                    <button
                      type="button"
                      className="more-btn"
                      aria-label={`${m.text}: 고칠 말과 사전에 넣기`}
                      title="고칠 말과 사전에 넣기"
                      onClick={(e) => found && menu(e, found)}
                    >
                      <Icon name="more" size={14} />
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}
