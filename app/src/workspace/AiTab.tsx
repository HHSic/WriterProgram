// AI 점검 tab of the right column: 회차 요약 and 설정 모순 점검 with the
// writer's own API key. Every request first shows what will be sent; the
// results are only shown (and, for a summary, put into the synopsis on the
// writer's click). Nothing here changes the manuscript or is kept on disk.

import { useEffect, useState } from 'react';
import { api } from '../api';
import type { AiCheck, AiFinding, AiPreview, AiSummary, AiSwap, AiTask } from '../api/types';
import { addUsage, sizeText, usageText } from '../lib/aiText';
import { errorText, num } from '../lib/format';
import { UNTITLED, docNoun, docNumber, withObject } from '../lib/labels';
import { aiReady, findDoc, jumpTo, openDialog, putSynopsis, saveEverything, showToast, useApp } from '../store';

type Step =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'preview'; task: AiTask; previews: AiPreview[] }
  | { kind: 'running'; task: AiTask; done: number; total: number }
  | { kind: 'summaries'; items: AiSummary[]; error?: string }
  | { kind: 'check'; result: AiCheck }
  | { kind: 'error'; text: string; task: AiTask; docIds: string[] };

/** The last request from a chapter's menu that was taken up. */
let handled = 0;

const TASK_LABEL: Record<AiTask, (noun: string) => string> = {
  summary: (noun) => `${noun} 요약`,
  check: () => '설정 모순 점검',
};

