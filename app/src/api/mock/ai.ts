// AI with the writer's own key, played in memory: settings and "keys" here,
// believable Korean answers after a short wait. Nothing leaves the browser.

import type {
  AiCheck,
  AiCompany,
  AiFinding,
  AiModels,
  AiPreview,
  AiProvider,
  AiSettings,
  AiSummary,
  AiSwap,
  AiTask,
  Backend,
  Card,
  SearchMatch,
} from '../types';
import { doc, now, project } from './state';

const LABELS: Record<AiProvider, string> = {
  anthropic: 'Anthropic (Claude)',
  openai: 'OpenAI',
  gemini: 'Google Gemini',
};

const DEFAULTS: Record<AiProvider, AiModels> = {
  anthropic: { summary: 'claude-haiku-4-5', check: 'claude-sonnet-5-5' },
  openai: { summary: 'gpt-5-nano', check: 'gpt-5-mini' },
  gemini: { summary: 'gemini-2.5-flash-lite', check: 'gemini-2.5-flash' },
};

const PROVIDERS: AiProvider[] = ['anthropic', 'openai', 'gemini'];

const state = {
  enabled: false,
  provider: 'anthropic' as AiProvider,
  agreed: null as string | null,
  models: { ...DEFAULTS } as Record<AiProvider, AiModels>,
  keys: new Map<AiProvider, string>(),
};

function settings(): AiSettings {
  const companies: AiCompany[] = PROVIDERS.map((p) => ({
    provider: p,
    label: LABELS[p],
    models: { ...state.models[p] },
    defaults: { ...DEFAULTS[p] },
    hasKey: state.keys.has(p),
  }));
  return { enabled: state.enabled, provider: state.provider, agreed: state.agreed, companies };
}

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

const SUMMARY_SYSTEM =
  '당신은 소설가의 원고 정리를 돕는 도우미입니다. 원고를 고쳐 쓰거나 이어 쓰거나 새 문장을 지어내지 않습니다.\n주어진 회차 본문의 줄거리를 2~4문장으로 요약하세요. …';
const CHECK_SYSTEM =
  '당신은 소설가의 설정 점검을 돕는 도우미입니다. 원고를 고쳐 쓰거나 이어 쓰거나 고친 문장을 제안하지 않습니다.\n[설정 카드]와 [회차 본문]을 견주어, 본문이 카드에 적힌 설정과 어긋나는 곳만 찾으세요. …';

function ready() {
  if (!state.enabled) throw "AI 연결이 꺼져 있음. 'AI 연결 (내 API 키)'에서 켜 주세요.";
  if (!state.keys.has(state.provider)) throw "API 키가 없음. 'AI 연결 (내 API 키)'에서 키를 넣어 주세요.";
}

/** Paragraph texts of a chapter (scene breaks as "* * *"). */
function paragraphs(root: string, docId: string): string[] {
  const d = doc(project(root), docId);
  return (d.body.content ?? []).map((node) =>
    node.type === 'sceneBreak' ? '* * *' : (node.content ?? []).map((i) => (i.type === 'hardBreak' ? '\n' : (i.text ?? ''))).join(''),
  );
}

function names(c: Card): string[] {
  return [c.name, ...c.aliases].map((n) => n.trim()).filter((n) => n.length >= 2);
}

const KIND: Record<string, string> = { person: '인물', place: '장소', term: '용어' };

function letters(n: number): string {
  let s = '';
  let i = n;
  for (;;) {
    s = String.fromCharCode(65 + (i % 26)) + s;
    if (i < 26) return s;
    i = Math.floor(i / 26) - 1;
  }
}

