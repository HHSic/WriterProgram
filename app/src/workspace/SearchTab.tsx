// 찾기 탭 (S9): find across the chapter, the part or the whole project, and
// replace one match or all of them.

import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../api';
import type { SearchMatch, SearchResult } from '../api/types';
import { buildRegex, replaceMatch, setHighlight } from '../editor/search';
import { errorText, num } from '../lib/format';
import { flushAll } from '../lib/flush';
import { UNTITLED, docNumber } from '../lib/labels';
import {
  findDoc,
  jumpTo,
  openDialog,
  refreshOverview,
  saveEverything,
  showToast,
  toastError,
  useApp,
  type FindScope,
} from '../store';

type Found = SearchMatch & { docId: string };

const SCOPES: { id: FindScope; label: string }[] = [
  { id: 'doc', label: '이 문서' },
  { id: 'part', label: '이 부' },
  { id: 'all', label: '작품 전체' },
];

export function SearchTab() {
  const ov = useApp((s) => s.overview)!;
  const activeDocId = useApp((s) => s.activeDocId);
  const editor = useApp((s) => s.editor);
  const request = useApp((s) => s.findRequest);
  const docVersion = useApp((s) => s.docVersion);
  const [text, setText] = useState('');
  const [replacement, setReplacement] = useState('');
  const [scope, setScope] = useState<FindScope>('all');
  const [regex, setRegex] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [result, setResult] = useState<SearchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [current, setCurrent] = useState(-1);
  const [runs, setRuns] = useState(0);
  const findRef = useRef<HTMLInputElement>(null);
  const replaceRef = useRef<HTMLInputElement>(null);

  // Requests from the sidebar search box and Ctrl+F / Ctrl+H.
  useEffect(() => {
    if (!request) return;
    if (request.text !== undefined) setText(request.text);
    if (request.scope) setScope(request.scope);
    const input = request.focus === 'replace' ? replaceRef.current : findRef.current;
    input?.focus();
    input?.select();
  }, [request]);

  // Documents in scope, as a string so the search does not rerun on every render.
  const scopeKey = (() => {
    if (scope === 'all') return '';
    if (scope === 'doc') return activeDocId ?? '-';
    const place = activeDocId ? findDoc(ov, activeDocId) : null;
    if (!place) return '-';
    return (place.part ? place.part.docs : ov.planning).map((d) => d.id).join(',') || '-';
  })();
  const docIds = useMemo(() => (scopeKey === '' ? undefined : scopeKey === '-' ? [] : scopeKey.split(',')), [scopeKey]);

  useEffect(() => {
    if (!text) {
      setResult(null);
      setError(null);
      return;
    }
    let alive = true;
    const timer = setTimeout(async () => {
      await flushAll();
      try {
        const found = await api.search(ov.root, { text, regex, wholeWord, docIds });
        if (!alive) return;
        setResult(found);
        setError(null);
        setCurrent(-1);
      } catch (e) {
        if (!alive) return;
        setResult(null);
        setError(errorText(e));
      }
    }, 250);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [ov.root, text, regex, wholeWord, docIds, runs, docVersion]);

  // Highlight matches in the open document while this tab is showing.
  useEffect(() => {
    if (!editor) return;
    setHighlight(editor, text ? buildRegex({ text, regex, wholeWord }) : null);
    return () => setHighlight(editor, null);
  }, [editor, text, regex, wholeWord]);

  const flat: Found[] = useMemo(
    () => result?.docs.flatMap((d) => d.matches.map((m) => ({ ...m, docId: d.docId }))) ?? [],
    [result],
  );

  const go = (index: number) => {
    const found = flat[index];
    if (!found) return;
    setCurrent(index);
    void jumpTo({ docId: found.docId, block: found.block, start: found.start, end: found.end });
  };

  const step = (dir: 1 | -1) => {
    if (!flat.length) return;
    go((current + dir + flat.length) % flat.length);
  };

  /** The replacement for one match, with $1 groups filled in for regex search. */
  const expanded = (found: Found) => {
    if (!regex) return replacement;
    const one = buildRegex({ text, regex, wholeWord });
    if (!one) return replacement;
    return found.text.replace(new RegExp(one.source, 'u'), replacement);
  };

  const replaceOne = () => {
    const found = flat[current];
    if (!found) {
      step(1);
      return;
    }
    if (found.docId !== activeDocId || !editor) {
      go(current);
      return;
    }
    if (!replaceMatch(editor, found, expanded(found))) {
      showToast({ text: '그 사이 글이 바뀌어 다시 찾았습니다' });
    }
    setTimeout(() => setRuns((n) => n + 1), 1000);
  };

  const replaceAll = () => {
    if (!result?.total) return;
    const where = scope === 'all' ? '작품 전체' : scope === 'part' ? '이 부' : '이 문서';
    openDialog({
      kind: 'confirm',
      title: '모두 바꾸기',
      message: `${where}에서 찾은 ${num(result.total)}곳을 바꿉니다 ('${text}' → '${replacement}'). 바뀌는 문서마다 '바꾸기 전' 기록이 남아서 되돌릴 수 있습니다.`,
      confirm: `${num(result.total)}곳 바꾸기`,
      onConfirm: async () => {
        if (!(await saveEverything())) return;
        try {
          const outcome = await api.replaceAll(ov.root, { text, regex, wholeWord, docIds }, replacement);
          useApp.setState((s) => ({ docVersion: s.docVersion + 1, recordsVersion: s.recordsVersion + 1 }));
          await refreshOverview();
          showToast({
            text: `${num(outcome.replaced)}곳 바꿈 · 문서 ${outcome.docs.length}개`,
            action: {
              label: '되돌리기',
              run: () => {
                void (async () => {
                  for (const d of outcome.docs) await api.snapshotRestore(ov.root, d.docId, d.snapshot.id);
                  useApp.setState((s) => ({ docVersion: s.docVersion + 1, recordsVersion: s.recordsVersion + 1 }));
                  await refreshOverview();
                })().catch((e) => toastError('되돌리지 못함', e));
              },
            },
          });
        } catch (e) {
          toastError('바꾸지 못함', e);
        }
      },
    });
  };

  const docLabel = (docId: string) => {
    const p = findDoc(ov, docId);
    if (!p) return UNTITLED;
    const title = p.doc.title || UNTITLED;
    return p.number !== null ? `${docNumber(ov.project.kind, p.number)} · ${title}` : title;
  };

  let index = -1;
  return (
    <div className="find">
      <div className="find-inputs">
        <input
          ref={findRef}
          value={text}
          placeholder="찾을 말"
          aria-label="찾을 말"
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
              e.preventDefault();
              step(e.shiftKey ? -1 : 1);
            }
          }}
        />
        <input
          ref={replaceRef}
          value={replacement}
          placeholder="바꿀 말"
          aria-label="바꿀 말"
          onChange={(e) => setReplacement(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
              e.preventDefault();
              replaceOne();
            }
          }}
        />
      </div>
      <div className="segmented small">
        {SCOPES.map((s) => (
          <label key={s.id} className={scope === s.id ? 'on' : ''}>
            <input type="radio" checked={scope === s.id} onChange={() => setScope(s.id)} />
            {s.label}
          </label>
        ))}
      </div>
      <div className="row find-options">
        <label className="check">
          <input type="checkbox" checked={wholeWord} onChange={(e) => setWholeWord(e.target.checked)} />
          단어 단위
        </label>
        <label className="check">
          <input type="checkbox" checked={regex} onChange={(e) => setRegex(e.target.checked)} />
          정규식
        </label>
      </div>
      <div className="row find-actions">
        <button type="button" className="btn small" disabled={!flat.length} onClick={replaceOne}>
          바꾸기
        </button>
        <button type="button" className="btn small" disabled={!flat.length} onClick={replaceAll}>
          모두 바꾸기{result?.total ? ` (${num(result.total)}곳)` : ''}
        </button>
      </div>

      {error && <p className="empty-note error-note">{error}</p>}
      {result && result.total === 0 && <p className="empty-note">찾은 곳이 없습니다.</p>}
      {result && result.total > 0 && (
        <p className="find-summary">
          {num(result.total)}곳 · 문서 {result.docs.length}개{result.truncated ? ' (앞부분만 보여 줌)' : ''}
        </p>
      )}
      <div className="find-results">
        {result?.docs.map((d) => (
          <section key={d.docId} className="find-doc">
            <h3>
              <span className="ellipsis">{docLabel(d.docId)}</span>
              <span className="count">{d.matches.length}곳</span>
            </h3>
            <ul>
              {d.matches.map((m) => {
                index += 1;
                const i = index;
                return (
                  <li key={`${m.block}:${m.start}`}>
                    <button type="button" className={`find-hit${i === current ? ' on' : ''}`} onClick={() => go(i)}>
                      {m.before}
                      <mark>{m.text}</mark>
                      {m.after}
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
