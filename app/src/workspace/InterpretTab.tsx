// 해석 tab (해석 모드): what the sentence under the cursor does and means, and
// the sentences it links to, here and in other chapters; with nothing picked,
// the chapter's reading as a whole. The reading is written by an assistant
// through writer-mcp (docs/reading.md); the writer can change it, take parts
// out, read a sentence themselves and link sentences, also across chapters.

import { useEffect, useState } from 'react';
import type { InterpretIncoming, InterpretPlace, InterpretRemote, InterpretSentence, InterpretSpot, InterpretView } from '../api/types';
import { placeAt } from '../editor/interpret';
import { timeLabel } from '../lib/format';
import {
  cancelLinking,
  emptyReading,
  finishLinking,
  jumpTo,
  loadInterpretation,
  newSentence,
  pickSentence,
  removeIncoming,
  removeLink,
  removeSentence,
  saveSentence,
  setInterpretMode,
  setLinking,
  startLinking,
  useApp,
} from '../store';

/** What a sentence can do, as the kfiction guide names them (references/reading.md). */
export const ROLES = ['복선', '회수', '인물', '관계', '긴장', '정보', '분위기', '이미지', '리듬', '전환', '주제', '시점'];
const KINDS = ['회수', '복선', '같은 이미지', '대비', '변화', '되풀이'];

function emptyView(docId: string): InterpretView {
  return { reading: emptyReading(docId), read: false, places: [], gone: [], changed: false, remote: [], incoming: [] };
}

export function InterpretTab({ docId }: { docId: string }) {
  const mode = useApp((s) => s.interpretMode);
  const version = useApp((s) => s.interpretVersion);
  const loaded = useApp((s) => (s.interpret?.docId === docId ? s.interpret : null));
  const pick = useApp((s) => (s.interpretPick?.docId === docId ? s.interpretPick : null));
  const linking = useApp((s) => s.interpretLinking);
  const [draft, setDraft] = useState<InterpretSentence | null>(null);

  // With the mode on, the document pane loads it (it needs it for the marks).
  useEffect(() => {
    if (!mode) void loadInterpretation(docId);
  }, [mode, docId, version]);
  // A form left open belongs to the sentence it was opened on.
  useEffect(() => setDraft(null), [docId, pick?.block, pick?.offset]);

  const modeBar = (
    <div className="interpret-mode">
      <button type="button" className={`btn${mode ? ' dark' : ''}`} aria-pressed={mode} onClick={() => setInterpretMode(!mode)}>
        해석 모드 {mode ? '끄기' : '켜기'}
      </button>
      <small className="hint">{mode ? '문장을 누르면 그 문장의 해석이 여기 나옵니다.' : '켜고 문장을 누르면 그 문장의 해석과 연결된 문장이 나옵니다.'}</small>
    </div>
  );

  if (!loaded) return <div className="interpret">{modeBar}<p className="empty-note">해석을 읽는 중…</p></div>;
  const view = loaded.view ?? emptyView(docId);

  const current = pick ? placeAt(view.places, pick.block, pick.offset) : null;
  const sentence = current ? view.reading.sentences.find((s) => s.id === current.id) ?? null : null;
  const incomingHere = pick
    ? view.incoming.filter((l) => l.place && l.place.block === pick.block && l.place.start <= pick.offset && pick.offset <= l.place.end)
    : [];
  const go: Go = (spot) => {
    if (spot.doc === docId) pickSentence(docId, spot.block, spot.start);
    void jumpTo({ docId: spot.doc, block: spot.block, start: spot.start, end: spot.end });
  };
  const goHere = (place: InterpretPlace) => go({ ...place, doc: docId, title: '' });

  let body;
  if (draft) {
    body = (
      <SentenceForm
        draft={draft}
        isNew={!view.reading.sentences.some((s) => s.id === draft.id)}
        onCancel={() => setDraft(null)}
        onSave={async (s) => {
          if (await saveSentence(docId, s)) setDraft(null);
        }}
      />
    );
  } else if (sentence && current) {
    body = <SentenceCard docId={docId} view={view} sentence={sentence} go={go} goHere={goHere} edit={() => setDraft(structuredClone(sentence))} />;
  } else if (pick && mode) {
    const target = pick.sentence ?? null;
    body = (
      <>
        {incomingHere.length > 0 ? (
          <section className="interpret-card" aria-label="문장 해석">
            {target && <blockquote>{target.text}</blockquote>}
            <p className="hint">이 문장에는 이 회차의 해석이 없지만 다른 회차에서 이어져 있습니다.</p>
            <h4>연결된 문장</h4>
            <ul className="interpret-links">
              {incomingHere.map((l) => (
                <IncomingRow key={`${l.doc}${l.from}${l.kind}`} docId={docId} link={l} go={go} />
              ))}
            </ul>
          </section>
        ) : (
          <>
            {target && <blockquote className="interpret-unread">{target.text}</blockquote>}
            <p className="empty-note">이 문장에는 해석이 없습니다. 밑줄 친 문장을 누르거나 직접 해석을 남기세요.</p>
          </>
        )}
        {target && !linking && (
          <button type="button" className="btn small" onClick={() => setDraft(newSentence(docId, target))}>
            이 문장 해석 추가
          </button>
        )}
        <Overview view={view} go={go} goHere={goHere} />
      </>
    );
  } else {
    body = <Overview view={view} go={go} goHere={goHere} />;
  }

  return (
    <div className="interpret">
      {modeBar}
      {linking && <LinkingBar docId={docId} target={mode ? (pick?.sentence ?? null) : null} />}
      {view.changed && (
        <p className="interpret-stale" role="status">
          해석한 뒤 본문이 바뀌었습니다.{view.gone.length ? ` 찾지 못한 문장 ${view.gone.length}개는 빠졌습니다.` : ' 문장은 모두 찾았습니다.'}
        </p>
      )}
      {!view.read && !draft && !linking && (
        <p className="empty-note">
          {view.incoming.length
            ? '이 회차는 아직 해석하지 않았습니다. 다른 회차에서 이어진 문장만 보입니다.'
            : '이 회차는 아직 해석하지 않았습니다. AI 도우미(작품 설정 → AI 도우미 연결)를 연결한 Claude Code에서 “이 회차 해석해 줘”라고 하면 만들어져 여기에 나옵니다. 해석 모드에서 문장을 눌러 직접 남길 수도 있습니다.'}
        </p>
      )}
      {body}
      <Byline view={view} />
    </div>
  );
}