/** Stand-ins like the Rust side: 인물A for the name, 인물A2 … for other names. */
function maskerFor(cards: Card[], text: string) {
  const met = cards
    .map((c) => ({ c, at: Math.min(...names(c).map((n) => (text.includes(n) ? text.indexOf(n) : Infinity))) }))
    .filter((x) => x.at !== Infinity)
    .sort((a, b) => a.at - b.at)
    .map((x) => x.c);
  const swaps: AiSwap[] = [];
  met.forEach((c, i) => {
    const head = `${KIND[c.cardType] ?? '설정'}${letters(i)}`;
    names(c).forEach((n, j) => swaps.push({ name: n, standIn: j === 0 ? head : `${head}${j + 1}`, card: c.name }));
  });
  const ordered = [...swaps].sort((a, b) => b.name.length - a.name.length);
  const mask = (s: string) => {
    if (!ordered.length) return s;
    const re = new RegExp(ordered.map((x) => x.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'g');
    return s.replace(re, (m) => ordered.find((x) => x.name === m)?.standIn ?? m);
  };
  return { met, swaps, mask };
}

function cardText(c: Card): string {
  const lines = [`### ${c.name} (${KIND[c.cardType] ?? '설정'})`];
  if (c.aliases.length) lines.push(`다른 이름: ${c.aliases.join(', ')}`);
  for (const [k, v] of c.fields) if (v.trim()) lines.push(`${k}: ${v}`);
  if (c.description.trim()) lines.push(`설명: ${c.description.trim()}`);
  return lines.join('\n');
}

function preview(root: string, task: AiTask, docId: string): AiPreview {
  const p = project(root);
  const d = doc(p, docId);
  const text = paragraphs(root, docId)
    .filter((x) => x.trim())
    .join('\n');
  if (!text.trim()) throw '이 회차는 비어 있어 보낼 글이 없음';
  const cards = [...p.cards.values()];
  const { met } = maskerFor(cards, text);
  // Cards named only in the sent cards' own text are masked too.
  const cardsText = met.map(cardText).join('\n\n');
  const { swaps, mask } = maskerFor(cards, task === 'summary' ? text : `${text}\n${cardsText}`);
  let body: string;
  if (task === 'summary') {
    body = `[회차 본문]\n${mask(text)}`;
  } else {
    if (!met.length) throw '이 회차에 나오는 설정 카드가 없어 견줄 것이 없음. 설정집에 인물·장소를 먼저 적어 주세요.';
    body = `[설정 카드]\n${mask(cardsText)}\n\n[회차 본문]\n${mask(text)}`;
  }
  const instructions = task === 'summary' ? SUMMARY_SYSTEM : CHECK_SYSTEM;
  return {
    task,
    docId,
    title: d.meta.title,
    instructions,
    text: body,
    swaps,
    cards: task === 'check' ? met.map((c) => c.name) : [],
    chars: instructions.length + body.length,
  };
}

/** Where a quote is in the chapter, like the Rust side's `locate`. */
function place(root: string, docId: string, quote: string): SearchMatch | null {
  const paras = paragraphs(root, docId);
  for (let block = 0; block < paras.length; block += 1) {
    const at = paras[block].indexOf(quote);
    if (at >= 0) {
      return {
        block,
        start: at,
        end: at + quote.length,
        before: paras[block].slice(Math.max(0, at - 24), at),
        text: quote,
        after: paras[block].slice(at + quote.length, at + quote.length + 24),
      };
    }
  }
  return null;
}

const SUMMARIES: Record<string, string> = {
  '문 닫는 시간':
    '윤서하는 할머니 때부터 이어 온 대로 매일 밤 열한 시에 달빛 서점의 문을 닫는다. 이날도 손님은 없었고, 서하의 혼잣말만 오래된 서가 사이에 남는다.',
  '빗소리가 들리는 밤': '저녁부터 비가 내리는 밤, 서하는 장부를 덮고 빗방울 너머로 번지는 골목의 가로등을 오래 바라본다.',
  '비에 젖은 손님':
    '셔터를 내리던 서하에게 삼 년 만에 늦은 손님이 찾아온다. 비에 젖은 남자는 품에 안은 종이봉투만 말려 두었고, 서하는 영업이 끝났는데도 그를 들인다. 남자가 봉투에서 꺼낸 것은 같은 이름이 빼곡히 적힌 오래된 대여 카드다.',
};

export const aiMethods = {
  async aiSettings() {
    return settings();
  },
  async aiSettingsSet(patch) {
    if (patch.provider) state.provider = patch.provider;
    if (patch.models) {
      const [p, m] = patch.models;
      if (/[^A-Za-z0-9._:-]/.test(m.summary + m.check)) throw "모델 이름에는 영문, 숫자, '-', '.', '_'만 쓸 수 있음";
      state.models[p] = { summary: m.summary.trim() || DEFAULTS[p].summary, check: m.check.trim() || DEFAULTS[p].check };
    }
    if (patch.enabled !== undefined) {
      if (patch.enabled && !state.enabled) state.agreed = now();
      state.enabled = patch.enabled;
    }
    return settings();
  },
  async aiKeySet(provider, key) {
    const k = key.trim();
    if (!k) throw 'API 키를 붙여 넣어 주세요';
    if (/\s/.test(k)) throw 'API 키 모양이 아님. AI 회사 누리집에서 키를 다시 복사해 주세요.';
    state.keys.set(provider, k);
    return settings();
  },
  async aiKeyRemove(provider) {
    state.keys.delete(provider);
    return settings();
  },
  async aiConnectionCheck() {
    await pause(500);
    const key = state.keys.get(state.provider);
    if (!key) throw "API 키가 없음. 'AI 연결 (내 API 키)'에서 키를 넣어 주세요.";
    if (key.includes('wrong')) throw 'API 키가 맞지 않음. 키를 다시 복사해 넣었는지, 고른 AI 회사의 키인지 확인해 주세요.';
    const m = state.models[state.provider];
    return `${LABELS[state.provider]}에 연결됨. 요약에 ${m.summary}, 점검에 ${m.check}을(를) 씁니다.`;
  },
  async aiPreview(root, task, docIds) {
    ready();
    return docIds.map((id) => preview(root, task, id));
  },
  async aiSummarize(root, docId): Promise<AiSummary> {
    ready();
    const sent = preview(root, 'summary', docId);
    await pause(900);
    const d = doc(project(root), docId);
    const first = paragraphs(root, docId).find((x) => x.trim() && x !== '* * *') ?? '';
    const text = SUMMARIES[d.meta.title] ?? `${first.slice(0, 60)}${first.length > 60 ? '…' : ''} 이 회차는 이 장면을 중심으로 흘러간다.`;
    return {
      docId,
      text,
      usage: { inputTokens: Math.round(sent.chars * 1.2), outputTokens: Math.round(text.length * 1.1) },
      sentChars: sent.chars,
    };
  },
  async aiCheck(root, docId): Promise<AiCheck> {
    ready();
    const sent = preview(root, 'check', docId);
    await pause(1400);
    const findings: AiFinding[] = [];
    const quote = '“오늘도 손님은 없었네.”';
    const at = place(root, docId, quote);
    if (at && sent.cards.includes('윤서하')) {
      findings.push({
        quote,
        card: '윤서하',
        problem: '윤서하는 말투가 존댓말로 적혀 있는데 여기서는 반말로 말함. 혼잣말이라면 괜찮을 수 있음.',
        place: at,
      });
    }
    return {
      docId,
      findings,
      cards: sent.cards,
      usage: { inputTokens: Math.round(sent.chars * 1.2), outputTokens: 40 + findings.length * 60 },
      sentChars: sent.chars,
    };
  },
} satisfies Partial<Backend>;
