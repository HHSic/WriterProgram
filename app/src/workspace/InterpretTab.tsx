// 해석 tab (해석 모드): what the sentence under the cursor does and means, and
// the sentences it links to; with nothing picked, the chapter's reading as a
// whole. The reading is written by an assistant through writer-mcp
// (docs/reading.md); this only shows it.

import { useEffect } from 'react';
import type { InterpretPlace, InterpretSentence, InterpretView } from '../api/types';
import { placeAt } from '../editor/interpret';
import { timeLabel } from '../lib/format';
import { jumpTo, loadInterpretation, pickSentence, setInterpretMode, useApp } from '../store';

export function InterpretTab({ docId }: { docId: string }) {
  const mode = useApp((s) => s.interpretMode);
  const version = useApp((s) => s.interpretVersion);
  const loaded = useApp((s) => (s.interpret?.docId === docId ? s.interpret : null));
  const pick = useApp((s) => (s.interpretPick?.docId === docId ? s.interpretPick : null));

  // With the mode on, the document pane loads it (it needs it for the marks).
  useEffect(() => {
    if (!mode) void loadInterpretation(docId);
  }, [mode, docId, version]);

  const modeBar = (
    <div className="interpret-mode">
      <button type="button" className={`btn${mode ? ' dark' : ''}`} aria-pressed={mode} onClick={() => setInterpretMode(!mode)}>
        해석 모드 {mode ? '끄기' : '켜기'}
      </button>
      <small className="hint">{mode ? '문장을 누르면 그 문장의 해석이 여기 나옵니다.' : '켜고 문장을 누르면 그 문장의 해석과 연결된 문장이 나옵니다.'}</small>
    </div>
  );

  if (!loaded) return <div className="interpret">{modeBar}<p className="empty-note">해석을 읽는 중…</p></div>;
  const view = loaded.view;
  if (!view) {
    return (
      <div className="interpret">
        {modeBar}
        <p className="empty-note">
          이 회차는 아직 해석하지 않았습니다. AI 도우미(작품 설정 → AI 도우미 연결)를 연결한 Claude Code에서 “이 회차 해석해 줘”라고 하면 만들어져
          여기에 나옵니다.
        </p>
      </div>
    );
  }

  const current = pick ? placeAt(view.places, pick.block, pick.offset) : null;
  const sentence = current ? view.reading.sentences.find((s) => s.id === current.id) ?? null : null;
  const go = (place: InterpretPlace) => {
    pickSentence(docId, place.block, place.start);
    void jumpTo({ docId, block: place.block, start: place.start, end: place.end });
  };

  return (
    <div className="interpret">
      {modeBar}
      {view.changed && (
        <p className="interpret-stale" role="status">
          해석한 뒤 본문이 바뀌었습니다.{view.gone.length ? ` 찾지 못한 문장 ${view.gone.length}개는 빠졌습니다.` : ' 문장은 모두 찾았습니다.'}
        </p>
      )}
      {sentence && current ? (
        <SentenceCard view={view} sentence={sentence} go={go} />
      ) : (
        <>
          {pick && mode && <p className="empty-note">이 문장에는 해석이 없습니다. 밑줄 친 문장을 눌러 보세요.</p>}
          <Overview view={view} go={go} />
        </>
      )}
      <p className="meta interpret-by">
        해석: {view.reading.by || '도우미'} · {timeLabel(view.reading.made)}
      </p>
    </div>
  );
}

type Go = (place: InterpretPlace) => void;

function placeOf(view: InterpretView, id: string): InterpretPlace | null {
  return view.places.find((p) => p.id === id) ?? null;
}