type Go = (spot: InterpretSpot) => void;
type GoHere = (place: InterpretPlace) => void;

function Byline({ view }: { view: InterpretView }) {
  const { made, by, edited } = view.reading;
  if (!view.read) return null;
  const parts = [made ? `해석: ${by || '도우미'} · ${timeLabel(made)}` : '해석: 작가'];
  if (edited) parts.push(`작가가 고침 · ${timeLabel(edited)}`);
  return <p className="meta interpret-by">{parts.join(' · ')}</p>;
}

/** 연결 추가 under way: what it links as, and the sentence picked to link to. */
function LinkingBar({ docId, target }: { docId: string; target: { text: string; block: number; start: number; end: number } | null }) {
  const linking = useApp((s) => s.interpretLinking)!;
  const [busy, setBusy] = useState(false);
  return (
    <section className="interpret-linking" aria-label="연결 추가">
      <p>
        <span className="interpret-label">연결 추가</span>“{linking.fromQuote}”에서
      </p>
      <div className="interpret-linking-fields">
        <select aria-label="연결의 종류" value={linking.kind} onChange={(e) => setLinking({ kind: e.target.value })}>
          {[...new Set([...KINDS, linking.kind])].map((k) => (
            <option key={k}>{k}</option>
          ))}
        </select>
        <input aria-label="연결 설명" placeholder="설명(선택)" value={linking.note} onChange={(e) => setLinking({ note: e.target.value })} />
      </div>
      {target ? (
        <p>
          <span className="interpret-label">이을 문장</span>“{target.text}”
        </p>
      ) : (
        <small className="hint">이을 문장을 본문에서 누르세요. 다른 회차로 옮겨 가서 눌러도 됩니다.</small>
      )}
      <div className="interpret-actions">
        <button
          type="button"
          className="btn small primary"
          disabled={!target || busy}
          onClick={async () => {
            if (!target) return;
            setBusy(true);
            await finishLinking(docId, target);
            setBusy(false);
          }}
        >
          이 문장과 잇기
        </button>
        <button type="button" className="btn small ghost" onClick={cancelLinking}>
          그만두기
        </button>
      </div>
    </section>
  );
}

