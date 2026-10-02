// Sentences about AI requests: how much will be sent, and what was used.
// Plain sentences, no prices (the writer pays the AI company directly).

import type { AiUsage } from '../api/types';
import { num } from './format';

/** Rounds a character count for "about" sentences (약 5,100자). */
function about(chars: number): string {
  if (chars < 1000) return num(chars);
  return num(Math.round(chars / 100) * 100);
}

/** "보낼 글은 약 5,100자입니다" for one or more chapters. */
export function sizeText(chars: number, count = 1, noun = '회차'): string {
  const lead = count > 1 ? `${noun} ${count}개를 하나씩 보냅니다. ` : '';
  return `${lead}보낼 글은 모두 약 ${about(chars)}자입니다.`;
}

/** What one request used, as the company counts it, when it says. */
export function usageText(usage: AiUsage | null, sentChars: number): string {
  const sent = `보낸 글 약 ${about(sentChars)}자`;
  if (!usage || (usage.inputTokens == null && usage.outputTokens == null)) {
    return `${sent}. 쓴 양은 AI 회사 사용 내역에서 볼 수 있습니다.`;
  }
  const parts: string[] = [];
  if (usage.inputTokens != null) parts.push(`읽은 양 ${num(usage.inputTokens)}토큰`);
  if (usage.outputTokens != null) parts.push(`답한 양 ${num(usage.outputTokens)}토큰`);
  return `${sent}, AI 회사 기준으로 ${parts.join(', ')}을 썼습니다. 요금은 AI 회사 계정에서 나갑니다.`;
}

/** Adds up what several requests used; unknown stays unknown. */
export function addUsage(list: (AiUsage | null)[]): AiUsage | null {
  let input: number | null = null;
  let output: number | null = null;
  for (const u of list) {
    if (!u) continue;
    if (u.inputTokens != null) input = (input ?? 0) + u.inputTokens;
    if (u.outputTokens != null) output = (output ?? 0) + u.outputTokens;
  }
  return input == null && output == null ? null : { inputTokens: input, outputTokens: output };
}