function SentenceCard({ view, sentence, go }: { view: InterpretView; sentence: InterpretSentence; go: Go }) {
  const scene = [...view.reading.scenes].reverse().find((s) => s.block <= sentence.block);
  const byId = new Map(view.reading.sentences.map((s) => [s.id, s]));
  const incoming = view.reading.sentences.flatMap((s) =>
    s.links.filter((l) => l.to === sentence.id && !sentence.links.some((m) => m.to === s.id)).map((l) => ({ from: s, link: l })),
  );
  const linkRow = (other: InterpretSentence, kind: string, note: string, back: boolean) => {
    const place = placeOf(view, other.id);
    return (
      <li key={`${back ? '<' : '>'}${other.id}${kind}`}>
        <span className="interpret-kind">{back ? `← ${kind}` : `${kind} →`}</span>
        {place ? (
          <button type="button" className="link-like" onClick={() => go(place)}>
            “{other.quote}”
          </button>
        ) : (
          <span className="meta">“{other.quote}” (본문에서 찾지 못함)</span>
        )}
        {note && <small className="hint">{note}</small>}
      </li>
    );
  };
  const links = sentence.links.flatMap((l) => {
    const other = byId.get(l.to);
    return other ? [linkRow(other, l.kind, l.note, false)] : [];
  });

  return (
    <section className="interpret-card" aria-label="문장 해석">
      {scene && (
        <p className="interpret-scene">
          <strong>{scene.title}</strong> · {scene.purpose}
        </p>
      )}
      <blockquote>{sentence.quote}</blockquote>
      {sentence.roles.length > 0 && (
        <div className="interpret-roles">
          {sentence.roles.map((r) => (
            <span key={r} className="chip interpret-role">
              {r}
            </span>
          ))}
        </div>
      )}
      {sentence.meaning && (
        <p>
          <span className="interpret-label">의미</span>
          {sentence.meaning}
        </p>
      )}
      {sentence.intent && (
        <p>
          <span className="interpret-label">의도</span>
          {sentence.intent}
        </p>
      )}
      {sentence.unclear && (
        <p className="interpret-unclear">
          <span className="interpret-label">의도가 흐릴 수 있음</span>
          {sentence.unclear}
        </p>
      )}
      {(links.length > 0 || incoming.length > 0) && (
        <>
          <h4>연결된 문장</h4>
          <ul className="interpret-links">
            {links}
            {incoming.map(({ from, link }) => linkRow(from, link.kind, link.note, true))}
          </ul>
        </>
      )}
    </section>
  );
}

function Overview({ view, go }: { view: InterpretView; go: Go }) {
  const { reading } = view;
  const byId = new Map(reading.sentences.map((s) => [s.id, s]));
  const pairs = reading.sentences.flatMap((s) => s.links.map((l) => ({ from: s, to: byId.get(l.to), kind: l.kind }))).filter((p) => p.to);
  const unclear = reading.sentences.filter((s) => s.unclear);
  const sentenceButton = (s: InterpretSentence) => {
    const place = placeOf(view, s.id);
    return place ? (
      <button type="button" className="link-like" onClick={() => go(place)}>
        “{s.quote}”
      </button>
    ) : (
      <span className="meta">“{s.quote}”</span>
    );
  };
  return (
    <section className="interpret-overview" aria-label="회차 해석">
      {reading.summary && <p className="interpret-summary">{reading.summary}</p>}
      {reading.scenes.length > 0 && (
        <>
          <h4>장면</h4>
          <ol className="interpret-scenes">
            {reading.scenes.map((s) => (
              <li key={`${s.block}${s.title}`}>
                <strong>{s.title}</strong>
                <span>{s.purpose}</span>
              </li>
            ))}
          </ol>
        </>
      )}
      {pairs.length > 0 && (
        <>
          <h4>연결 {pairs.length}</h4>
          <ul className="interpret-links">
            {pairs.map((p) => (
              <li key={`${p.from.id}>${p.to!.id}${p.kind}`}>
                {sentenceButton(p.from)}
                <span className="interpret-kind">{p.kind} →</span>
                {sentenceButton(p.to!)}
              </li>
            ))}
          </ul>
        </>
      )}
      <h4>의도가 흐릴 수 있는 문장 {unclear.length}</h4>
      {unclear.length ? (
        <ul className="interpret-links">
          {unclear.map((s) => (
            <li key={s.id}>
              {sentenceButton(s)}
              <small className="hint">{s.unclear}</small>
            </li>
          ))}
        </ul>
      ) : (
        <p className="empty-note">없습니다.</p>
      )}
    </section>
  );
}