function SentenceForm({
  draft,
  isNew,
  onSave,
  onCancel,
}: {
  draft: InterpretSentence;
  isNew: boolean;
  onSave: (s: InterpretSentence) => Promise<void>;
  onCancel: () => void;
}) {
  const [s, setS] = useState(draft);
  const [busy, setBusy] = useState(false);
  const toggle = (role: string) =>
    setS({ ...s, roles: s.roles.includes(role) ? s.roles.filter((r) => r !== role) : [...s.roles, role] });
  const roles = [...new Set([...ROLES, ...s.roles])];
  return (
    <form
      className="interpret-card interpret-form"
      aria-label={isNew ? '해석 추가' : '해석 고치기'}
      onSubmit={async (e) => {
        e.preventDefault();
        setBusy(true);
        const unclear = s.unclear?.trim();
        await onSave({ ...s, meaning: s.meaning.trim(), intent: s.intent.trim(), unclear: unclear || undefined });
        setBusy(false);
      }}
    >
      <blockquote>{s.quote}</blockquote>
      <span className="field-label">하는 일</span>
      <div className="interpret-roles">
        {roles.map((r) => (
          <button key={r} type="button" className={`chip interpret-role-pick${s.roles.includes(r) ? ' on' : ''}`} aria-pressed={s.roles.includes(r)} onClick={() => toggle(r)}>
            {r}
          </button>
        ))}
      </div>
      <label className="field">
        <span className="field-label">의미</span>
        <textarea rows={2} value={s.meaning} placeholder="말 너머에서 읽히는 것" onChange={(e) => setS({ ...s, meaning: e.target.value })} />
      </label>
      <label className="field">
        <span className="field-label">의도</span>
        <textarea rows={2} value={s.intent} placeholder="왜 그 자리에 있는가" onChange={(e) => setS({ ...s, intent: e.target.value })} />
      </label>
      <label className="field">
        <span className="field-label">의도가 흐릴 수 있음</span>
        <textarea rows={2} value={s.unclear ?? ''} placeholder="비워 두면 흐리지 않은 문장" onChange={(e) => setS({ ...s, unclear: e.target.value })} />
      </label>
      <div className="interpret-actions">
        <button type="submit" className="btn small primary" disabled={busy}>
          {isNew ? '해석 남기기' : '저장'}
        </button>
        <button type="button" className="btn small ghost" onClick={onCancel}>
          취소
        </button>
      </div>
    </form>
  );
}

function SentenceCard({
  docId,
  view,
  sentence,
  go,
  goHere,
  edit,
}: {
  docId: string;
  view: InterpretView;
  sentence: InterpretSentence;
  go: Go;
  goHere: GoHere;
  edit: () => void;
}) {
  const linking = useApp((s) => s.interpretLinking);
  const [confirming, setConfirming] = useState(false);
  useEffect(() => setConfirming(false), [sentence.id]);
  const scene = [...view.reading.scenes].reverse().find((s) => s.block <= sentence.block);
  const byId = new Map(view.reading.sentences.map((s) => [s.id, s]));
  const placeOf = (id: string) => view.places.find((p) => p.id === id) ?? null;
  const backHere = view.reading.sentences.flatMap((s) =>
    s.links.filter((l) => !l.doc && l.to === sentence.id && !sentence.links.some((m) => !m.doc && m.to === s.id)).map((l) => ({ from: s, link: l })),
  );
  const remote = view.remote.filter((r) => r.from === sentence.id);
  const here = placeOf(sentence.id);
  const incoming = here
    ? view.incoming.filter((l) => l.place && l.place.block === here.block && l.place.start < here.end && here.start < l.place.end)
    : [];

  const row = (key: string, other: InterpretSentence, kind: string, note: string, back: boolean, remove?: () => void) => {
    const place = placeOf(other.id);
    return (
      <li key={key}>
        <span className="interpret-kind">{back ? `← ${kind}` : `${kind} →`}</span>
        {place ? (
          <button type="button" className="link-like" onClick={() => goHere(place)}>
            “{other.quote}”
          </button>
        ) : (
          <span className="meta">“{other.quote}” (본문에서 찾지 못함)</span>
        )}
        {note && <small className="hint">{note}</small>}
        {remove && <RemoveLink onClick={remove} />}
      </li>
    );
  };
  const links = sentence.links.flatMap((l, i) => {
    if (l.doc) return [];
    const other = byId.get(l.to);
    return other ? [row(`>${i}`, other, l.kind, l.note, false, () => void removeLink(docId, sentence.id, i))] : [];
  });
  const count = links.length + backHere.length + remote.length + incoming.length;

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
      {count > 0 && (
        <>
          <h4>연결된 문장</h4>
          <ul className="interpret-links">
            {links}
            {backHere.map(({ from, link }) => row(`<${from.id}${link.kind}`, from, link.kind, link.note, true))}
            {remote.map((r) => (
              <RemoteRow key={`r${r.link}`} remote={r} go={go} remove={() => void removeLink(docId, sentence.id, r.link)} />
            ))}
            {incoming.map((l) => (
              <IncomingRow key={`i${l.doc}${l.from}${l.kind}`} docId={docId} link={l} go={go} />
            ))}
          </ul>
        </>
      )}
      <div className="interpret-actions">
        <button type="button" className="btn small" onClick={edit}>
          고치기
        </button>
        <button type="button" className="btn small" disabled={!!linking} onClick={() => startLinking(docId, sentence)}>
          연결 추가
        </button>
        {confirming ? (
          <>
            <button type="button" className="btn small danger" onClick={() => void removeSentence(docId, sentence.id)}>
              해석 지우기
            </button>
            <button type="button" className="btn small ghost" onClick={() => setConfirming(false)}>
              그대로 두기
            </button>
          </>
        ) : (
          <button type="button" className="btn small ghost" onClick={() => setConfirming(true)}>
            지우기
          </button>
        )}
      </div>
    </section>
  );
}