export function AiTab({ docId }: { docId: string }) {
  const ov = useApp((s) => s.overview)!;
  const ai = useApp((s) => s.ai);
  const request = useApp((s) => s.aiRequest);
  const [step, setStep] = useState<Step>({ kind: 'idle' });
  const [whole, setWhole] = useState(false);
  const kind = ov.project.kind;
  const noun = docNoun(kind);
  const place = findDoc(ov, docId);
  const partDocs = place?.part?.docs.map((d) => d.id) ?? [docId];
  const company = ai?.companies.find((c) => c.provider === ai.provider);

  const label = (id: string) => {
    const found = findDoc(ov, id);
    if (!found) return UNTITLED;
    const title = found.doc.title || UNTITLED;
    return found.number ? `${docNumber(kind, found.number)} · ${title}` : title;
  };

  const prepare = async (task: AiTask, docIds: string[]) => {
    if (!(await saveEverything())) return;
    setStep({ kind: 'loading' });
    try {
      const previews = await api.aiPreview(ov.root, task, docIds);
      setStep({ kind: 'preview', task, previews });
    } catch (e) {
      setStep({ kind: 'error', text: errorText(e), task, docIds });
    }
  };

  useEffect(() => {
    // A request from a chapter's ⋯ menu: show what would be sent.
    if (!request || request.nonce === handled) return;
    handled = request.nonce;
    void prepare(request.task, request.docIds);
  }, [request?.nonce]); // eslint-disable-line react-hooks/exhaustive-deps

  const send = async (task: AiTask, previews: AiPreview[]) => {
    const docIds = previews.map((p) => p.docId);
    if (task === 'summary') {
      const items: AiSummary[] = [];
      for (let i = 0; i < previews.length; i += 1) {
        setStep({ kind: 'running', task, done: i, total: previews.length });
        try {
          items.push(await api.aiSummarize(ov.root, previews[i].docId));
        } catch (e) {
          if (!items.length) setStep({ kind: 'error', text: errorText(e), task, docIds });
          else setStep({ kind: 'summaries', items, error: errorText(e) });
          return;
        }
      }
      setStep({ kind: 'summaries', items });
      return;
    }
    setStep({ kind: 'running', task, done: 0, total: 1 });
    try {
      setStep({ kind: 'check', result: await api.aiCheck(ov.root, previews[0].docId) });
    } catch (e) {
      setStep({ kind: 'error', text: errorText(e), task, docIds });
    }
  };

  if (!aiReady(ai)) {
    return (
      <div className="ai-tab">
        <p className="empty-note">
          {ai?.enabled
            ? `${company?.label ?? 'AI 회사'} API 키가 없습니다. 키를 넣으면 ${noun} 요약과 설정 모순 점검을 쓸 수 있습니다.`
            : `AI 점검이 꺼져 있습니다. 내 API 키로 AI 회사에 연결하면 ${noun} 요약과 설정 모순 점검을 쓸 수 있습니다. AI는 원고를 고치거나 이어 쓰지 않습니다.`}
        </p>
        <button type="button" className="btn" onClick={() => openDialog({ kind: 'ai' })}>
          AI 연결 설정…
        </button>
      </div>
    );
  }

  const reset = () => setStep({ kind: 'idle' });

  return (
    <div className="ai-tab">
      {step.kind === 'idle' && (
        <>
          <section className="ai-action">
            <strong>{noun} 요약</strong>
            <p className="hint">줄거리를 2~4문장으로 요약합니다. 시놉시스에 넣거나 복사할 수 있습니다.</p>
            {partDocs.length > 1 && (
              <div className="segmented small">
                <label className={!whole ? 'on' : ''}>
                  <input type="radio" checked={!whole} onChange={() => setWhole(false)} />이 {noun}
                </label>
                <label className={whole ? 'on' : ''}>
                  <input type="radio" checked={whole} onChange={() => setWhole(true)} />이 부 전체 ({partDocs.length}개)
                </label>
              </div>
            )}
            <button type="button" className="btn small" onClick={() => void prepare('summary', whole ? partDocs : [docId])}>
              보낼 내용 보기
            </button>
          </section>
          <section className="ai-action">
            <strong>설정 모순 점검</strong>
            <p className="hint">
              이 {withObject(noun)} 여기 나오는 설정 카드와 견주어 나이, 외모, 호칭, 능력, 관계가 어긋난 곳을 찾습니다.
            </p>
            <button type="button" className="btn small" onClick={() => void prepare('check', [docId])}>
              보낼 내용 보기
            </button>
          </section>
          <p className="hint">
            {company?.label} · 보내기 전에 보낼 내용을 그대로 보여 드립니다. AI는 원고를 고치거나 이어 쓰지 않습니다.
          </p>
        </>
      )}

      {step.kind === 'loading' && <p className="empty-note">보낼 내용을 준비하는 중…</p>}

      {step.kind === 'preview' && (
        <PreviewView
          step={step}
          noun={noun}
          company={company?.label ?? 'AI 회사'}
          label={label}
          onSend={() => void send(step.task, step.previews)}
          onCancel={reset}
        />
      )}

      {step.kind === 'running' && (
        <p className="empty-note ai-busy">
          AI가 읽는 중…{step.total > 1 ? ` (${step.done + 1}/${step.total})` : ''}
        </p>
      )}

      {step.kind === 'summaries' && <SummariesView items={step.items} error={step.error} label={label} onDone={reset} />}

      {step.kind === 'check' && <CheckView result={step.result} label={label} onDone={reset} />}

      {step.kind === 'error' && (
        <div className="ai-result">
          <p className="warn-text">{step.text}</p>
          <div className="row wrap">
            <button type="button" className="btn small" onClick={() => void prepare(step.task, step.docIds)}>
              다시 해 보기
            </button>
            <button type="button" className="btn small" onClick={() => openDialog({ kind: 'ai' })}>
              AI 연결 설정…
            </button>
            <button type="button" className="btn small" onClick={reset}>
              처음으로
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

/** Which names go out as which stand-ins (each chapter has its own). */
function Swaps({ swaps }: { swaps: AiSwap[] }) {
  if (!swaps.length) return null;
  return (
    <p className="ai-swaps">
      가려서 보낼 이름:{' '}
      {swaps.map((s, i) => (
        <span key={s.name}>
          {i > 0 && ' · '}
          {s.name} → <code>{s.standIn}</code>
        </span>
      ))}
    </p>
  );
}

function PreviewView({
  step,
  noun,
  company,
  label,
  onSend,
  onCancel,
}: {
  step: { task: AiTask; previews: AiPreview[] };
  noun: string;
  company: string;
  label: (id: string) => string;
  onSend: () => void;
  onCancel: () => void;
}) {
  const { task, previews } = step;
  const chars = previews.reduce((n, p) => n + p.chars, 0);
  const single = previews.length === 1;
  const cards = [...new Set(previews.flatMap((p) => p.cards))];
  return (
    <div className="ai-preview">
      <h3>{TASK_LABEL[task](noun)} · 보낼 내용</h3>
      <p>
        {sizeText(chars, previews.length, noun)} {company}로 갑니다.
      </p>
      {cards.length > 0 && <p>함께 보낼 설정 카드: {cards.join(', ')}</p>}
      {single && <Swaps swaps={previews[0].swaps} />}
      {previews.map((p) => (
        <details key={p.docId} open={single}>
          <summary>
            {label(p.docId)} · {num(p.chars)}자
          </summary>
          {!single && <Swaps swaps={p.swaps} />}
          <pre className="ai-sent">{p.text}</pre>
        </details>
      ))}
      <details>
        <summary>AI에게 주는 지시</summary>
        <pre className="ai-sent">{previews[0]?.instructions}</pre>
      </details>
      <div className="row wrap">
        <button type="button" className="btn primary small" onClick={onSend}>
          보내기
        </button>
        <button type="button" className="btn small" onClick={onCancel}>
          취소
        </button>
      </div>
    </div>
  );
}

function SummariesView({
  items,
  error,
  label,
  onDone,
}: {
  items: AiSummary[];
  error?: string;
  label: (id: string) => string;
  onDone: () => void;
}) {
  const ov = useApp((s) => s.overview)!;
  const [put, setPut] = useState<Record<string, boolean>>({});

  const intoSynopsis = (item: AiSummary) => {
    const current = findDoc(ov, item.docId)?.doc.synopsis ?? '';
    const run = async () => {
      if (await putSynopsis(item.docId, item.text)) setPut((p) => ({ ...p, [item.docId]: true }));
    };
    if (current.trim() && current.trim() !== item.text) {
      openDialog({
        kind: 'confirm',
        title: '시놉시스 바꾸기',
        message: `지금 시놉시스 “${current.length > 60 ? `${current.slice(0, 60)}…` : current}”를 이 요약으로 바꿉니다.`,
        confirm: '바꾸기',
        onConfirm: run,
      });
    } else void run();
  };

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast({ text: '요약을 복사함' });
    } catch {
      showToast({ text: '복사하지 못함', tone: 'error' });
    }
  };

  return (
    <div className="ai-result">
      {items.map((item) => (
        <section key={item.docId} className="ai-summary">
          <h3>{label(item.docId)}</h3>
          <p>{item.text}</p>
          <div className="row wrap">
            <button type="button" className="btn small" disabled={put[item.docId]} onClick={() => intoSynopsis(item)}>
              {put[item.docId] ? '시놉시스에 넣음' : '시놉시스에 넣기'}
            </button>
            <button type="button" className="btn small" onClick={() => void copy(item.text)}>
              복사
            </button>
          </div>
        </section>
      ))}
      {error && <p className="warn-text">나머지는 요약하지 못함 · {error}</p>}
      <p className="hint">
        AI가 쓴 요약이라 틀릴 수 있습니다. 원고는 바뀌지 않았습니다.{' '}
        {usageText(
          addUsage(items.map((i) => i.usage)),
          items.reduce((n, i) => n + i.sentChars, 0),
        )}
      </p>
      <button type="button" className="btn small" onClick={onDone}>
        처음으로
      </button>
    </div>
  );
}

function CheckView({ result, label, onDone }: { result: AiCheck; label: (id: string) => string; onDone: () => void }) {
  const go = (f: AiFinding) => {
    if (f.place) void jumpTo({ docId: result.docId, block: f.place.block, start: f.place.start, end: f.place.end });
  };
  return (
    <div className="ai-result">
      <h3>{label(result.docId)}</h3>
      <p className="find-summary">
        {result.findings.length
          ? `설정 카드와 어긋날 수 있는 곳 ${result.findings.length}곳`
          : '설정 카드와 어긋난 곳을 찾지 못했습니다.'}
      </p>
      <ul className="ai-findings">
        {result.findings.map((f, i) => (
          <li key={i}>
            <button type="button" className="find-hit ai-finding" disabled={!f.place} onClick={() => go(f)}>
              <span className="ai-quote">
                {f.place ? (
                  <>
                    {f.place.before}
                    <mark>{f.place.text}</mark>
                    {f.place.after}
                  </>
                ) : (
                  <mark>{f.quote}</mark>
                )}
              </span>
              <span className="ai-problem">
                <strong>{f.card}</strong> · {f.problem}
              </span>
              {!f.place && <span className="hint">원고에서 이 구절을 찾지 못함</span>}
            </button>
          </li>
        ))}
      </ul>
      {result.cards.length > 0 && <p className="hint">견준 설정 카드: {result.cards.join(', ')}</p>}
      <p className="hint">
        AI의 점검은 틀릴 수 있으니 참고만 하세요. 원고는 바뀌지 않았습니다. {usageText(result.usage, result.sentChars)}
      </p>
      <button type="button" className="btn small" onClick={onDone}>
        처음으로
      </button>
    </div>
  );
}