function RemoveLink({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" className="interpret-unlink" aria-label="연결 지우기" title="연결 지우기" onClick={onClick}>
      ×
    </button>
  );
}

/** A link from this chapter into another. */
function RemoteRow({ remote, go, remove }: { remote: InterpretRemote; go: Go; remove?: () => void }) {
  return (
    <li>
      <span className="interpret-kind">{remote.kind} →</span>
      <span className="interpret-chapter">「{remote.title || '지워진 회차'}」</span>
      {remote.spot ? (
        <button type="button" className="link-like" onClick={() => go(remote.spot!)}>
          “{remote.quote}”
        </button>
      ) : (
        <span className="meta">“{remote.quote}” (그 회차에서 찾지 못함)</span>
      )}
      {remote.note && <small className="hint">{remote.note}</small>}
      {remove && <RemoveLink onClick={remove} />}
    </li>
  );
}

/** A link another chapter's reading makes into this one. */
function IncomingRow({ docId, link, go, showHere }: { docId: string; link: InterpretIncoming; go: Go; showHere?: boolean }) {
  return (
    <li>
      <span className="interpret-kind">← {link.kind}</span>
      <span className="interpret-chapter">「{link.title || '지워진 회차'}」</span>
      {link.fromSpot ? (
        <button type="button" className="link-like" onClick={() => go(link.fromSpot!)}>
          “{link.fromQuote}”
        </button>
      ) : (
        <span className="meta">“{link.fromQuote}” (그 회차에서 찾지 못함)</span>
      )}
      {showHere && (
        <>
          <span className="interpret-kind">→</span>
          {link.place ? (
            <button type="button" className="link-like" onClick={() => go({ ...link.place!, doc: docId, title: '' })}>
              “{link.quote}”
            </button>
          ) : (
            <span className="meta">“{link.quote}” (본문에서 찾지 못함)</span>
          )}
        </>
      )}
      {link.note && <small className="hint">{link.note}</small>}
      <RemoveLink onClick={() => void removeIncoming(docId, link)} />
    </li>
  );
}

function Overview({ view, go, goHere }: { view: InterpretView; go: Go; goHere: GoHere }) {
  const { reading } = view;
  const byId = new Map(reading.sentences.map((s) => [s.id, s]));
  const pairs = reading.sentences
    .flatMap((s) => s.links.filter((l) => !l.doc).map((l) => ({ from: s, to: byId.get(l.to), kind: l.kind })))
    .filter((p) => p.to);
  const unclear = reading.sentences.filter((s) => s.unclear);
  const sentenceButton = (s: InterpretSentence) => {
    const place = view.places.find((p) => p.id === s.id);
    return place ? (
      <button type="button" className="link-like" onClick={() => goHere(place)}>
        “{s.quote}”
      </button>
    ) : (
      <span className="meta">“{s.quote}”</span>
    );
  };
  const across = view.remote.length + view.incoming.length;
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
      {across > 0 && (
        <>
          <h4>다른 회차와의 연결 {across}</h4>
          <ul className="interpret-links">
            {view.remote.map((r) => {
              const from = byId.get(r.from);
              return (
                <li key={`r${r.from}${r.link}`}>
                  {from && sentenceButton(from)}
                  <ul className="interpret-links nested">
                    <RemoteRow remote={r} go={go} />
                  </ul>
                </li>
              );
            })}
            {view.incoming.map((l) => (
              <IncomingRow key={`i${l.doc}${l.from}${l.kind}${l.quote}`} docId={reading.doc} link={l} go={go} showHere />
            ))}
          </ul>
        </>
      )}
      {view.read && (
        <>
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
        </>
      )}
    </section>
  );
}
